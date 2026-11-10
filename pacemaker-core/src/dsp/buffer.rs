//! Audio Process Buffer corresponding to `xponaut::audioengine::AudioProcessBuffer`.

#[derive(Debug, Clone)]
pub struct AudioProcessBuffer {
    pub channels: usize,
    pub samples_per_channel: usize,
    pub data: Vec<Vec<f32>>,
}

impl AudioProcessBuffer {
    pub fn new(channels: usize, samples: usize) -> Self {
        Self {
            channels,
            samples_per_channel: samples,
            data: vec![vec![0.0; samples]; channels],
        }
    }

    pub fn clear(&mut self) {
        for ch in &mut self.data {
            ch.fill(0.0);
        }
    }

    /// Add from another buffer with gain scaling (matching `addFrom`)
    pub fn add_from(&mut self, other: &AudioProcessBuffer, gain: f32) {
        let ch_count = self.channels.min(other.channels);
        let sample_count = self.samples_per_channel.min(other.samples_per_channel);

        for c in 0..ch_count {
            let dest = &mut self.data[c];
            let src = &other.data[c];
            for i in 0..sample_count {
                dest[i] += src[i] * gain;
            }
        }
    }

    /// Apply gain in-place
    pub fn apply_gain(&mut self, gain: f32) {
        for ch in &mut self.data {
            for sample in ch {
                *sample *= gain;
            }
        }
    }
}
