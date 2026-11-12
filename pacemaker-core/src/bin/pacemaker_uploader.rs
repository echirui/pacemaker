//! Tonium Pacemaker Firmware & Music Uploader (Clean-room Rust implementation)
//! Replaces the legacy Windows / Mac 32-bit PairingWizard and Firmware Updater.

use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use pacemaker_core::firmware::{detect_pacemaker_device, PfwInfo};

#[derive(Parser)]
#[command(name = "pacemaker-uploader")]
#[command(author = "OpenPacemaker Team")]
#[command(version = "1.0.0")]
#[command(about = "Modern cross-platform Firmware & Media Uploader for Tonium Pacemaker")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect a .pfw firmware file
    Inspect {
        /// Path to .pfw firmware image
        firmware: PathBuf,
    },
    /// Upload firmware to a connected Pacemaker device
    Upload {
        /// Path to .pfw firmware image
        firmware: PathBuf,

        /// Custom Pacemaker volume path (optional, auto-detected if omitted)
        #[arg(short, long)]
        dest: Option<PathBuf>,

        /// Skip version check / confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Scan and display connected Tonium Pacemaker device info
    Device,
    /// Download and import music from YouTube or YouTube Music to Pacemaker device
    ImportYt {
        /// YouTube / YouTube Music URL or search query (e.g. "Artist - Title")
        query: String,

        /// Optional Playlist (Case) name to add the track to
        #[arg(short, long)]
        case: Option<String>,

        /// Custom Pacemaker volume path (optional, auto-detected if omitted)
        #[arg(short, long)]
        dest: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { firmware } => {
            cmd_inspect(&firmware);
        }
        Commands::Upload { firmware, dest, force } => {
            cmd_upload(&firmware, dest, force);
        }
        Commands::Device => {
            cmd_device();
        }
        Commands::ImportYt { query, case, dest } => {
            cmd_import_yt(&query, case.as_deref(), dest.as_deref());
        }
    }
}

fn cmd_inspect(path: &Path) {
    println!("\n📦 === Tonium Pacemaker Firmware Image Inspector ===");
    println!("File: {}", path.display());

    match PfwInfo::parse(path) {
        Ok(info) => {
            println!("Firmware Name:       {}", info.name);
            println!("Firmware Version:    {}", info.fw_version);
            println!("Software Version:    {}", info.sw_version);
            println!("Min Bootloader Ver:  {}", info.min_bootloader_version);
            println!("UUID / Signature:    {}", info.signature);
            println!("Total File Size:     {:.2} MB ({} bytes)", info.file_size as f64 / 1_048_576.0, info.file_size);
            println!("ext3 Partition Size: {:.2} MB ({} bytes)", info.ext3_image_size as f64 / 1_048_576.0, info.ext3_image_size);
            println!("Footer Status:       ✓ Valid 512-byte PfwHeader detected");
            println!("====================================================\n");
        }
        Err(e) => {
            eprintln!("❌ Error inspecting firmware: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_device() {
    println!("\n🔍 Scanning for connected Pacemaker device...");
    if let Some(dev) = detect_pacemaker_device() {
        println!("✓ Pacemaker Volume Found: {}", dev.display());
        let sys_xml = dev.join(".Pacemaker/system.xml");
        if sys_xml.exists() {
            if let Ok(xml) = fs::read_to_string(&sys_xml) {
                println!("\n--- Device system.xml ---");
                println!("{}", xml.trim());
            }
        }
    } else {
        println!("❌ No Tonium Pacemaker device detected.");
        println!("Please connect your Pacemaker via USB cable.");
    }
    println!();
}

fn cmd_upload(fw_path: &Path, custom_dest: Option<PathBuf>, force: bool) {
    println!("\n🚀 === Tonium Pacemaker Firmware Uploader ===");

    // 1. Validate Firmware File
    let fw_info = match PfwInfo::parse(fw_path) {
        Ok(info) => {
            println!("Loaded Firmware: {} (v{})", info.name, info.fw_version);
            info
        }
        Err(e) => {
            eprintln!("❌ Invalid firmware file: {}", e);
            std::process::exit(1);
        }
    };

    // 2. Discover Pacemaker Destination
    let dest_dir = if let Some(d) = custom_dest {
        d
    } else if let Some(d) = detect_pacemaker_device() {
        d
    } else {
        eprintln!("❌ Error: Pacemaker device not detected.");
        eprintln!("Connect the Pacemaker or specify destination with `--dest <path>`");
        std::process::exit(1);
    };

    println!("Target Volume:   {}", dest_dir.display());

    // 3. Prepare target file path
    // As seen in DeviceCommunication::updateFirmware: copies as `dfw_XXXXX.pfw`
    let fw_filename = fw_path.file_name().unwrap_or_default().to_string_lossy();
    let target_file = if dest_dir.join(".Pacemaker").exists() {
        dest_dir.join(".Pacemaker").join(&*fw_filename)
    } else {
        dest_dir.join(&*fw_filename)
    };

    println!("Target Path:     {}", target_file.display());

    if !force {
        print!("Proceed with firmware upload? [y/N]: ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Upload canceled.");
            return;
        }
    }

    // 4. Stream upload with progress bar
    println!("\nUploading firmware to Pacemaker storage...");
    let mut src = match File::open(fw_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot open firmware file: {}", e);
            std::process::exit(1);
        }
    };

    let mut dest = match File::create(&target_file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot create target file on device: {}", e);
            std::process::exit(1);
        }
    };

    let pb = ProgressBar::new(fw_info.file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut buf = [0u8; 1024 * 64]; // 64KB buffer
    let mut transferred: u64 = 0;
    let start_time = Instant::now();

    loop {
        let n = match src.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                eprintln!("\nRead error during upload: {}", e);
                std::process::exit(1);
            }
        };

        if let Err(e) = dest.write_all(&buf[..n]) {
            eprintln!("\nWrite error to device: {}", e);
            std::process::exit(1);
        }

        transferred += n as u64;
        pb.set_position(transferred);
    }

    dest.flush().unwrap();
    pb.finish_with_message("Upload complete!");

    let elapsed = start_time.elapsed().as_secs_f64();
    let speed_mb = (transferred as f64 / 1_048_576.0) / elapsed.max(0.001);
    println!("✓ Transferred {:.2} MB in {:.1}s ({:.2} MB/s)", transferred as f64 / 1_048_576.0, elapsed, speed_mb);

    // 5. Update sync flag
    let syncfile = dest_dir.join(".Pacemaker/syncfile");
    if syncfile.parent().map(|p| p.exists()).unwrap_or(false) {
        let _ = fs::write(syncfile, [0xfe]);
    }

    // 6. Final Instructions
    println!("\n========================================================");
    println!("🎉 Firmware Successfully Uploaded to Tonium Pacemaker!");
    println!("========================================================");
    println!("Instructions to complete update:");
    println!("  1. Safely eject the Pacemaker volume from your OS.");
    println!("  2. Unplug the USB cable.");
    println!("  3. The Pacemaker will automatically boot into update mode,");
    println!("     flash the new firmware banks, and restart.");
    println!("========================================================\n");
}

fn cmd_import_yt(query: &str, case: Option<&str>, custom_dest: Option<&Path>) {
    println!("\n🎵 === Tonium Pacemaker YouTube Music Importer ===");
    println!("Query / Target: {}", query);

    let vol = match custom_dest.map(PathBuf::from).or_else(detect_pacemaker_device) {
        Some(v) => v,
        None => {
            eprintln!("❌ Pacemaker device not detected. Connect device via USB or specify --dest <path>");
            std::process::exit(1);
        }
    };
    println!("✓ Target Device Volume: {}", vol.display());

    let ytdlp_candidates = [
        "/opt/homebrew/bin/yt-dlp",
        "/usr/local/bin/yt-dlp",
        "yt-dlp",
    ];
    let ytdlp_bin = ytdlp_candidates.iter().find(|p| Path::new(p).exists())
        .copied().unwrap_or("yt-dlp");

    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let temp_dir = std::env::temp_dir().join(format!("pm_uploader_yt_{}", now_ts));
    let _ = fs::create_dir_all(&temp_dir);

    println!("⬇️  Downloading and extracting audio via yt-dlp...");
    let is_url = query.starts_with("http://") || query.starts_with("https://");
    let target = if is_url { query.to_string() } else { format!("ytsearch1:{}", query) };
    let is_playlist = is_url && (query.contains("list=") || query.contains("/playlist"));

    let current_path = std::env::var("PATH").unwrap_or_default();
    let enhanced_path = format!("/opt/homebrew/bin:/usr/local/bin:{}", current_path);

    let mut cmd = std::process::Command::new(ytdlp_bin);
    cmd.env("PATH", &enhanced_path);
    cmd.args(&[
        "--extract-audio",
        "--audio-format", "mp3",
        "--audio-quality", "0",
        "--print-json",
        "-o", &format!("{}/%(id)s.%(ext)s", temp_dir.display()),
    ]);
    if !is_playlist {
        cmd.arg("--no-playlist");
    }
    cmd.arg(&target);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("❌ Failed to execute yt-dlp: {}", e);
            eprintln!("Please ensure yt-dlp and ffmpeg are installed: 'brew install yt-dlp ffmpeg'");
            let _ = fs::remove_dir_all(&temp_dir);
            std::process::exit(1);
        }
    };

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        eprintln!("❌ Download failed: {}", err.lines().last().unwrap_or(&err));
        let _ = fs::remove_dir_all(&temp_dir);
        std::process::exit(1);
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let db_path = vol.join(".Pacemaker/music.db");
    let db = match pacemaker_core::db::PacemakerDb::open(&db_path.to_string_lossy()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("❌ Failed to open Pacemaker database at {}: {}", db_path.display(), e);
            let _ = fs::remove_dir_all(&temp_dir);
            std::process::exit(1);
        }
    };

    let case_id = case.and_then(|cname| {
        db.create_case(cname).ok()
    });

    let mut count = 0;
    for line in stdout_str.lines() {
        let line_trimmed = line.trim();
        if !line_trimmed.starts_with('{') { continue; }
        let json: serde_json::Value = match serde_json::from_str(line_trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let video_id = json["id"].as_str().unwrap_or("track");
        let title = json["title"].as_str().unwrap_or("Unknown Title");
        let artist = json["artist"].as_str()
            .or_else(|| json["creator"].as_str())
            .or_else(|| json["uploader"].as_str())
            .unwrap_or("YouTube Music");
        let album = json["album"].as_str().unwrap_or("YouTube Imports");
        let duration = json["duration"].as_f64().unwrap_or(0.0).round() as u32;

        let audio_file = temp_dir.join(format!("{}.mp3", video_id));
        if !audio_file.exists() { continue; }
        let file_size = fs::metadata(&audio_file).map(|m| m.len()).unwrap_or(0);
        if file_size == 0 { continue; }

        let paths = pacemaker_core::hasher::generate_track_paths(title, file_size, "mp3");
        let dest_file = vol.join(&paths.relative_music_path);
        if let Some(p) = dest_file.parent() { let _ = fs::create_dir_all(p); }
        if let Err(e) = fs::copy(&audio_file, &dest_file) {
            eprintln!("❌ Error copying {} to {}: {}", audio_file.display(), dest_file.display(), e);
            continue;
        }

        let str_file = dest_file.with_extension("str");
        let str_content = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<TrackAnalysedData>\n\t<normalizeFactor>1</normalizeFactor>\n\t<beatStartOffset>0</beatStartOffset>\n\t<realBeatLocations></realBeatLocations>\n</TrackAnalysedData>\n";
        let _ = fs::write(&str_file, str_content);

        let input = pacemaker_core::db::NewTrackInput {
            title: title.to_string(),
            artist: artist.to_string(),
            album: Some(album.to_string()),
            genre: Some("YouTube Music".to_string()),
            year: Some(2026),
            play_time_secs: duration,
            file_size,
            bit_rate: 320,
            sample_rate: 44100,
            format: "MPEG-1 Layer 3".to_string(),
            location: paths.virtual_location,
            bpm: None,
            key: None,
            comments: Some(format!("Imported from YouTube: {}", video_id)),
        };

        if let Ok(tid) = db.insert_track(&input) {
            if let Some(cid) = case_id {
                let _ = db.add_track_to_case(cid, tid);
            }
            println!("  ✓ Registered: {} - {} (ID: {})", artist, title, tid);
            count += 1;
        }
    }

    let syncfile = vol.join(".Pacemaker/syncfile");
    let _ = fs::write(syncfile, b"1");
    let _ = fs::remove_dir_all(&temp_dir);

    println!("\n🎉 Successfully imported {} track(s) to Tonium Pacemaker!", count);
    println!("========================================================\n");
}
