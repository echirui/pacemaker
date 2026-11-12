use serde::{Deserialize, Serialize};

pub type TrackId = i64;
pub type TimeMs = i64;
pub type Bpm = f64;
pub type PlaylistId = i64;

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

impl DeckId {
    #[inline]
    pub fn opposite(&self) -> Self {
        match self {
            DeckId::DeckA => DeckId::DeckB,
            DeckId::DeckB => DeckId::DeckA,
        }
    }
}

/// Playlist repeat mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatMode {
    Off,
    One,
    All,
}

/// Playlist playback order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackOrder {
    Sequential,
    Shuffle,
}

/// DJ Deck transition type for Auto-Mix / Playlist continuous playback
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TransitionType {
    Cut,
    Crossfade { duration_secs: f32 },
    BeatMatch { duration_secs: f32 },
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

impl PartialEq for TrackMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.title == other.title
            && self.artist == other.artist
            && self.album == other.album
            && self.genre == other.genre
            && self.year == other.year
            && (self.bpm - other.bpm).abs() < 1e-4
            && self.play_time_secs == other.play_time_secs
            && self.cue_point_ms == other.cue_point_ms
            && self.loop_in_ms == other.loop_in_ms
            && self.loop_out_ms == other.loop_out_ms
            && self.virtual_location == other.virtual_location
            && self.key == other.key
            && self.rating == other.rating
            && self.comments == other.comments
    }
}

impl Default for TrackMetadata {
    fn default() -> Self {
        Self {
            id: 0,
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            genre: "Music".into(),
            year: 0,
            bpm: 120.0,
            play_time_secs: 0,
            cue_point_ms: -1,
            loop_in_ms: -1,
            loop_out_ms: -1,
            virtual_location: String::new(),
            key: String::new(),
            rating: 0,
            comments: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseItem {
    pub case_id: i64,
    pub name: String,
    pub date_created: i64,
    pub date_modified: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaylistError {
    EmptyPlaylist,
    IndexOutOfBounds(usize),
    TrackNotFound(TrackId),
    InvalidM3u(String),
    DatabaseError(String),
}

impl std::fmt::Display for PlaylistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPlaylist => write!(f, "Playlist is empty"),
            Self::IndexOutOfBounds(idx) => write!(f, "Playlist index out of bounds: {}", idx),
            Self::TrackNotFound(id) => write!(f, "Track with id {} not found in playlist", id),
            Self::InvalidM3u(msg) => write!(f, "Invalid M3U format: {}", msg),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for PlaylistError {}
