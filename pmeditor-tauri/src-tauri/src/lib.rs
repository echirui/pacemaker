use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::command;

use pacemaker_core::db::PacemakerDb;
use pacemaker_core::firmware::{detect_pacemaker_device, PfwInfo};
use pacemaker_core::types::{CaseItem, TrackId, TrackMetadata, TimeMs};
use pacemaker_core::waveform::TrackAnalysedData;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceStatusResponse {
    pub connected: bool,
    pub volume_path: Option<String>,
    pub fw_version: String,
    pub sw_version: String,
    pub serial_number: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

fn resolve_pacemaker_volume() -> Option<PathBuf> {
    // 1. Detect actual physical hardware device across mounted external volumes
    if let Some(dev) = detect_pacemaker_device() {
        return Some(dev);
    }

    // 2. Explicit standard mounted volume paths for Pacemaker hardware
    let standard_mounts = [
        "/Volumes/Pacemaker",
        "/Volumes/PACEMAKER",
    ];
    for cand in &standard_mounts {
        let p = PathBuf::from(cand);
        if p.exists() && (p.join(".Pacemaker/music.db").exists() || p.join(".Pacemaker/system.xml").exists() || p.join("music.db").exists()) {
            return Some(p);
        }
    }

    // 3. ONLY allow developer mock testing if explicitly opted in via environment variable
    if std::env::var("PACEMAKER_MOCK").unwrap_or_default() == "1" {
        for cand in &["/tmp/Pacemaker", ".", "..", "../.."] {
            let p = PathBuf::from(cand);
            if p.join(".Pacemaker/music.db").exists() || p.join("music.db").exists() {
                return Some(p);
            }
        }
    }

    None
}

fn resolve_pacemaker_db_path() -> Option<PathBuf> {
    let vol = resolve_pacemaker_volume()?;
    let p = vol.join(".Pacemaker/music.db");
    if p.exists() {
        return Some(p);
    }
    let p_root = vol.join("music.db");
    if p_root.exists() {
        return Some(p_root);
    }
    None
}

fn resolve_track_file_path(virtual_location: &str) -> Option<PathBuf> {
    if virtual_location.is_empty() {
        return None;
    }
    let clean = if let Some(stripped) = virtual_location.strip_prefix("/pmdb_tracks/") {
        stripped
    } else if let Some(stripped) = virtual_location.strip_prefix("/") {
        stripped
    } else {
        virtual_location
    };

    let vol = resolve_pacemaker_volume()?;
    let candidates = [
        vol.join(".Pacemaker/Music").join(clean),
        vol.join("Music").join(clean),
        vol.join(clean),
        PathBuf::from("/Volumes/Pacemaker/.Pacemaker/Music").join(clean),
    ];

    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

fn touch_syncfile(vol: &Path) {
    let syncfile = vol.join(".Pacemaker/syncfile");
    let _ = fs::write(syncfile, b"1");
}

#[command]
fn get_device_status() -> DeviceStatusResponse {
    let vol_opt = resolve_pacemaker_volume();
    if let Some(ref path) = vol_opt {
        let sys_xml_path = path.join(".Pacemaker/system.xml");
        let sys_xml_root = path.join("system.xml");
        let mut fw = "14142".to_string();
        let mut sw = "0514142".to_string();
        let mut serial = "PM-DEV-001".to_string();

        let xml_target = if sys_xml_path.exists() {
            Some(sys_xml_path)
        } else if sys_xml_root.exists() {
            Some(sys_xml_root)
        } else {
            None
        };

        if let Some(ref xp) = xml_target {
            if let Ok(xml) = fs::read_to_string(xp) {
                if let Some(s) = extract_tag(&xml, "FWVERSION") { fw = s; }
                if let Some(s) = extract_tag(&xml, "SWVERSION") { sw = s; }
                if let Some(s) = extract_tag(&xml, "SERIALNUMBER") { serial = s; }
            }
        }

        // Get filesystem capacity if possible
        let (total, free) = get_disk_space(path);

        return DeviceStatusResponse {
            connected: true,
            volume_path: Some(path.display().to_string()),
            fw_version: fw,
            sw_version: sw,
            serial_number: serial,
            total_bytes: total,
            free_bytes: free,
        };
    }

    DeviceStatusResponse {
        connected: false,
        volume_path: None,
        fw_version: "--".into(),
        sw_version: "--".into(),
        serial_number: "--".into(),
        total_bytes: 0,
        free_bytes: 0,
    }
}

fn get_disk_space(path: &Path) -> (u64, u64) {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;
        use std::os::unix::ffi::OsStrExt;

        if let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) {
            unsafe {
                let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
                if libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) == 0 {
                    let s = stat.assume_init();
                    let total = s.f_blocks as u64 * s.f_frsize as u64;
                    let free = s.f_bavail as u64 * s.f_frsize as u64;
                    return (total, free);
                }
            }
        }
    }
    (128_000_000_000, 110_000_000_000)
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

static DB_CACHE: std::sync::Mutex<Option<(PathBuf, PacemakerDb)>> = std::sync::Mutex::new(None);

fn with_db<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&mut PacemakerDb) -> Result<R, String>,
{
    let db_path = resolve_pacemaker_db_path()
        .ok_or_else(|| "Pacemaker ハードウェアが接続されていません。USBケーブルで実機を接続してください。".to_string())?;

    let mut cache = DB_CACHE.lock().map_err(|e| e.to_string())?;

    let need_open = match &*cache {
        Some((path, _)) => path != &db_path || !path.exists(),
        None => true,
    };

    if need_open {
        let db = PacemakerDb::open(&db_path.to_string_lossy())
            .map_err(|e| format!("Database error: {}", e))?;
        *cache = Some((db_path, db));
    }

    let (_, ref mut db) = cache.as_mut().unwrap();
    let res = f(db);

    // Periodically release SQLite cached memory back to system for ultra-low memory footprint
    let _ = db.shrink_memory();

    res
}

#[command]
fn get_tracks(
    search: Option<String>,
    case_id: Option<i64>,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> Result<Vec<TrackMetadata>, String> {
    if resolve_pacemaker_volume().is_none() {
        return Ok(Vec::new());
    }
    with_db(|db| {
        db.get_tracks(
            search.as_deref(),
            case_id,
            sort_by.as_deref(),
            sort_order.as_deref(),
        )
        .map_err(|e| format!("Query error: {}", e))
    })
}

#[command]
fn get_cases() -> Result<Vec<CaseItem>, String> {
    if resolve_pacemaker_volume().is_none() {
        return Ok(Vec::new());
    }
    with_db(|db| db.get_cases().map_err(|e| format!("Cases error: {}", e)))
}

#[command]
fn create_case(name: String) -> Result<i64, String> {
    let id = with_db(|db| db.create_case(&name).map_err(|e| format!("Create case error: {}", e)))?;

    if let Some(vol) = resolve_pacemaker_volume() {
        touch_syncfile(&vol);
    }
    Ok(id)
}

#[command]
fn delete_case(case_id: i64) -> Result<(), String> {
    with_db(|db| db.delete_case(case_id).map_err(|e| format!("Delete case error: {}", e)))?;

    if let Some(vol) = resolve_pacemaker_volume() {
        touch_syncfile(&vol);
    }
    Ok(())
}

#[command]
fn set_cue_point(track_id: TrackId, cue_ms: TimeMs) -> Result<(), String> {
    with_db(|db| db.set_cue_point(track_id, cue_ms).map_err(|e| format!("Update error: {}", e)))?;

    if let Some(vol) = resolve_pacemaker_volume() {
        touch_syncfile(&vol);
    }
    Ok(())
}

#[command]
fn set_loop_points(track_id: TrackId, loop_in_ms: TimeMs, loop_out_ms: TimeMs) -> Result<(), String> {
    with_db(|db| db.set_loop_points(track_id, loop_in_ms, loop_out_ms).map_err(|e| format!("Update error: {}", e)))?;

    if let Some(vol) = resolve_pacemaker_volume() {
        touch_syncfile(&vol);
    }
    Ok(())
}

#[command]
fn delete_track(track_id: TrackId) -> Result<(), String> {
    with_db(|db| {
        if let Ok(Some(track)) = db.get_track(track_id) {
            if let Some(file_path) = resolve_track_file_path(&track.virtual_location) {
                let _ = fs::remove_file(&file_path);
                let mut str_path = file_path.clone();
                str_path.set_extension("str");
                let _ = fs::remove_file(&str_path);
            }
        }
        db.delete_track(track_id).map_err(|e| format!("Delete error: {}", e))
    })?;

    if let Some(vol) = resolve_pacemaker_volume() {
        touch_syncfile(&vol);
    }
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateTrackInput {
    pub track_id: TrackId,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub comments: Option<String>,
}

#[command]
fn update_track_metadata(input: UpdateTrackInput) -> Result<(), String> {
    with_db(|db| {
        db.update_track_metadata(
            input.track_id,
            input.title.as_deref(),
            input.artist.as_deref(),
            input.album.as_deref(),
            input.genre.as_deref(),
            input.year,
            input.bpm,
            input.key.as_deref(),
            input.comments.as_deref(),
        )
        .map_err(|e| format!("Update metadata error: {}", e))
    })?;

    if let Some(vol) = resolve_pacemaker_volume() {
        touch_syncfile(&vol);
    }
    Ok(())
}

#[command]
fn get_waveform_data(track_id: TrackId) -> Result<TrackAnalysedData, String> {
    let track = with_db(|db| {
        db.get_track(track_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| "Track not found".to_string())
    })?;

    let audio_path = resolve_track_file_path(&track.virtual_location)
        .ok_or_else(|| format!("Audio file not found: {}", track.virtual_location))?;

    let mut str_path = audio_path.clone();
    str_path.set_extension("str");

    if !str_path.exists() {
        return Err("No .str waveform analysis file found for track".to_string());
    }

    let xml = fs::read_to_string(&str_path)
        .map_err(|e| format!("Failed to read .str file: {}", e))?;

    TrackAnalysedData::parse_xml(&xml)
}

#[command]
fn get_track_audio_bytes(track_id: TrackId) -> Result<Vec<u8>, String> {
    let track = with_db(|db| {
        db.get_track(track_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Track ID {} not found", track_id))
    })?;

    let file_path = resolve_track_file_path(&track.virtual_location)
        .ok_or_else(|| format!("Audio file not found on disk: {}", track.virtual_location))?;

    fs::read(&file_path).map_err(|e| format!("Failed to read audio file {}: {}", file_path.display(), e))
}

#[command]
fn inspect_firmware(path: String) -> Result<PfwInfo, String> {
    PfwInfo::parse(&path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeImportItem {
    pub track_id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: u32,
    pub file_path: String,
    pub virtual_location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeImportResponse {
    pub success: bool,
    pub message: String,
    pub items: Vec<YoutubeImportItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeAuthStatus {
    pub is_logged_in: bool,
    pub cookie_file_exists: bool,
    pub cookie_file_path: String,
    pub cookie_file_size: u64,
    pub last_modified: Option<u64>,
    pub account_hint: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeAuthTestResult {
    pub success: bool,
    pub message: String,
    pub details: Option<String>,
}

fn get_pacemaker_config_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("pacemaker")
    } else {
        std::env::temp_dir().join("pacemaker")
    }
}

fn get_youtube_cookie_path() -> PathBuf {
    get_pacemaker_config_dir().join("youtube_cookies.txt")
}

fn format_relative_timestamp(secs: u64) -> String {
    if secs == 0 { return "不明".to_string(); }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if now >= secs {
        let diff = now - secs;
        if diff < 60 {
            "たった今".to_string()
        } else if diff < 3600 {
            format!("{}分前", diff / 60)
        } else if diff < 86400 {
            format!("{}時間前", diff / 3600)
        } else {
            format!("{}日前", diff / 86400)
        }
    } else {
        "直近".to_string()
    }
}

#[command]
fn get_youtube_auth_status() -> Result<YoutubeAuthStatus, String> {
    let cookie_file = get_youtube_cookie_path();
    if !cookie_file.exists() {
        return Ok(YoutubeAuthStatus {
            is_logged_in: false,
            cookie_file_exists: false,
            cookie_file_path: cookie_file.display().to_string(),
            cookie_file_size: 0,
            last_modified: None,
            account_hint: None,
            details: "Cookieファイル未設定 (ゲストモード)".to_string(),
        });
    }

    let meta = fs::metadata(&cookie_file)
        .map_err(|e| format!("ファイル情報の取得に失敗しました: {}", e))?;
    let size = meta.len();
    let mtime = meta.modified().ok().and_then(|t| {
        t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
    });

    let content = fs::read_to_string(&cookie_file).unwrap_or_default();
    let has_yt_domain = content.contains(".youtube.com") || content.contains(".google.com");
    let has_auth_tokens = content.contains("LOGIN_INFO")
        || content.contains("SAPISID")
        || content.contains("__Secure-1PSID")
        || content.contains("__Secure-3PSID")
        || content.contains("SSID")
        || content.contains("SID");

    let is_logged_in = has_yt_domain && has_auth_tokens;
    let hint = if is_logged_in {
        Some("YouTube Music ログイン有効".to_string())
    } else if has_yt_domain {
        Some("YouTube Cookieあり (未ログイン/ゲスト)".to_string())
    } else {
        None
    };

    let details = if is_logged_in {
        format!("ログイン中 ({:.1} KB, {})", size as f64 / 1024.0, format_relative_timestamp(mtime.unwrap_or(0)))
    } else {
        "Cookieが存在しますが、ログイン情報が検出されませんでした".to_string()
    };

    Ok(YoutubeAuthStatus {
        is_logged_in,
        cookie_file_exists: true,
        cookie_file_path: cookie_file.display().to_string(),
        cookie_file_size: size,
        last_modified: mtime,
        account_hint: hint,
        details,
    })
}

#[command]
fn save_youtube_cookies(content: String) -> Result<YoutubeAuthStatus, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("Cookieの内容が空です。".to_string());
    }

    let dir = get_pacemaker_config_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("設定ディレクトリの作成に失敗しました: {}", e))?;

    let cookie_file = get_youtube_cookie_path();
    let file_content = if !trimmed.starts_with("# Netscape") && trimmed.contains('\t') {
        format!("# Netscape HTTP Cookie File\n# Generated for Tonium Pacemaker\n{}\n", trimmed)
    } else {
        format!("{}\n", trimmed)
    };

    fs::write(&cookie_file, file_content)
        .map_err(|e| format!("Cookieファイルの保存に失敗しました: {}", e))?;

    get_youtube_auth_status()
}

#[command]
fn clear_youtube_cookies() -> Result<YoutubeAuthStatus, String> {
    let cookie_file = get_youtube_cookie_path();
    if cookie_file.exists() {
        let _ = fs::remove_file(&cookie_file);
    }
    get_youtube_auth_status()
}

#[command]
async fn test_youtube_auth(
    method: String,
    browser: Option<String>,
) -> Result<YoutubeAuthTestResult, String> {
    let ytdlp_path = find_executable("yt-dlp")
        .ok_or_else(|| "yt-dlp が見つかりません。'brew install yt-dlp' でインストールしてください。".to_string())?;

    let current_path = std::env::var("PATH").unwrap_or_default();
    let enhanced_path = format!("/opt/homebrew/bin:/usr/local/bin:{}", current_path);

    let mut cmd = std::process::Command::new(&ytdlp_path);
    cmd.env("PATH", &enhanced_path);
    cmd.args(&[
        "--dump-single-json",
        "--simulate",
        "--no-playlist",
    ]);

    if method == "browser" {
        let b = browser.unwrap_or_else(|| "chrome".to_string());
        cmd.args(&["--cookies-from-browser", &b]);
    } else {
        let cookie_file = get_youtube_cookie_path();
        if !cookie_file.exists() {
            return Ok(YoutubeAuthTestResult {
                success: false,
                message: "Cookieファイルが保存されていません。先にファイルを保存してください。".to_string(),
                details: None,
            });
        }
        cmd.args(&["--cookies", &cookie_file.display().to_string()]);
    }

    cmd.arg("https://music.youtube.com/watch?v=kJQP7kiw5Fk");

    let output = cmd.output().map_err(|e| format!("yt-dlp の実行に失敗しました: {}", e))?;

    if output.status.success() {
        Ok(YoutubeAuthTestResult {
            success: true,
            message: "✓ YouTube Music との通信およびログイン認証に成功しました！".to_string(),
            details: Some("セッションCookieは正常に機能しています。".to_string()),
        })
    } else {
        let err_text = String::from_utf8_lossy(&output.stderr);
        let error_msg = if err_text.contains("Operation not permitted") {
            "macOSのセキュリティ保護によりブラウザCookieへのアクセスが拒否されました。「Cookieファイル / 貼り付け」をご利用いただくか、システム設定で権限を付与してください。".to_string()
        } else if err_text.contains("could not find") && err_text.contains("cookies database") {
            "ブラウザのCookieデータベースが見つかりませんでした。対象ブラウザで一度YouTube Musicを開いてログインするか、「Cookieファイル」方式をお試しください。".to_string()
        } else {
            let last_line = err_text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("不明なエラー");
            format!("認証エラー: {}", last_line)
        };

        Ok(YoutubeAuthTestResult {
            success: false,
            message: error_msg,
            details: Some(err_text.to_string()),
        })
    }
}

#[command]
fn open_in_browser(url: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("ブラウザの起動に失敗しました: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("ブラウザの起動に失敗しました: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", &url])
            .spawn()
            .map_err(|e| format!("ブラウザの起動に失敗しました: {}", e))?;
    }
    Ok(())
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let candidates = [
        format!("/opt/homebrew/bin/{}", name),
        format!("/usr/local/bin/{}", name),
        format!("/usr/bin/{}", name),
    ];
    for c in &candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(output) = std::process::Command::new("which").arg(name).output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(PathBuf::from(s));
            }
        }
    }
    None
}

#[command]
async fn import_youtube_track(
    query_or_url: String,
    case_name: Option<String>,
    auth_method: Option<String>,
    browser_name: Option<String>,
) -> Result<YoutubeImportResponse, String> {
    let vol = resolve_pacemaker_volume()
        .ok_or_else(|| "Pacemaker 実機が接続されていません。USBケーブルで実機を接続してください。".to_string())?;

    let trimmed = query_or_url.trim();
    if trimmed.is_empty() {
        return Err("URLまたは検索ワードを入力してください。".to_string());
    }

    let ytdlp_path = find_executable("yt-dlp")
        .ok_or_else(|| "yt-dlp が見つかりません。Terminalで 'brew install yt-dlp ffmpeg' を実行してインストールしてください。".to_string())?;

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let temp_dir = std::env::temp_dir().join(format!("pacemaker_yt_{}", now_ts));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("一時ディレクトリの作成に失敗しました: {}", e))?;

    let is_url = trimmed.starts_with("http://") || trimmed.starts_with("https://");
    let target = if is_url {
        trimmed.to_string()
    } else {
        format!("ytsearch1:{}", trimmed)
    };

    let is_playlist = is_url && (trimmed.contains("list=") || trimmed.contains("/playlist"));

    let current_path = std::env::var("PATH").unwrap_or_default();
    let enhanced_path = format!("/opt/homebrew/bin:/usr/local/bin:{}", current_path);

    let mut cmd = std::process::Command::new(&ytdlp_path);
    cmd.env("PATH", &enhanced_path);
    cmd.args(&[
        "--extract-audio",
        "--audio-format", "mp3",
        "--audio-quality", "0",
        "--print-json",
        "-o", &format!("{}/%(id)s.%(ext)s", temp_dir.display()),
    ]);

    // Handle authentication cookies
    let method = auth_method.as_deref().unwrap_or("auto");
    match method {
        "browser" => {
            let b = browser_name.as_deref().unwrap_or("chrome");
            cmd.args(&["--cookies-from-browser", b]);
        }
        "none" => {
            // Guest mode
        }
        "cookies_file" | "auto" | _ => {
            let cookie_file = get_youtube_cookie_path();
            if cookie_file.exists() {
                cmd.args(&["--cookies", &cookie_file.display().to_string()]);
            }
        }
    }

    if !is_playlist {
        cmd.arg("--no-playlist");
    }

    cmd.arg(&target);

    let output = cmd.output().map_err(|e| format!("ダウンロードプロセスの実行に失敗しました: {}", e))?;

    if !output.status.success() {
        let err_text = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_dir_all(&temp_dir);
        let error_msg = if err_text.contains("Operation not permitted") {
            "macOSのセキュリティ保護によりブラウザCookieへのアクセスが制限されています。『Cookieファイル / 貼り付け』をご利用ください。".to_string()
        } else if err_text.contains("Sign in to confirm") || err_text.contains("bot") || err_text.contains("Music Premium members") {
            "YouTube Musicのログインが必要です。「🔐 ログイン設定」からCookieを設定してください。".to_string()
        } else {
            let last_line = err_text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("不明なエラー");
            format!("ダウンロードに失敗しました: {}", last_line)
        };
        return Err(error_msg);
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut imported_items = Vec::new();

    // If case_name is specified, optionally create or add to Case
    let mut target_case_id: Option<i64> = None;
    if let Some(ref cname) = case_name {
        let trimmed_case = cname.trim();
        if !trimmed_case.is_empty() {
            let cid = with_db(|db| db.create_case(trimmed_case).map_err(|e| e.to_string()))?;
            target_case_id = Some(cid);
        }
    }

    for line in stdout_str.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() || !line_trimmed.starts_with('{') {
            continue;
        }

        let json: serde_json::Value = match serde_json::from_str(line_trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let video_id = json["id"].as_str().unwrap_or("track").to_string();
        let raw_title = json["title"].as_str().unwrap_or("Unknown Title").to_string();
        let raw_artist = json["artist"]
            .as_str()
            .or_else(|| json["creator"].as_str())
            .or_else(|| json["uploader"].as_str())
            .or_else(|| json["channel"].as_str())
            .unwrap_or("YouTube Music")
            .to_string();

        let album_name = json["album"].as_str().unwrap_or("YouTube Imports").to_string();
        let duration = json["duration"].as_f64().unwrap_or(0.0).round() as u32;

        let temp_audio_file = temp_dir.join(format!("{}.mp3", video_id));
        let actual_audio_file = if temp_audio_file.exists() {
            Some(temp_audio_file)
        } else {
            // Find any mp3 in temp_dir
            fs::read_dir(&temp_dir).ok().and_then(|entries| {
                entries.flatten().find_map(|e| {
                    let p = e.path();
                    if p.extension().and_then(|ext| ext.to_str()) == Some("mp3") {
                        Some(p)
                    } else {
                        None
                    }
                })
            })
        };

        let audio_file = match actual_audio_file {
            Some(f) => f,
            None => continue,
        };

        let file_size = fs::metadata(&audio_file).map(|m| m.len()).unwrap_or(0);
        if file_size == 0 {
            continue;
        }

        // Generate official Pacemaker filesystem path: .Pacemaker/Music/X/Y/HASH.mp3
        let paths = pacemaker_core::hasher::generate_track_paths(&raw_title, file_size, "mp3");
        let dest_path = vol.join(&paths.relative_music_path);

        if let Some(parent) = dest_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        fs::copy(&audio_file, &dest_path)
            .map_err(|e| format!("Pacemaker ストレージへのファイル転送に失敗しました: {}", e))?;

        // Generate corresponding waveform .str file
        let str_path = dest_path.with_extension("str");
        let str_content = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<TrackAnalysedData>\n\t<normalizeFactor>1</normalizeFactor>\n\t<beatStartOffset>0</beatStartOffset>\n\t<realBeatLocations></realBeatLocations>\n</TrackAnalysedData>\n";
        let _ = fs::write(&str_path, str_content);

        // Register in SQLite music.db
        let track_input = pacemaker_core::db::NewTrackInput {
            title: raw_title.clone(),
            artist: raw_artist.clone(),
            album: Some(album_name.clone()),
            genre: Some("YouTube Music".into()),
            year: Some(2026),
            play_time_secs: duration,
            file_size,
            bit_rate: 320,
            sample_rate: 44100,
            format: "MPEG-1 Layer 3".into(),
            location: paths.virtual_location.clone(),
            bpm: None,
            key: None,
            comments: Some(format!("Imported from YouTube: {}", video_id)),
        };

        let track_id = with_db(|db| {
            let id = db.insert_track(&track_input)
                .map_err(|e| format!("Database insert error: {}", e))?;
            if let Some(cid) = target_case_id {
                let _ = db.add_track_to_case(cid, id);
            }
            Ok(id)
        })?;

        imported_items.push(YoutubeImportItem {
            track_id,
            title: raw_title,
            artist: raw_artist,
            album: album_name,
            duration_secs: duration,
            file_path: dest_path.display().to_string(),
            virtual_location: paths.virtual_location,
        });
    }

    // Touch Pacemaker syncfile to notify hardware of update
    touch_syncfile(&vol);

    // Clean up temporary download folder
    let _ = fs::remove_dir_all(&temp_dir);

    if imported_items.is_empty() {
        return Err("音声を変換・保存できませんでした。URLまたは検索ワードをご確認ください。".to_string());
    }

    Ok(YoutubeImportResponse {
        success: true,
        message: format!("{} 曲を Pacemaker に正常にダウンロード・登録しました！", imported_items.len()),
        items: imported_items,
    })
}

fn serve_audio_request(req: tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    let path = req.uri().path();
    // Path format: /tracks/{track_id}
    let track_id_opt: Option<i64> = path
        .strip_prefix("/tracks/")
        .and_then(|s| s.split('/').next())
        .and_then(|s| s.parse().ok());

    let file_path = match track_id_opt {
        Some(tid) => {
            with_db(|db| {
                if let Ok(Some(track)) = db.get_track(tid) {
                    Ok(resolve_track_file_path(&track.virtual_location))
                } else {
                    Ok(None)
                }
            }).unwrap_or(None)
        }
        None => None,
    };

    let target_file = match file_path {
        Some(f) if f.exists() => f,
        _ => {
            return tauri::http::Response::builder()
                .status(404)
                .header("Content-Type", "text/plain")
                .body(b"Audio track file not found".to_vec())
                .unwrap();
        }
    };

    let mut file = match fs::File::open(&target_file) {
        Ok(f) => f,
        Err(_) => {
            return tauri::http::Response::builder()
                .status(500)
                .body(b"Unable to open file".to_vec())
                .unwrap();
        }
    };

    let file_size = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => 0,
    };

    let ext = target_file.extension().and_then(|s| s.to_str()).unwrap_or("");
    let mime_type = match ext.to_lowercase().as_str() {
        "m4a" | "mp4" => "audio/mp4",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "aif" | "aiff" => "audio/aiff",
        _ => "audio/mpeg",
    };

    // Handle HTTP Range
    if let Some(range_val) = req.headers().get("range").and_then(|h| h.to_str().ok()) {
        if let Some(spec) = range_val.strip_prefix("bytes=") {
            let parts: Vec<&str> = spec.split('-').collect();
            let start: u64 = parts[0].parse().unwrap_or(0);
            let end: u64 = if parts.len() > 1 && !parts[1].is_empty() {
                parts[1].parse().unwrap_or(file_size.saturating_sub(1))
            } else {
                file_size.saturating_sub(1)
            }.min(file_size.saturating_sub(1));

            // Limit single range response to 64KB for strict low-memory footprint
            let max_range = 65536u64;
            let end = end.min(start + max_range - 1);

            let len = if end >= start { (end - start) + 1 } else { 0 };
            let mut buf = vec![0u8; len as usize];
            if file.seek(SeekFrom::Start(start)).is_ok() && file.read_exact(&mut buf).is_ok() {
                return tauri::http::Response::builder()
                    .status(206)
                    .header("Content-Type", mime_type)
                    .header("Content-Range", format!("bytes {}-{}/{}", start, end, file_size))
                    .header("Content-Length", len.to_string())
                    .header("Accept-Ranges", "bytes")
                    .body(buf)
                    .unwrap();
            }
        }
    }

    // Low-memory streaming fallback: read only initial 64KB chunk instead of buffering full file into RAM
    let max_initial = 65536usize;
    let chunk_len = (file_size as usize).min(max_initial);
    let mut initial_buf = vec![0u8; chunk_len];
    let n = file.read(&mut initial_buf).unwrap_or(0);
    initial_buf.truncate(n);

    tauri::http::Response::builder()
        .status(200)
        .header("Content-Type", mime_type)
        .header("Content-Length", file_size.to_string())
        .header("Accept-Ranges", "bytes")
        .body(initial_buf)
        .unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .register_uri_scheme_protocol("pmaudio", |_ctx, req| {
            serve_audio_request(req)
        })
        .invoke_handler(tauri::generate_handler![
            get_device_status,
            get_tracks,
            get_cases,
            create_case,
            delete_case,
            set_cue_point,
            set_loop_points,
            delete_track,
            update_track_metadata,
            get_waveform_data,
            get_track_audio_bytes,
            inspect_firmware,
            import_youtube_track,
            get_youtube_auth_status,
            save_youtube_cookies,
            clear_youtube_cookies,
            test_youtube_auth,
            open_in_browser
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_device_status_and_tracks() {
        let _guard = TEST_MUTEX.lock().unwrap();
        std::env::set_var("PACEMAKER_MOCK", "1");
        let status = get_device_status();
        println!("Device status: {:?}", status);
        let tracks = get_tracks(None, None, None, None).expect("Failed to get tracks");
        println!("Track count: {}", tracks.len());
        assert!(!tracks.is_empty(), "Track list should not be empty with PACEMAKER_MOCK=1");

        let first_id = tracks[0].id;
        let bytes = get_track_audio_bytes(first_id);
        println!("Track {} audio bytes: {:?}", first_id, bytes.as_ref().map(|b| b.len()));
        if let Ok(b) = bytes {
            assert!(b.len() > 10_000);
        }
        std::env::remove_var("PACEMAKER_MOCK");
    }

    #[test]
    fn test_youtube_auth_management() {
        let _guard = TEST_MUTEX.lock().unwrap();
        // Save test cookies
        let test_cookie = "# Netscape HTTP Cookie File\n.youtube.com\tTRUE\t/\tTRUE\t2147483647\tLOGIN_INFO\tAFmmF2swRQIhAO...\n.youtube.com\tTRUE\t/\tTRUE\t2147483647\tSAPISID\tabcd1234\n";
        let status = save_youtube_cookies(test_cookie.to_string()).expect("Failed to save cookies");
        assert!(status.is_logged_in, "Auth status should be logged in with valid tokens");
        assert!(status.cookie_file_exists, "Cookie file should exist");
        assert!(status.cookie_file_size > 0);

        // Verify retrieval
        let retrieved = get_youtube_auth_status().expect("Failed to get auth status");
        assert!(retrieved.is_logged_in);

        // Clear cookies
        let cleared = clear_youtube_cookies().expect("Failed to clear cookies");
        assert!(!cleared.is_logged_in);
        assert!(!cleared.cookie_file_exists);
    }
}

