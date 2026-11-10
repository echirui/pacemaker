//! Parser for Pacemaker's proprietary waveform and beat-grid `.str` files (`TrackAnalysedData`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackAnalysedData {
    pub normalize_factor: f32,
    pub beat_start_offset: i64,
    pub real_beat_locations_raw: String,
    pub beat_count: usize,
}

impl TrackAnalysedData {
    /// Parses a `.str` XML file content
    pub fn parse_xml(content: &str) -> Result<Self, String> {
        let extract = |tag: &str| -> Option<String> {
            let open = format!("<{}>", tag);
            let close = format!("</{}>", tag);
            let start = content.find(&open)? + open.len();
            let end = content[start..].find(&close)? + start;
            Some(content[start..end].trim().to_string())
        };

        let norm_str = extract("normalizeFactor").unwrap_or_else(|| "1.0".to_string());
        let offset_str = extract("beatStartOffset").unwrap_or_else(|| "0".to_string());
        let beats_str = extract("realBeatLocations").unwrap_or_default();

        let normalize_factor = norm_str.parse::<f32>().map_err(|e| e.to_string())?;
        let beat_start_offset = offset_str.parse::<i64>().map_err(|e| e.to_string())?;

        let beat_count = if let Some(dot_idx) = beats_str.find('.') {
            beats_str[..dot_idx].parse::<usize>().unwrap_or(0)
        } else {
            0
        };

        Ok(TrackAnalysedData {
            normalize_factor,
            beat_start_offset,
            real_beat_locations_raw: beats_str,
            beat_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_analysed_data() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<TrackAnalysedData>
	<normalizeFactor>1</normalizeFactor>
	<beatStartOffset>67169</beatStartOffset>
	<realBeatLocations>4736.3wYcb8symcSC79Jw</realBeatLocations>
</TrackAnalysedData>"#;

        let res = TrackAnalysedData::parse_xml(xml).expect("Failed to parse");
        assert_eq!(res.normalize_factor, 1.0);
        assert_eq!(res.beat_start_offset, 67169);
        assert_eq!(res.beat_count, 4736);
    }
}
