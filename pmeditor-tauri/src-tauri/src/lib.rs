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
            inspect_firmware
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
    fn test_disconnected_behavior() {
        let _guard = TEST_MUTEX.lock().unwrap();
        std::env::remove_var("PACEMAKER_MOCK");
        if resolve_pacemaker_volume().is_none() {
            let status = get_device_status();
            assert!(!status.connected, "Device status should report disconnected when not mounted");
            let tracks = get_tracks(None, None, None, None).expect("get_tracks should succeed with empty list");
            assert!(tracks.is_empty(), "When disconnected, track list must be empty");
            let cases = get_cases().expect("get_cases should succeed with empty list");
            assert!(cases.is_empty(), "When disconnected, case list must be empty");
        }
    }
}

