//! Compound Crossfader corresponding to `audiocore::AudioCore::getCompoundXFader`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossfaderCurve {
    Linear,
    Smooth, // Equal power / constant power curve (-3dB center)
    Sharp,  // Scratch cut curve
}

#[derive(Debug, Clone)]
pub struct CompoundCrossfader {
    pub position: f32, // -1.0 (Full Deck A) to +1.0 (Full Deck B)
    pub curve: CrossfaderCurve,
}

impl CompoundCrossfader {
    pub fn new() -> Self {
        Self {
            position: 0.0, // Center
            curve: CrossfaderCurve::Smooth,
        }
    }

    /// Returns `(gain_a, gain_b)` corresponding to the crossfader position and curve
    pub fn calculate_gains(&self) -> (f32, f32) {
        // Normalize position from [-1.0, 1.0] to [0.0, 1.0] (0.0 = A, 1.0 = B)
        let t = ((self.position.clamp(-1.0, 1.0) + 1.0) / 2.0).clamp(0.0, 1.0);

        match self.curve {
            CrossfaderCurve::Linear => {
                let gain_a = (1.0 - t).max(0.0);
                let gain_b = t.max(0.0);
                (gain_a, gain_b)
            }
            CrossfaderCurve::Smooth => {
                // Constant power curve (sine/cosine law)
                let angle = t * std::f32::consts::FRAC_PI_2;
                (angle.cos().max(0.0), angle.sin().max(0.0))
            }
            CrossfaderCurve::Sharp => {
                // DJ Scratch cut: Full volume almost everywhere, quick fade at edges
                let gain_a = if t > 0.95 { (1.0 - t) / 0.05 } else { 1.0 };
                let gain_b = if t < 0.05 { t / 0.05 } else { 1.0 };
                (gain_a, gain_b)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossfader_center() {
        let mut xf = CompoundCrossfader::new();
        xf.position = 0.0;
        let (ga, gb) = xf.calculate_gains();
        assert!((ga - 0.7071).abs() < 0.01);
        assert!((gb - 0.7071).abs() < 0.01);
    }

    #[test]
    fn test_crossfader_extremes() {
        let mut xf = CompoundCrossfader::new();
        xf.position = -1.0; // Left (Deck A)
        let (ga, gb) = xf.calculate_gains();
        assert!((ga - 1.0).abs() < 1e-6);
        assert!((gb - 0.0).abs() < 1e-6);

        xf.position = 1.0; // Right (Deck B)
        let (ga, gb) = xf.calculate_gains();
        assert!((ga - 0.0).abs() < 1e-6);
        assert!((gb - 1.0).abs() < 1e-6);
    }
}
