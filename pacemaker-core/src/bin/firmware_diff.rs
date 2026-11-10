use pacemaker_core::firmware::{ALL_FIRMWARE_PROFILES, compare_firmwares, FirmwareVersion};

fn main() {
    println!("=========================================================================================");
    println!("             Tonium Pacemaker Firmware Matrix & Rust Re-implementation                   ");
    println!("=========================================================================================\n");

    println!("{:<12} | {:<10} | {:<20} | {:<12} | {:<10} | {:<10}", 
        "Version", "FW Code", "Branch", "pmplayer", "libAAC", "libMP3");
    println!("{:-<12}-+-{:-<10}-+-{:-<20}-+-{:-<12}-+-{:-<10}-+-{:-<10}", "", "", "", "", "", "");

    for p in ALL_FIRMWARE_PROFILES {
        println!("{:<12} | {:<10} | {:<20} | {:>10} B | {:>8} B | {:>8} B",
            p.tag,
            p.fw_code,
            p.branch,
            p.pmplayer_size,
            p.aac_decoder_size,
            p.mp3_decoder_size
        );
    }

    println!("\n=========================================================================================");
    println!("                           Version Feature Profiles & Capabilities                        ");
    println!("=========================================================================================\n");

    for p in ALL_FIRMWARE_PROFILES {
        println!("▼ Version {} (FW: {}, Branch: {})", p.tag, p.fw_code, p.branch);
        println!("  Description: {}", p.description);
        if p.features.is_empty() {
            println!("  Features: (Baseline features only: Freeverb Reverb, Basic Mixer/Pitch)");
        } else {
            println!("  Features ({}):", p.features.len());
            for f in p.features {
                println!("    * {}", f.name());
            }
        }
        println!();
    }

    println!("=========================================================================================");
    println!("                   Evolutionary Diffs vs Reference V14142 (Device FW)                    ");
    println!("=========================================================================================\n");

    let ref_v = FirmwareVersion::V14142;
    for target in &[
        FirmwareVersion::V13557,
        FirmwareVersion::V16219,
        FirmwareVersion::V13606814,
        FirmwareVersion::V146061537,
        FirmwareVersion::V180096069,
    ] {
        let diff = compare_firmwares(ref_v, *target);
        println!("▷ Diff: Reference 14142 -> {}", target.profile().tag);
        println!("  - pmplayer size delta: {:+10} bytes", diff.pmplayer_size_delta);
        println!("  - AAC decoder delta:   {:+10} bytes", diff.aac_decoder_size_delta);
        println!("  - MP3 decoder delta:   {:+10} bytes", diff.mp3_decoder_size_delta);
        if !diff.added_features.is_empty() {
            println!("  - Added features:");
            for f in &diff.added_features {
                println!("     + {}", f.name());
            }
        }
        if !diff.removed_features.is_empty() {
            println!("  - Removed/Replaced features:");
            for f in &diff.removed_features {
                println!("     - {}", f.name());
            }
        }
        println!();
    }
}
