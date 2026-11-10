//! 2-Channel DJ Mixer coordinating Deck A, Deck B, crossfader, EQs, and master output buffer.

use crate::types::DeckId;
use crate::engine::deck::Deck;
use crate::engine::events::AudioCoreEvent;
use crate::dsp::{AudioProcessBuffer, CompoundCrossfader};

pub struct PacemakerMixer {
    pub deck_a: Deck,
    pub deck_b: Deck,
    pub crossfader: CompoundCrossfader,
    pub master_volume: f32,
    pub headphone_volume: f32,
    pub sample_rate: f32,
}

impl PacemakerMixer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            deck_a: Deck::new(DeckId::DeckA, sample_rate),
            deck_b: Deck::new(DeckId::DeckB, sample_rate),
            crossfader: CompoundCrossfader::new(),
            master_volume: 1.0,
            headphone_volume: 0.8,
            sample_rate,
        }
    }

    pub fn deck(&self, id: DeckId) -> &Deck {
        match id {
            DeckId::DeckA => &self.deck_a,
            DeckId::DeckB => &self.deck_b,
        }
    }

    pub fn deck_mut(&mut self, id: DeckId) -> &mut Deck {
        match id {
            DeckId::DeckA => &mut self.deck_a,
            DeckId::DeckB => &mut self.deck_b,
        }
    }

    pub fn set_crossfader(&mut self, norm_position: f32) {
        // Map [0.0, 1.0] to [-1.0, +1.0] for internal CompoundCrossfader
        self.crossfader.position = (norm_position.clamp(0.0, 1.0) * 2.0) - 1.0;
    }

    pub fn crossfader_gains(&self) -> (f32, f32) {
        self.crossfader.calculate_gains()
    }

    pub fn handle_event(&mut self, event: AudioCoreEvent) {
        match event {
            AudioCoreEvent::LoadTrack { deck: _, track_id: _ } => {
                // Delegated with full metadata
            }
            AudioCoreEvent::PlayTrack { deck } => match deck {
                DeckId::DeckA => self.deck_a.play(),
                DeckId::DeckB => self.deck_b.play(),
            },
            AudioCoreEvent::PauseTrack { deck } => match deck {
                DeckId::DeckA => self.deck_a.pause(),
                DeckId::DeckB => self.deck_b.pause(),
            },
            AudioCoreEvent::SetCuePoint { deck, time_ms } => {
                let d = match deck {
                    DeckId::DeckA => &mut self.deck_a,
                    DeckId::DeckB => &mut self.deck_b,
                };
                d.current_time_ms = time_ms;
                d.set_cue();
            }
            AudioCoreEvent::JumpToCue { deck } => match deck {
                DeckId::DeckA => self.deck_a.jump_to_cue(),
                DeckId::DeckB => self.deck_b.jump_to_cue(),
            },
            AudioCoreEvent::SetLoopInPosition { deck, time_ms } => match deck {
                DeckId::DeckA => self.deck_a.audio_loop.set_loop_in(time_ms),
                DeckId::DeckB => self.deck_b.audio_loop.set_loop_in(time_ms),
            },
            AudioCoreEvent::SetLoopEndPosition { deck, time_ms } => match deck {
                DeckId::DeckA => self.deck_a.audio_loop.set_loop_out(time_ms),
                DeckId::DeckB => self.deck_b.audio_loop.set_loop_out(time_ms),
            },
            AudioCoreEvent::SplitLoop { deck } => match deck {
                DeckId::DeckA => self.deck_a.audio_loop.split_loop(),
                DeckId::DeckB => self.deck_b.audio_loop.split_loop(),
            },
            AudioCoreEvent::SetPlaySpeed { deck, speed_factor } => match deck {
                DeckId::DeckA => self.deck_a.set_speed(speed_factor),
                DeckId::DeckB => self.deck_b.set_speed(speed_factor),
            },
            AudioCoreEvent::SetKeyOffset { deck, semitones } => match deck {
                DeckId::DeckA => self.deck_a.key_offset_semitones = semitones,
                DeckId::DeckB => self.deck_b.key_offset_semitones = semitones,
            },
            AudioCoreEvent::SetCrossfaderPosition { position } => {
                self.crossfader.position = position;
            }
            AudioCoreEvent::SetCrossfaderCurve { curve } => {
                self.crossfader.curve = curve;
            }
        }
    }

    /// Renders a block of master stereo audio mixing Deck A and Deck B through Crossfader
    pub fn render_block(
        &mut self,
        buf_deck_a: &mut AudioProcessBuffer,
        buf_deck_b: &mut AudioProcessBuffer,
        out_master: &mut AudioProcessBuffer,
    ) {
        let sample_count = out_master.samples_per_channel;

        // Apply 3-Band Parametric EQ to each deck
        self.deck_a.eq.process(buf_deck_a);
        self.deck_b.eq.process(buf_deck_b);

        // Calculate crossfader gains
        let (gain_a, gain_b) = self.crossfader.calculate_gains();

        // Mix into master buffer
        out_master.clear();
        out_master.add_from(buf_deck_a, gain_a * self.master_volume);
        out_master.add_from(buf_deck_b, gain_b * self.master_volume);

        // Advance deck playheads
        self.deck_a.advance_samples(sample_count, self.sample_rate);
        self.deck_b.advance_samples(sample_count, self.sample_rate);
    }
}
