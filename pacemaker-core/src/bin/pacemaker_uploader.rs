//! Tonium Pacemaker Firmware & Music Uploader (Clean-room Rust implementation)
//! Replaces the legacy Windows / Mac 32-bit PairingWizard and Firmware Updater.

use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use pacemaker_core::firmware::{detect_pacemaker_device, PfwInfo};
use pacemaker_core::hasher::generate_track_paths;

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
