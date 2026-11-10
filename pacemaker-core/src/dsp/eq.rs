//! 3-Band Parametric EQ & Kill Filter corresponding to `xponaut::audioengine::PeqAudioProcess`.

use super::buffer::AudioProcessBuffer;

#[derive(Debug, Clone)]
pub struct ParametricEq {
    pub sample_rate: f32,
    // Gains in dB (-26.0 dB to +6.0 dB)
    pub low_gain_db: f32,
    pub mid_gain_db: f32,
    pub high_gain_db: f32,
    // Kill switches (true = muted)
    pub low_kill: bool,
    pub mid_kill: bool,
    pub high_kill: bool,

    // Internal filter state per channel (2 channels for stereo)
    state_low: [Biquad; 2],
    state_mid: [Biquad; 2],
    state_high: [Biquad; 2],
}

impl ParametricEq {
    pub fn new(sample_rate: f32) -> Self {
        let mut eq = Self {
            sample_rate,
            low_gain_db: 0.0,
            mid_gain_db: 0.0,
            high_gain_db: 0.0,
            low_kill: false,
            mid_kill: false,
            high_kill: false,
            state_low: [Biquad::new(), Biquad::new()],
            state_mid: [Biquad::new(), Biquad::new()],
            state_high: [Biquad::new(), Biquad::new()],
        };
        eq.recalculate();
        eq
    }

    pub fn set_gains(&mut self, low_db: f32, mid_db: f32, high_db: f32) {
        self.low_gain_db = low_db;
        self.mid_gain_db = mid_db;
        self.high_gain_db = high_db;
        self.recalculate();
    }

    pub fn set_kills(&mut self, low_kill: bool, mid_kill: bool, high_kill: bool) {
        self.low_kill = low_kill;
        self.mid_kill = mid_kill;
        self.high_kill = high_kill;
        self.recalculate();
    }

    fn recalculate(&mut self) {
        let low_db = if self.low_kill { -90.0 } else { self.low_gain_db };
        let mid_db = if self.mid_kill { -90.0 } else { self.mid_gain_db };
        let high_db = if self.high_kill { -90.0 } else { self.high_gain_db };

        // Pacemaker standard crossover frequencies: Low: 250Hz, Mid: 1kHz (Q=1.0), High: 4kHz
        for i in 0..2 {
            self.state_low[i].configure_lowshelf(self.sample_rate, 250.0, low_db);
            self.state_mid[i].configure_peaking(self.sample_rate, 1000.0, 1.0, mid_db);
            self.state_high[i].configure_highshelf(self.sample_rate, 4000.0, high_db);
        }
    }

    pub fn process(&mut self, buffer: &mut AudioProcessBuffer) {
        let ch_count = buffer.channels.min(2);
        for c in 0..ch_count {
            for sample in &mut buffer.data[c] {
                let mut s = *sample;
                s = self.state_low[c].process(s);
                s = self.state_mid[c].process(s);
                s = self.state_high[c].process(s);
                *sample = s;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Biquad {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl Biquad {
    fn new() -> Self {
        Self { b0: 1.0, ..Default::default() }
    }

    fn process(&mut self, x0: f32) -> f32 {
        let y0 = self.b0 * x0 + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y0;
        y0
    }

    fn configure_lowshelf(&mut self, fs: f32, f0: f32, gain_db: f32) {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * f0 / fs;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * 2.0f32.sqrt();

        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        self.b0 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha)) / a0;
        self.b1 = (2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
        self.b2 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha)) / a0;
        self.a1 = (-2.0 * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
        self.a2 = ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha) / a0;
    }

    fn configure_highshelf(&mut self, fs: f32, f0: f32, gain_db: f32) {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * f0 / fs;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * 2.0f32.sqrt();

        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        self.b0 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha)) / a0;
        self.b1 = (-2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
        self.b2 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha)) / a0;
        self.a1 = (2.0 * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
        self.a2 = ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha) / a0;
    }

    fn configure_peaking(&mut self, fs: f32, f0: f32, q: f32, gain_db: f32) {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * f0 / fs;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let a0 = 1.0 + alpha / a;
        self.b0 = (1.0 + alpha * a) / a0;
        self.b1 = (-2.0 * cos_w0) / a0;
        self.b2 = (1.0 - alpha * a) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha / a) / a0;
    }
}
