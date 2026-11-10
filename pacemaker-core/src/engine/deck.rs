//! DJ Deck state machine corresponding to `audiocore::AudioClipCollection` & `AudioClip`.

use crate::types::{DeckId, PlaybackState, TimeMs, TrackMetadata};
use crate::engine::loop_engine::AudioLoop;
use crate::dsp::ParametricEq;

#[derive(Debug, Clone)]
pub struct Deck {
    pub id: DeckId,
    pub state: PlaybackState,
    pub current_track: Option<TrackMetadata>,
    pub current_time_ms: TimeMs,
    pub cue_point_ms: Option<TimeMs>,
    pub audio_loop: AudioLoop,
    pub eq: ParametricEq,

    // Pitch & Tempo Controls
    pub play_speed: f32, // 1.0 = normal, 1.05 = +5%
    pub key_offset_semitones: f32,
    pub is_key_locked: bool,
}

impl Deck {
    pub fn new(id: DeckId, sample_rate: f32) -> Self {
        Self {
            id,
            state: PlaybackState::Stopped,
            current_track: None,
            current_time_ms: 0,
            cue_point_ms: None,
            audio_loop: AudioLoop::new(),
            eq: ParametricEq::new(sample_rate),
            play_speed: 1.0,
            key_offset_semitones: 0.0,
            is_key_locked: true,
        }
    }

    pub fn load_track(&mut self, track: TrackMetadata) {
        self.cue_point_ms = if track.cue_point_ms > 0 { Some(track.cue_point_ms) } else { None };
        self.current_time_ms = self.cue_point_ms.unwrap_or(0);
        self.audio_loop = AudioLoop::new();

        if track.loop_in_ms >= 0 && track.loop_out_ms > track.loop_in_ms {
            self.audio_loop.set_loop_in(track.loop_in_ms);
            self.audio_loop.set_loop_out(track.loop_out_ms);
        }

        self.current_track = Some(track);
        self.state = PlaybackState::Paused;
    }

    pub fn play(&mut self) {
        if self.current_track.is_some() {
            self.state = PlaybackState::Playing;
        }
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.current_time_ms = self.cue_point_ms.unwrap_or(0);
    }

    pub fn set_cue(&mut self) {
        self.cue_point_ms = Some(self.current_time_ms);
    }

    pub fn jump_to_cue(&mut self) {
        self.current_time_ms = self.cue_point_ms.unwrap_or(0);
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    pub fn set_speed(&mut self, factor: f32) {
        self.play_speed = factor.clamp(0.5, 2.0);
    }

    /// Advances the playhead by `delta_samples` at the given sample rate
    pub fn advance_samples(&mut self, samples: usize, sample_rate: f32) {
        if self.state != PlaybackState::Playing {
            return;
        }

        let delta_secs = (samples as f32 / sample_rate) * self.play_speed;
        let delta_ms = (delta_secs * 1000.0) as TimeMs;
        self.current_time_ms += delta_ms;

        // Apply looping check
        self.current_time_ms = self.audio_loop.wrap_time(self.current_time_ms);

        // Check end of track
        if let Some(ref track) = self.current_track {
            let max_ms = (track.play_time_secs as i64) * 1000;
            if self.current_time_ms >= max_ms {
                self.current_time_ms = max_ms;
                self.state = PlaybackState::Stopped;
            }
        }
    }
}
