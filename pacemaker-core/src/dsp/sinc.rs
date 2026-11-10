//! Windowed Sinc Interpolator DSP module.
//!
//! Reconstructed from Tonium Pacemaker firmware `13606814` (`SincInterpolator`).
//! Employs a Blackman-windowed Sinc function for high-fidelity bandlimited resampling,
//! pitch-shifting, and tempo adjustment without aliasing artifacts.

use std::f32::consts::PI;

pub struct SincInterpolator {
    points: usize,
    table: Vec<f32>,
    phases: usize,
}

impl SincInterpolator {
    /// Creates a new windowed Sinc interpolator with precomputed polyphase lookup tables.
    /// Uses a single contiguous 1D buffer to minimize memory footprint and eliminate heap fragmentation.
    pub fn new(half_points: usize, phases: usize) -> Self {
        let total_points = half_points * 2;
        let mut table = Vec::with_capacity(phases * total_points);

        for phase_idx in 0..phases {
            let phase = phase_idx as f32 / phases as f32;
            let mut filter = Vec::with_capacity(total_points);

            for i in 0..total_points {
                let x = (i as f32 - (half_points as f32 - 1.0)) - phase;
                let sinc = if x.abs() < 1e-6 {
                    1.0
                } else {
                    let px = PI * x;
                    px.sin() / px
                };

                // Blackman window
                let norm = i as f32 / (total_points - 1) as f32;
                let window = 0.42 - 0.5 * (2.0 * PI * norm).cos() + 0.08 * (4.0 * PI * norm).cos();

                filter.push(sinc * window);
            }

            // Normalize filter coefficients
            let sum: f32 = filter.iter().sum();
            if sum.abs() > 1e-6 {
                for c in &mut filter {
                    *c /= sum;
                }
            }

            table.extend_from_slice(&filter);
        }

        Self {
            points: total_points,
            table,
            phases,
        }
    }

    /// Interpolates a single sample from a buffer at a fractional index `pos`
    pub fn interpolate(&self, buffer: &[f32], pos: f32) -> f32 {
        let int_pos = pos.floor() as isize;
        let frac = pos - pos.floor();

        let phase_idx = ((frac * self.phases as f32).round() as usize) % self.phases;
        let offset = phase_idx * self.points;
        let filter = &self.table[offset..offset + self.points];
        let half = (self.points / 2) as isize;

        let mut sum = 0.0f32;
        for (i, &coeff) in filter.iter().enumerate() {
            let sample_idx = int_pos - (half - 1) + i as isize;
            let sample = if sample_idx < 0 {
                buffer.first().copied().unwrap_or(0.0)
            } else if (sample_idx as usize) >= buffer.len() {
                buffer.last().copied().unwrap_or(0.0)
            } else {
                buffer[sample_idx as usize]
            };
            sum += sample * coeff;
        }

        sum
    }
}

impl Default for SincInterpolator {
    /// Ultra-compact low-memory default profile (12 points, 16 phases = 768 bytes RAM)
    fn default() -> Self {
        Self::new(6, 16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sinc_interpolator_identity() {
        let sinc = SincInterpolator::default();
        let buffer = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        // At exact integer index 2.0, output should be close to 1.0
        let val = sinc.interpolate(&buffer, 2.0);
        assert!((val - 1.0).abs() < 0.05);
    }
}
