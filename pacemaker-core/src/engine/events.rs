//! Event types reverse-engineered from `audiocore` in Tonium Pacemaker firmware.

use crate::types::{DeckId, TimeMs, TrackId};
use crate::dsp::CrossfaderCurve;

#[derive(Debug, Clone)]
pub enum AudioCoreEvent {
    /// Corresponds to `audiocore::LoadTrackEvent`
    LoadTrack {
        deck: DeckId,
        track_id: TrackId,
    },
    /// Corresponds to `audiocore::PlayTrackEvent`
    PlayTrack {
        deck: DeckId,
    },
    PauseTrack {
        deck: DeckId,
    },
    /// Corresponds to `audiocore::SetCuePointEvent`
    SetCuePoint {
        deck: DeckId,
        time_ms: TimeMs,
    },
    JumpToCue {
        deck: DeckId,
    },
    /// Corresponds to `audiocore::SetLoopInPositionEvent`
    SetLoopInPosition {
        deck: DeckId,
        time_ms: TimeMs,
    },
    /// Corresponds to `audiocore::SetLoopEndPositionEvent`
    SetLoopEndPosition {
        deck: DeckId,
        time_ms: TimeMs,
    },
    /// Corresponds to `audiocore::SplitLoopEvent`
    SplitLoop {
        deck: DeckId,
    },
    /// Corresponds to `audiocore::SetPlaySpeedEvent`
    SetPlaySpeed {
        deck: DeckId,
        speed_factor: f32, // 1.0 = normal, 1.08 = +8% pitch
    },
    /// Corresponds to `audiocore::SetKeyOffsetEvent`
    SetKeyOffset {
        deck: DeckId,
        semitones: f32,
    },
    /// Corresponds to `audiocore::CrossfaderCurveEvent`
    SetCrossfaderPosition {
        position: f32, // -1.0 to +1.0
    },
    SetCrossfaderCurve {
        curve: CrossfaderCurve,
    },
}
