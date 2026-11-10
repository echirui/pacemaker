use serde::{Deserialize, Serialize};

pub type TrackId = i64;
pub type TimeMs = i64;
pub type Bpm = f64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    CuePreview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeckId {
    DeckA,
    DeckB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub id: TrackId,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: u32,
    pub bpm: Bpm,
    pub play_time_secs: u32,
    pub cue_point_ms: TimeMs,
    pub loop_in_ms: TimeMs,
    pub loop_out_ms: TimeMs,
    pub virtual_location: String,
    pub key: String,
    pub rating: u8,
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseItem {
    pub case_id: i64,
    pub name: String,
    pub date_created: i64,
    pub date_modified: i64,
}
