//! Audio Loop Engine corresponding to `audiocore::AudioLoop` and `LoopAudioProcess`.

use crate::types::TimeMs;

#[derive(Debug, Clone)]
pub struct AudioLoop {
    pub loop_in_ms: Option<TimeMs>,
    pub loop_out_ms: Option<TimeMs>,
    pub enabled: bool,
    pub is_reversed: bool,
}

impl AudioLoop {
    pub fn new() -> Self {
        Self {
            loop_in_ms: None,
            loop_out_ms: None,
            enabled: false,
            is_reversed: false,
        }
    }

    pub fn set_loop_in(&mut self, time_ms: TimeMs) {
        self.loop_in_ms = Some(time_ms);
        if let Some(out) = self.loop_out_ms {
            if out > time_ms {
                self.enabled = true;
            }
        }
    }

    pub fn set_loop_out(&mut self, time_ms: TimeMs) {
        if let Some(in_time) = self.loop_in_ms {
            if time_ms > in_time {
                self.loop_out_ms = Some(time_ms);
                self.enabled = true;
            }
        }
    }

    pub fn toggle(&mut self) -> bool {
        if self.is_valid() {
            self.enabled = !self.enabled;
        }
        self.enabled
    }

    pub fn is_valid(&self) -> bool {
        match (self.loop_in_ms, self.loop_out_ms) {
            (Some(start), Some(end)) => end > start,
            _ => false,
        }
    }

    pub fn duration_ms(&self) -> Option<TimeMs> {
        match (self.loop_in_ms, self.loop_out_ms) {
            (Some(start), Some(end)) if end > start => Some(end - start),
            _ => None,
        }
    }

    /// Split loop in half (corresponding to `audiocore::SplitLoopEvent`)
    pub fn split_loop(&mut self) {
        if let (Some(start), Some(end)) = (self.loop_in_ms, self.loop_out_ms) {
            let half = (end - start) / 2;
            if half >= 10 { // Minimum 10ms loop
                self.loop_out_ms = Some(start + half);
            }
        }
    }

    /// Double loop length (2x)
    pub fn double_loop(&mut self) {
        if let (Some(start), Some(end)) = (self.loop_in_ms, self.loop_out_ms) {
            let double = (end - start) * 2;
            self.loop_out_ms = Some(start + double);
        }
    }

    /// Check if current playhead exceeds loop out and return loop wrap position
    pub fn wrap_time(&self, current_time_ms: TimeMs) -> TimeMs {
        if self.enabled {
            if let (Some(start), Some(end)) = (self.loop_in_ms, self.loop_out_ms) {
                if current_time_ms >= end {
                    return start;
                }
            }
        }
        current_time_ms
    }
}

/// Realtime quantized BeatJump processor.
///
/// Reconstructed from Tonium Pacemaker firmware `13606814` (`BeatJump`).
/// Calculates quantized forward/backward beat displacements without drift.
#[derive(Debug, Clone)]
pub struct BeatJump;

impl BeatJump {
    /// Calculate target jump position in milliseconds.
    ///
    /// * `current_ms` - Current playhead position
    /// * `bpm` - Beats per minute of current track
    /// * `beats` - Number of beats to jump (positive = forward, negative = backward)
    /// * `total_duration_ms` - Track duration limit to prevent jumping beyond track bounds
    pub fn calculate_jump(
        current_ms: TimeMs,
        bpm: f64,
        beats: f64,
        total_duration_ms: TimeMs,
    ) -> Result<TimeMs, &'static str> {
        if bpm <= 20.0 || bpm > 350.0 {
            return Err("Invalid BPM for beat jump");
        }

        let ms_per_beat = 60_000.0 / bpm;
        let delta_ms = (beats * ms_per_beat).round() as TimeMs;
        let target = current_ms + delta_ms;

        // Strict clamp to prevent internal error / negative wrap (Issue #3)
        let clamped = target.max(0).min(total_duration_ms);
        Ok(clamped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_jump_calculations() {
        // At 120 BPM, 1 beat = 500 ms
        let pos = 10_000;
        let jump_4_beats = BeatJump::calculate_jump(pos, 120.0, 4.0, 60_000).unwrap();
        assert_eq!(jump_4_beats, 12_000); // 10000 + 2000 ms

        let jump_back_2 = BeatJump::calculate_jump(pos, 120.0, -2.0, 60_000).unwrap();
        assert_eq!(jump_back_2, 9_000); // 10000 - 1000 ms

        // Out of bounds clamp
        let jump_underflow = BeatJump::calculate_jump(500, 120.0, -4.0, 60_000).unwrap();
        assert_eq!(jump_underflow, 0);
    }
}
