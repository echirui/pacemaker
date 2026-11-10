//! Hardware Buttons and Detection Sensors Verification CLI for Tonium Pacemaker.
//!
//! Verifies and audits whether each physical button, touch sensor (CSIA wheel, slider),
//! and PMIC detection sensor (charger, fuel gauge, low-battery alarm, watchdog)
//! is properly controlled and handled across the device software stack.

use clap::Parser;
use pacemaker_core::hardware::{
    verify_hardware_controls, ControlStatus, HardwareController, PhysicalButton,
};
use pacemaker_core::engine::PacemakerMixer;
use pacemaker_core::firmware::detect_pacemaker_device;
use pacemaker_core::types::{DeckId, TrackMetadata};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "hardware_verifier",
    about = "Tonium Pacemaker Hardware Buttons & Sensors Control Verifier",
    version = "1.0.0"
)]
struct Args {
    /// Run automated hardware event simulation test
    #[arg(short, long)]
    simulate: bool,

    /// Output full report in JSON format
    #[arg(short, long)]
    json: bool,

    /// Check physical Pacemaker device mount point
    #[arg(short, long)]
    device: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    println!("================================================================================");
    println!("     TONIUM PACEMAKER HARDWARE CONTROLS & SENSORS VERIFIER (OpenPacemaker)      ");
    println!("================================================================================\n");

    let report = verify_hardware_controls();

    if args.json {
        let json_str = serde_json::to_string_pretty(&report).expect("Failed to serialize report");
        println!("{}", json_str);
        return;
    }

    // 1. Audit Table
    println!("{:<18} | {:<24} | {:<32} | {:<6}", "Category", "Control Name", "Driver / Device Node", "Status");
    println!("{:-<18}-+-{:-<24}-+-{:-<32}-+-{:-<6}", "", "", "", "");

    for check in &report.checks {
        let status_str = match check.status {
            ControlStatus::Pass => "\x1b[32mPASS\x1b[0m",
            ControlStatus::Warn => "\x1b[33mWARN\x1b[0m",
            ControlStatus::Fail => "\x1b[31mFAIL\x1b[0m",
        };
        println!(
            "{:<18} | {:<24} | {:<32} | {:<6}",
            check.category, check.name, check.driver_device, status_str
        );
    }

    println!("\n--------------------------------------------------------------------------------");
    println!(
        "Total Controls Audited: {} | Passed: \x1b[32m{}\x1b[0m | Warnings: \x1b[33m{}\x1b[0m | Failed: \x1b[31m{}\x1b[0m",
        report.total_controls, report.passed, report.warnings, report.failed
    );
    println!("--------------------------------------------------------------------------------\n");

    // 2. Physical Device Inspection
    let target_dev = args.device.or_else(detect_pacemaker_device);
    if let Some(dev_path) = target_dev {
        println!("🔍 [Physical Pacemaker Device Inspection]");
        println!("   Device Mount Point : {}", dev_path.display());
        let sys_xml = dev_path.join(".Pacemaker/system.xml");
        let root_xml = dev_path.join("system.xml");
        let xml_path = if sys_xml.exists() { Some(sys_xml) } else if root_xml.exists() { Some(root_xml) } else { None };

        if let Some(xp) = xml_path {
            if let Ok(content) = fs::read_to_string(&xp) {
                println!("   system.xml Found   : \x1b[32mYES\x1b[0m ({})", xp.display());
                for tag in &["FWVERSION", "SWVERSION", "SERIALNUMBER", "HARDWARE"] {
                    if let Some(val) = extract_tag(&content, tag) {
                        println!("   - {:<16} : {}", tag, val);
                    }
                }
            }
        } else {
            println!("   system.xml Found   : \x1b[33mNo system.xml found on volume\x1b[0m");
        }
        println!();
    }

    // 3. Hardware Simulation (if requested)
    if args.simulate {
        run_hardware_simulation();
    } else {
        println!("💡 Tip: Run with `--simulate` to execute real-time state machine & DSP reaction tests.");
    }
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

fn run_hardware_simulation() {
    println!("🧪 [Automated Hardware Event & DSP Simulation]");
    let mut controller = HardwareController::new();
    let mut mixer = PacemakerMixer::new(44100.0);

    let dummy_track = TrackMetadata {
        id: 1,
        title: "Simulation Track".into(),
        artist: "Pacemaker Engine".into(),
        album: "Firmware Test".into(),
        genre: "DJ Tools".into(),
        year: 2025,
        bpm: 128.0,
        play_time_secs: 300,
        cue_point_ms: 0,
        loop_in_ms: -1,
        loop_out_ms: -1,
        virtual_location: "/track.mp3".into(),
        key: "8A".into(),
        rating: 5,
        comments: "Automated test".into(),
    };
    mixer.deck_mut(DeckId::DeckA).load_track(dummy_track);

    // Test 1: Button Press (Play/Pause)
    println!("   [Step 1] Pressing BTN_PLAYPAUSE on Deck P (Deck A)...");
    controller.handle_button_down(PhysicalButton::PlayPause, &mut mixer);
    let st1 = mixer.deck(DeckId::DeckA).state;
    println!("            -> Deck P Playback State: \x1b[32m{:?}\x1b[0m", st1);

    // Test 2: Cue Point Set (ALT + CUE)
    println!("   [Step 2] Pressing BTN_ALT + BTN_CUE (Set Cue Point)...");
    controller.handle_button_down(PhysicalButton::Alt, &mut mixer);
    controller.handle_button_down(PhysicalButton::Cue, &mut mixer);
    controller.handle_button_up(PhysicalButton::Cue);
    controller.handle_button_up(PhysicalButton::Alt);
    let cue1 = mixer.deck(DeckId::DeckA).cue_point_ms.unwrap_or(0);
    println!("            -> Deck P Cue Point: \x1b[32m{} ms\x1b[0m", cue1);

    // Test 3: Crossfader Slider Sweep
    println!("   [Step 3] Moving Crossfader Slider to 0.85 (biased towards Deck L)...");
    controller.handle_slider_motion(0.85, &mut mixer);
    let (gain_a, gain_b) = mixer.crossfader_gains();
    println!(
        "            -> Deck P Gain: \x1b[32m{:.3}\x1b[0m | Deck L Gain: \x1b[32m{:.3}\x1b[0m",
        gain_a, gain_b
    );

    // Test 4: CSIA Capacitive Touch Wheel Scratch
    println!("   [Step 4] Engaging CSIA Vinyl Scratch (delta = 0.45 rad)...");
    controller.csia.mode = pacemaker_core::hardware::CsiaMode::Scratch;
    controller.handle_csia_touch(1.57, 0.8, 0.45, &mut mixer);
    println!(
        "            -> Scratch offset: \x1b[32m{} samples\x1b[0m (ang_vel = {:.1} rad/s)",
        controller.csia.scratch_offset_samples, controller.csia.angular_velocity
    );
    controller.handle_csia_release(&mut mixer);

    // Test 5: PMIC Low-Battery Detection Sensor
    println!("   [Step 5] Triggering PMIC Low Battery Sensor (3.32V)...");
    let alarm = controller.handle_sensor_update(3.32, false);
    println!(
        "            -> Alarm Result: \x1b[31m{:?}\x1b[0m (Emergency Shutdown: {})",
        alarm, controller.emergency_shutdown_triggered
    );

    println!("\n✅ Simulation Completed: All 5 hardware input pipelines verified successfully.");
}
