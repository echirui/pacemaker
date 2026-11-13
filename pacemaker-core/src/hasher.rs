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

/// Computes MD5 128-bit digest (RFC 1321) in pure Rust
pub fn md5_digest(data: &[u8]) -> [u8; 16] {
    let mut a: u32 = 0x67452301;
    let mut b: u32 = 0xefcdab89;
    let mut c: u32 = 0x98badcfe;
    let mut d: u32 = 0x10325476;

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
        0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
        0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
        0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
        0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
        0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
        0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
        0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
        0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
        0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
        0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
        0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
        0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
    ];

    const S: [u32; 64] = [
        7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,
        5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,
        4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,
        6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,
    ];

    for chunk in msg.chunks_exact(64) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes(chunk[i*4..(i+1)*4].try_into().unwrap());
        }

        let mut aa = a;
        let mut bb = b;
        let mut cc = c;
        let mut dd = d;

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((bb & cc) | (!bb & dd), i),
                16..=31 => ((dd & bb) | (!dd & cc), (5 * i + 1) % 16),
                32..=47 => (bb ^ cc ^ dd, (3 * i + 5) % 16),
                _ => (cc ^ (bb | !dd), (7 * i) % 16),
            };

            let temp = dd;
            dd = cc;
            cc = bb;
            bb = bb.wrapping_add((aa.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g])).rotate_left(S[i]));
            aa = temp;
        }

        a = a.wrapping_add(aa);
        b = b.wrapping_add(bb);
        c = c.wrapping_add(cc);
        d = d.wrapping_add(dd);
    }

    let mut result = [0u8; 16];
    result[0..4].copy_from_slice(&a.to_le_bytes());
    result[4..8].copy_from_slice(&b.to_le_bytes());
    result[8..12].copy_from_slice(&c.to_le_bytes());
    result[12..16].copy_from_slice(&d.to_le_bytes());
    result
}

/// Generates a 32-hex string MD5 hash
pub fn md5_hex(data: &[u8]) -> String {
    let digest = md5_digest(data);
    let mut hex = String::with_capacity(32);
    for b in digest {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

/// Generates a 32-hex character `global_id` matching Tonium Pacemaker database format
pub fn generate_global_id(title: &str, artist: &str, file_size: u64) -> String {
    let raw = format!("{}:{}:{}", title.trim().to_lowercase(), artist.trim().to_lowercase(), file_size);
    md5_hex(raw.as_bytes())
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

    #[test]
    fn test_md5() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(
            md5_hex(b"The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
        let gid = generate_global_id("Test Title", "Test Artist", 123456);
        assert_eq!(gid.len(), 32);
    }
}
