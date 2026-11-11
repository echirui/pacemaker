//! Pacemaker Firmware (`.pfw`) inspection and upload module.
//!
//! Reconstructed from `PfwFile::readFwVersion` and `DeviceCommunication::copyFirmwareToDevice`.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PfwInfo {
    pub name: String,
    pub fw_version: String,
    pub sw_version: String,
    pub min_bootloader_version: String,
    pub signature: String,
    pub checksum_raw: String,
    pub file_size: u64,
    pub ext3_image_size: u64,
}

impl PfwInfo {
    /// Parses a `.pfw` file and its 512-byte footer
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut file = File::open(&path).map_err(|e| format!("Cannot open file: {}", e))?;
        let metadata = file.metadata().map_err(|e| format!("Cannot read metadata: {}", e))?;
        let file_size = metadata.len();

        if file_size < 512 {
            return Err("File too small to be a valid .pfw image".to_string());
        }

        // Seek to 512 bytes from the end
        file.seek(SeekFrom::End(-512))
            .map_err(|e| format!("Seek failed: {}", e))?;

        let mut footer = [0u8; 512];
        file.read_exact(&mut footer)
            .map_err(|e| format!("Read footer failed: {}", e))?;

        // Helper to parse Pascal/length-prefixed ASCII string as used in PfwFile::readFwVersion
        let read_prefixed_str = |offset: usize| -> String {
            if offset + 2 > footer.len() {
                return String::new();
            }
            let b0 = footer[offset];
            let b1 = footer[offset + 1];
            if b0 >= b'0' && b0 <= b'9' && b1 >= b'0' && b1 <= b'9' {
                let len = ((b0 - b'0') as usize) * 10 + ((b1 - b'0') as usize);
                let start = offset + 2;
                let end = (start + len).min(footer.len());
                String::from_utf8_lossy(&footer[start..end])
                    .trim_matches('\0')
                    .to_string()
            } else {
                // Fallback: read null-terminated string
                let slice = &footer[offset..];
                let nul = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
                String::from_utf8_lossy(&slice[..nul]).trim().to_string()
            }
        };

        // Offset 0x00: Name (e.g. "020109dfw_14142")
        let name_slice = &footer[0..128];
        let nul = name_slice.iter().position(|&c| c == 0).unwrap_or(128);
        let name = String::from_utf8_lossy(&name_slice[..nul]).trim().to_string();

        let sw_version = read_prefixed_str(0x84);
        let fw_version = read_prefixed_str(0xa4);
        let min_bootloader_version = read_prefixed_str(0xc4);

        // Offset 0xe4: UUID Signature (e.g. "24ab9d46ef-487c-40a5-b073-19a6f07c23c800")
        let sig_slice = &footer[0xe4..0x120];
        let nul_sig = sig_slice.iter().position(|&c| c == 0).unwrap_or(sig_slice.len());
        let signature = String::from_utf8_lossy(&sig_slice[..nul_sig]).trim().to_string();

        // Offset 0x1de: Checksum
        let chk_slice = &footer[0x1de..0x200];
        let nul_chk = chk_slice.iter().position(|&c| c == 0).unwrap_or(chk_slice.len());
        let checksum_raw = String::from_utf8_lossy(&chk_slice[..nul_chk]).trim().to_string();

        Ok(PfwInfo {
            name,
            fw_version,
            sw_version,
            min_bootloader_version,
            signature,
            checksum_raw,
            file_size,
            ext3_image_size: file_size - 512,
        })
    }
}

/// Known Tonium Pacemaker firmware releases
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FirmwareVersion {
    V13557,
    V13606814,
    V14108,
    V14142,
    V146061537,
    V16219,
    V180096069,
}

impl FirmwareVersion {
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "13557" | "0513557" => Some(Self::V13557),
            "13606814" | "0813606814" => Some(Self::V13606814),
            "14108" | "0514108" => Some(Self::V14108),
            "14142" | "0514142" => Some(Self::V14142),
            "146061537" | "09146061537" => Some(Self::V146061537),
            "16219" | "0516219" => Some(Self::V16219),
            "180096069" | "09180096069" => Some(Self::V180096069),
            _ => None,
        }
    }

    pub fn profile(&self) -> &'static FirmwareProfile {
        ALL_FIRMWARE_PROFILES
            .iter()
            .find(|p| p.version == *self)
            .expect("Profile must exist")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FirmwareFeature {
    /// Adaptive Simulated Annealing for tempo/beatgrid estimation
    AdaptiveSimulatedAnnealing,
    /// Realtime beat-quantized jumping (BeatJump)
    BeatJump,
    /// Automated loop points manager (LoopManager)
    LoopManager,
    /// Master limiter DSP (AudioPeakLimiter)
    AudioPeakLimiter,
    /// Feedback delay line DSP (Delayline)
    Delayline,
    /// Remote debug socket server (DebugServerProxy)
    DebugServerProxy,
    /// Pacemaker Editor direct communication adapter (PECommsAdapter)
    PECommsAdapter,
    /// Windowed sinc resampling interpolator (SincInterpolator)
    SincInterpolator,
    /// Large 560KB extended AAC/MPEG4 decoder
    LargeAACDecoder,
    /// Large 150KB extended MP3 decoder
    LargeMP3Decoder,
}

impl FirmwareFeature {
    pub fn name(&self) -> &'static str {
        match self {
            Self::AdaptiveSimulatedAnnealing => "BPM Annealing (AdaptiveSimulatedAnnealing)",
            Self::BeatJump => "Beat Jump (BeatJump)",
            Self::LoopManager => "Loop Manager (LoopManager)",
            Self::AudioPeakLimiter => "Peak Limiter (AudioPeakLimiter)",
            Self::Delayline => "DSP Delay Line (Delayline)",
            Self::DebugServerProxy => "Remote Debugger (DebugServerProxy)",
            Self::PECommsAdapter => "PMEditor Adapter (PECommsAdapter)",
            Self::SincInterpolator => "Sinc Resampling (SincInterpolator)",
            Self::LargeAACDecoder => "Advanced AAC Decoder (562KB)",
            Self::LargeMP3Decoder => "Advanced MP3 Decoder (150KB)",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FirmwareProfile {
    pub version: FirmwareVersion,
    pub tag: &'static str,
    pub fw_code: &'static str,
    pub branch: &'static str,
    pub pmplayer_size: u64,
    pub aac_decoder_size: u64,
    pub mp3_decoder_size: u64,
    pub features: &'static [FirmwareFeature],
    pub description: &'static str,
}

pub static ALL_FIRMWARE_PROFILES: &[FirmwareProfile] = &[
    FirmwareProfile {
        version: FirmwareVersion::V13557,
        tag: "13557",
        fw_code: "0513557",
        branch: "Standard (Early)",
        pmplayer_size: 11_026_232,
        aac_decoder_size: 64_728,
        mp3_decoder_size: 50_236,
        features: &[],
        description: "初期ベースライン版。Freeverb系DSP、基本DJ再生・クロスフェード。",
    },
    FirmwareProfile {
        version: FirmwareVersion::V14108,
        tag: "14108",
        fw_code: "0514108",
        branch: "Standard",
        pmplayer_size: 11_042_676,
        aac_decoder_size: 65_246,
        mp3_decoder_size: 51_018,
        features: &[FirmwareFeature::AdaptiveSimulatedAnnealing],
        description: "適応型アニーリングBPM解析アルゴリズムを新搭載。AACデコーダー安定化。",
    },
    FirmwareProfile {
        version: FirmwareVersion::V14142,
        tag: "14142",
        fw_code: "0514142",
        branch: "Standard (Reference)",
        pmplayer_size: 11_145_356,
        aac_decoder_size: 65_246,
        mp3_decoder_size: 51_018,
        features: &[FirmwareFeature::AdaptiveSimulatedAnnealing],
        description: "Pacemaker 実機の公式標準リファレンスファームウェア（高安定・汎用性）。",
    },
    FirmwareProfile {
        version: FirmwareVersion::V16219,
        tag: "16219",
        fw_code: "0516219",
        branch: "Standard (Late)",
        pmplayer_size: 11_194_644,
        aac_decoder_size: 65_246,
        mp3_decoder_size: 51_018,
        features: &[
            FirmwareFeature::AdaptiveSimulatedAnnealing,
            FirmwareFeature::BeatJump,
            FirmwareFeature::LoopManager,
            FirmwareFeature::DebugServerProxy,
        ],
        description: "BeatJump、LoopManager、およびリモートデバッグプロキシを新設。",
    },
    FirmwareProfile {
        version: FirmwareVersion::V13606814,
        tag: "13606814",
        fw_code: "0813606814",
        branch: "Extended / Pro",
        pmplayer_size: 12_452_472,
        aac_decoder_size: 107_572,
        mp3_decoder_size: 150_835,
        features: &[
            FirmwareFeature::AdaptiveSimulatedAnnealing,
            FirmwareFeature::BeatJump,
            FirmwareFeature::LoopManager,
            FirmwareFeature::AudioPeakLimiter,
            FirmwareFeature::PECommsAdapter,
            FirmwareFeature::SincInterpolator,
            FirmwareFeature::LargeMP3Decoder,
        ],
        description: "拡張ブランチ。Sinc高品質補間、ピークリミッター、150KB MP3デコーダー、PMEditor直結通信。",
    },
    FirmwareProfile {
        version: FirmwareVersion::V146061537,
        tag: "146061537",
        fw_code: "09146061537",
        branch: "Extended / Pro",
        pmplayer_size: 12_208_584,
        aac_decoder_size: 107_650,
        mp3_decoder_size: 150_567,
        features: &[
            FirmwareFeature::AdaptiveSimulatedAnnealing,
            FirmwareFeature::BeatJump,
            FirmwareFeature::LoopManager,
            FirmwareFeature::Delayline,
            FirmwareFeature::PECommsAdapter,
            FirmwareFeature::SincInterpolator,
            FirmwareFeature::LargeMP3Decoder,
        ],
        description: "拡張ブランチ第2世代。専用DSP Delayline、Sinc補間、PMEditor通信を統合。",
    },
    FirmwareProfile {
        version: FirmwareVersion::V180096069,
        tag: "180096069",
        fw_code: "09180096069",
        branch: "Evolution / Final",
        pmplayer_size: 11_286_972,
        aac_decoder_size: 562_629,
        mp3_decoder_size: 52_145,
        features: &[
            FirmwareFeature::AdaptiveSimulatedAnnealing,
            FirmwareFeature::BeatJump,
            FirmwareFeature::LoopManager,
            FirmwareFeature::Delayline,
            FirmwareFeature::PECommsAdapter,
            FirmwareFeature::LargeAACDecoder,
        ],
        description: "最終進化版ファームウェア。562KB 高度AACデコーダーを新搭載し、主要拡張DSPを統合最適化。",
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareDiff {
    pub from: FirmwareVersion,
    pub to: FirmwareVersion,
    pub pmplayer_size_delta: i64,
    pub aac_decoder_size_delta: i64,
    pub mp3_decoder_size_delta: i64,
    pub added_features: Vec<FirmwareFeature>,
    pub removed_features: Vec<FirmwareFeature>,
}

pub fn compare_firmwares(from: FirmwareVersion, to: FirmwareVersion) -> FirmwareDiff {
    let p_from = from.profile();
    let p_to = to.profile();

    let added: Vec<FirmwareFeature> = p_to
        .features
        .iter()
        .copied()
        .filter(|f| !p_from.features.contains(f))
        .collect();

    let removed: Vec<FirmwareFeature> = p_from
        .features
        .iter()
        .copied()
        .filter(|f| !p_to.features.contains(f))
        .collect();

    FirmwareDiff {
        from,
        to,
        pmplayer_size_delta: (p_to.pmplayer_size as i64) - (p_from.pmplayer_size as i64),
        aac_decoder_size_delta: (p_to.aac_decoder_size as i64) - (p_from.aac_decoder_size as i64),
        mp3_decoder_size_delta: (p_to.mp3_decoder_size as i64) - (p_from.mp3_decoder_size as i64),
        added_features: added,
        removed_features: removed,
    }
}

/// Device discovery across macOS, Windows, and Linux
pub fn detect_pacemaker_device() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let volumes = Path::new("/Volumes");
        if volumes.exists() {
            if let Ok(entries) = fs::read_dir(volumes) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name_lower = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_ascii_lowercase())
                        .unwrap_or_default();

                    // Skip internal system volume and hidden dirs
                    if name_lower == "macintosh hd" || name_lower.starts_with('.') {
                        continue;
                    }

                    if name_lower == "pacemaker"
                        || path.join(".Pacemaker/music.db").exists()
                        || path.join(".Pacemaker/system.xml").exists()
                        || path.join(".Pacemaker").is_dir()
                        || path.join("system.xml").exists()
                        || (name_lower.contains("pacemaker") && path.join("music.db").exists())
                    {
                        return Some(path);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Check Windows drive letters D: through Z:
        for drive in b'D'..=b'Z' {
            let drive_path = format!("{}:\\", drive as char);
            let path = PathBuf::from(&drive_path);
            if path.join(".Pacemaker\\system.xml").exists()
                || path.join(".Pacemaker\\music.db").exists()
                || path.join("system.xml").exists()
            {
                return Some(path);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        for base in &["/media", "/mnt", "/run/media"] {
            let base_path = Path::new(base);
            if base_path.exists() {
                if let Ok(entries) = fs::read_dir(base_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.join(".Pacemaker/system.xml").exists()
                            || path.join(".Pacemaker/music.db").exists()
                        {
                            return Some(path);
                        }
                        // Check subdirectories (e.g. /media/$USER/Pacemaker)
                        if let Ok(sub_entries) = fs::read_dir(&path) {
                            for sub in sub_entries.flatten() {
                                let sub_path = sub.path();
                                if sub_path.join(".Pacemaker/system.xml").exists()
                                    || sub_path.join(".Pacemaker/music.db").exists()
                                {
                                    return Some(sub_path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
