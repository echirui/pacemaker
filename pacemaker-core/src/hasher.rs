//! Path and Hash generator reverse-engineered from `osal::getEncodedFilePath` in Tonium Pacemaker firmware.

/// Jenkins one-at-a-time hash (Bob Jenkins 1997)
pub fn jenkins_hash(s: &str) -> u32 {
    let mut hash: u32 = 0;
    for byte in s.bytes() {
        hash = hash.wrapping_add(byte as u32);
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
    }
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15);
    hash
}

/// PJW / ELF hash function as implemented in JUCE / Pacemaker
pub fn pjw_hash(s: &str) -> u32 {
    let mut h: u32 = 0;
    for byte in s.bytes() {
        h = (h << 4).wrapping_add(byte as u32);
        let g = h & 0xf0000000;
        if g != 0 {
            h ^= g >> 24;
        }
        h &= !g;
    }
    h
}

/// Structure representing track paths in Pacemaker filesystem
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackPaths {
    pub virtual_location: String,
    pub relative_music_path: String,
    pub dir_char1: char,
    pub dir_char2: char,
    pub file_hash: String,
}

/// Generates the two-level directory and filename hash matching Pacemaker firmware
pub fn generate_track_paths(filename_without_ext: &str, file_size: u64, ext: &str) -> TrackPaths {
    let size_str = file_size.to_string();
    let j_hash = jenkins_hash(&size_str);
    let p_hash = pjw_hash(&size_str);
    let f_hash = jenkins_hash(filename_without_ext);

    let j_hex = format!("{:x}", j_hash);
    let p_hex = format!("{:x}", p_hash);
    let file_hex = format!("{:08x}", f_hash);

    let c1 = j_hex.chars().last().unwrap_or('0');
    let c2 = p_hex.chars().last().unwrap_or('0');

    let clean_ext = if ext.starts_with('.') { &ext[1..] } else { ext };
    let filename = format!("{}.{}", file_hex, clean_ext);
    let virtual_location = format!("/pmdb_tracks/{}/{}/{}", c1, c2, filename);
    let relative_music_path = format!(".Pacemaker/Music/{}/{}/{}", c1, c2, filename);

    TrackPaths {
        virtual_location,
        relative_music_path,
        dir_char1: c1,
        dir_char2: c2,
        file_hash: file_hex,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jenkins_hash() {
        assert_eq!(jenkins_hash("4519453"), 0x0fc6b80e);
    }

    #[test]
    fn test_pjw_hash() {
        assert_eq!(pjw_hash("4519453"), 0x0784c7b3);
    }
}
