//! Safe audio metadata and ID3 container parser for Pacemaker tracks.
//!
//! Replaces the legacy C++ `libcontainer_id3.so` which suffered from
//! `std::length_error` crashes when encountering malformed ID3 frames (Issue #4).
//! Enforces strict buffer bounds, safe Unicode transcoding, and graceful recovery.

use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ParsedMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub comments: Option<String>,
    pub duration_secs: Option<u32>,
}

/// Reads ID3v2 tags with bounds-checking to prevent length overflow crashes
pub fn read_id3_safely<P: AsRef<Path>>(path: P) -> Result<ParsedMetadata, String> {
    let mut file = File::open(&path).map_err(|e| format!("Cannot open file: {}", e))?;
    let file_size = file.metadata().map_err(|e| e.to_string())?.len();

    if file_size < 10 {
        return Ok(ParsedMetadata::default());
    }

    let mut header = [0u8; 10];
    file.read_exact(&mut header).map_err(|e| e.to_string())?;

    // Check "ID3" identifier
    if &header[0..3] != b"ID3" {
        return Ok(ParsedMetadata::default());
    }

    let major_version = header[3];
    if major_version < 2 || major_version > 4 {
        return Ok(ParsedMetadata::default());
    }

    // Synchsafe integer decoding for ID3 header size
    let tag_size = ((header[6] as usize & 0x7F) << 21)
        | ((header[7] as usize & 0x7F) << 14)
        | ((header[8] as usize & 0x7F) << 7)
        | (header[9] as usize & 0x7F);

    // Guard against malformed massive tag size
    let safe_tag_size = tag_size.min((file_size.saturating_sub(10)) as usize).min(10 * 1024 * 1024);
    let mut tag_buffer = vec![0u8; safe_tag_size];
    if file.read_exact(&mut tag_buffer).is_err() {
        return Ok(ParsedMetadata::default());
    }

    let mut meta = ParsedMetadata::default();
    let mut cursor = 0;

    // Parse ID3v2.3 and ID3v2.4 frames
    while cursor + 10 <= tag_buffer.len() {
        let frame_id = &tag_buffer[cursor..cursor + 4];
        if frame_id[0] == 0 {
            // Padding reached
            break;
        }

        let frame_size = if major_version == 4 {
            // Synchsafe int in v2.4
            ((tag_buffer[cursor + 4] as usize & 0x7F) << 21)
                | ((tag_buffer[cursor + 5] as usize & 0x7F) << 14)
                | ((tag_buffer[cursor + 6] as usize & 0x7F) << 7)
                | (tag_buffer[cursor + 7] as usize & 0x7F)
        } else {
            // Standard big-endian uint32 in v2.3
            ((tag_buffer[cursor + 4] as usize) << 24)
                | ((tag_buffer[cursor + 5] as usize) << 16)
                | ((tag_buffer[cursor + 6] as usize) << 8)
                | (tag_buffer[cursor + 7] as usize)
        };

        let frame_header_size = 10;
        let data_start = cursor + frame_header_size;
        let data_end = data_start + frame_size;

        // VITAL GUARD (Issue #4): Reject stated lengths that exceed remaining tag buffer!
        // Prevents C++ std::length_error crash observed in Pacemaker firmware 14142.
        if data_end > tag_buffer.len() {
            // Truncate or abort corrupted frame safely
            break;
        }

        let frame_data = &tag_buffer[data_start..data_end];
        let id_str = String::from_utf8_lossy(frame_id);

        match id_str.as_ref() {
            "TIT2" => meta.title = decode_text_frame(frame_data),
            "TPE1" => meta.artist = decode_text_frame(frame_data),
            "TALB" => meta.album = decode_text_frame(frame_data),
            "TCON" => meta.genre = decode_text_frame(frame_data),
            "TYER" | "TDRC" => {
                if let Some(s) = decode_text_frame(frame_data) {
                    if let Ok(y) = s.chars().filter(|c| c.is_ascii_digit()).take(4).collect::<String>().parse::<u32>() {
                        meta.year = Some(y);
                    }
                }
            }
            "TBPM" => {
                if let Some(s) = decode_text_frame(frame_data) {
                    if let Ok(b) = s.trim().parse::<f64>() {
                        meta.bpm = Some(b);
                    }
                }
            }
            "TKEY" => meta.key = decode_text_frame(frame_data),
            "COMM" => {
                // Comments frame: encoding(1) + lang(3) + short desc(0-term) + text
                if frame_data.len() > 4 {
                    meta.comments = decode_text_frame(&frame_data[4..]);
                }
            }
            _ => {}
        }

        cursor = data_end;
    }

    Ok(meta)
}

fn decode_text_frame(data: &[u8]) -> Option<String> {
    if data.is_empty() {
        return None;
    }
    let encoding = data[0];
    let content = &data[1..];
    if content.is_empty() {
        return None;
    }

    match encoding {
        0 => {
            // ISO-8859-1 (Latin-1)
            let s: String = content.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        1 => {
            // UTF-16 with BOM
            if content.len() < 2 {
                return None;
            }
            let is_be = content[0] == 0xFE && content[1] == 0xFF;
            let slice = &content[2..];
            let u16s: Vec<u16> = slice
                .chunks_exact(2)
                .map(|chunk| {
                    if is_be {
                        u16::from_be_bytes([chunk[0], chunk[1]])
                    } else {
                        u16::from_le_bytes([chunk[0], chunk[1]])
                    }
                })
                .take_while(|&u| u != 0)
                .collect();
            String::from_utf16(&u16s).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }
        2 => {
            // UTF-16BE without BOM
            let u16s: Vec<u16> = content
                .chunks_exact(2)
                .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                .take_while(|&u| u != 0)
                .collect();
            String::from_utf16(&u16s).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }
        3 => {
            // UTF-8
            String::from_utf8(content.to_vec())
                .ok()
                .map(|s| s.trim_matches('\0').trim().to_string())
                .filter(|s| !s.is_empty())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corrupted_id3_bounds_safety() {
        // Construct malformed ID3 header with stated frame length exceeding buffer
        let malformed = vec![
            b'I', b'D', b'3', 3, 0, 0, // ID3v2.3
            0, 0, 0, 30, // Tag size: 30 bytes
            // TIT2 frame with stated length of 0x1000 (exceeds buffer!)
            b'T', b'I', b'T', b'2',
            0x00, 0x00, 0x10, 0x00, // 4096 bytes stated
            0x00, 0x00, // flags
            0x00, // encoding: Latin-1
            b'H', b'e', b'l', b'l', b'o',
        ];

        let tmp_path = std::env::temp_dir().join("pacemaker_test_corrupt.id3");
        std::fs::write(&tmp_path, &malformed).unwrap();

        // Must not crash with length_error or panic
        let res = read_id3_safely(&tmp_path);
        assert!(res.is_ok(), "Must handle corrupted frame bounds safely");
        let _ = std::fs::remove_file(tmp_path);
    }
}
