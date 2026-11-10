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

/// Device discovery across macOS, Windows, and Linux
pub fn detect_pacemaker_device() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let volumes = Path::new("/Volumes");
        if volumes.exists() {
            if let Ok(entries) = fs::read_dir(volumes) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.join(".Pacemaker/system.xml").exists() || path.join("system.xml").exists() {
                        return Some(path);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Check Windows drive letters A: through Z:
        for drive in b'D'..=b'Z' {
            let drive_path = format!("{}:\\", drive as char);
            let path = PathBuf::from(&drive_path);
            if path.join(".Pacemaker\\system.xml").exists() || path.join("system.xml").exists() {
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
                        if path.join(".Pacemaker/system.xml").exists() {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }

    None
}
