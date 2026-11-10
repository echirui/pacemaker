//! Audio Peak Limiter DSP module.
//!
//! Reconstructed from Tonium Pacemaker firmware `13606814` (`AudioPeakLimiter`).
//! Provides transparent master limiting to prevent digital clipping (0 dBFS saturation)
//! during dual-deck summing and aggressive EQ boosts.

#[derive(Debug, Clone)]
pub struct AudioPeakLimiter {
    threshold: f32,
    release_coeff: f32,
    envelope: f32,
}

impl AudioPeakLimiter {
    pub fn new(sample_rate: f32, threshold_db: f32, _attack_ms: f32, release_ms: f32) -> Self {
        let threshold = 10.0f32.powf(threshold_db / 20.0);
        let release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();

        Self {
            threshold,
            release_coeff,
            envelope: 0.0,
        }
    }

    /// Default configuration for Pacemaker master output
    pub fn default_master(sample_rate: f32) -> Self {
        Self::new(sample_rate, -0.3, 1.5, 60.0)
    }

    /// Process in-place stereo or mono audio samples
    pub fn process_interleaved(&mut self, samples: &mut [f32], channels: usize) {
        if channels == 0 {
            return;
        }

        let frames = samples.len() / channels;
        for i in 0..frames {
            let offset = i * channels;
            let mut peak = 0.0f32;
            for ch in 0..channels {
                let abs_s = samples[offset + ch].abs();
                if abs_s > peak {
                    peak = abs_s;
                }
            }

            // Peak limiter: instant attack for hard ceiling protection, exponential release
            if peak > self.envelope {
                self.envelope = peak;
            } else {
                self.envelope = self.release_coeff * self.envelope + (1.0 - self.release_coeff) * peak;
            }

            // Soft-knee limiting gain
            let gain = if self.envelope > self.threshold {
                self.threshold / self.envelope
            } else {
                1.0
            };

            for ch in 0..channels {
                samples[offset + ch] *= gain;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peak_limiter_attenuation() {
        let mut limiter = AudioPeakLimiter::default_master(44100.0);
        // Feed loud signal exceeding 0 dBFS (amplitude 2.5)
        let mut loud_samples = vec![2.5f32; 1024];
        limiter.process_interleaved(&mut loud_samples, 2);

        // Peak must be clamped towards threshold (~0.96)
        let max_out = loud_samples.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(max_out < 1.1, "Limiter must prevent loud signals from clipping");
    }
}
