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
