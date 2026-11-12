use std::path::Path;
use std::process::Command;

/// Estimates BPM from an audio file using ffmpeg to extract downsampled mono PCM
/// and computing an onset energy autocorrelation with parabolic peak refinement.
pub fn estimate_bpm<P: AsRef<Path>>(file_path: P) -> Option<f64> {
    let p = file_path.as_ref();
    if !p.exists() {
        return None;
    }

    let ffmpeg_bin = find_ffmpeg()?;

    // Extract 60 seconds of mono 11025Hz f32le audio from the 20-second mark
    let output = Command::new(&ffmpeg_bin)
        .arg("-v")
        .arg("quiet")
        .arg("-ss")
        .arg("20")
        .arg("-t")
        .arg("60")
        .arg("-i")
        .arg(p)
        .arg("-f")
        .arg("f32le")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("11025")
        .arg("pipe:1")
        .output()
        .ok()?;

    if !output.status.success() || output.stdout.len() < 11025 * 4 * 5 {
        // Fallback: if track is short (< 25s), try from beginning
        let fallback = Command::new(&ffmpeg_bin)
            .arg("-v")
            .arg("quiet")
            .arg("-i")
            .arg(p)
            .arg("-f")
            .arg("f32le")
            .arg("-ac")
            .arg("1")
            .arg("-ar")
            .arg("11025")
            .arg("pipe:1")
            .output()
            .ok()?;

        if !fallback.status.success() || fallback.stdout.len() < 11025 * 4 * 5 {
            return None;
        }
        return compute_bpm_from_pcm(&fallback.stdout, 11025);
    }

    compute_bpm_from_pcm(&output.stdout, 11025)
}

fn find_ffmpeg() -> Option<String> {
    for candidate in &["ffmpeg", "/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg", "/usr/bin/ffmpeg"] {
        if let Ok(res) = Command::new(candidate).arg("-version").output() {
            if res.status.success() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

fn compute_bpm_from_pcm(raw_bytes: &[u8], sr: usize) -> Option<f64> {
    if raw_bytes.len() < 4 {
        return None;
    }
    let sample_count = raw_bytes.len() / 4;
    let mut samples = Vec::with_capacity(sample_count);
    for chunk in raw_bytes.chunks_exact(4) {
        let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        samples.push(f);
    }

    if samples.len() < sr * 5 {
        return None;
    }

    // 15ms window, 5ms hop
    let win_size = (sr as f64 * 0.015) as usize;
    let hop_size = (sr as f64 * 0.005) as usize;
    if win_size == 0 || hop_size == 0 || samples.len() <= win_size {
        return None;
    }

    let mut energies = Vec::new();
    let mut i = 0;
    while i + win_size <= samples.len() {
        let mut sum_sq = 0.0f64;
        for j in 0..win_size {
            let s = samples[i + j] as f64;
            sum_sq += s * s;
        }
        let rms = (sum_sq / win_size as f64).sqrt();
        energies.push(rms);
        i += hop_size;
    }

    if energies.len() < 20 {
        return None;
    }

    // Onset detection function (first difference, half-wave rectified)
    let mut onsets = Vec::with_capacity(energies.len());
    onsets.push(0.0f64);
    for idx in 1..energies.len() {
        let diff = energies[idx] - energies[idx - 1];
        onsets.push(if diff > 0.0 { diff } else { 0.0 });
    }

    let fps = sr as f64 / hop_size as f64;
    let min_bpm = 65.0f64;
    let max_bpm = 195.0f64;

    let min_lag = (fps * 60.0 / max_bpm).round() as usize;
    let max_lag = (fps * 60.0 / min_bpm).round() as usize;

    if max_lag >= onsets.len() || min_lag >= max_lag {
        return None;
    }

    let mut corrs = vec![0.0f64; max_lag + 2];
    let mut best_lag = min_lag;
    let mut best_val = -1.0f64;

    for lag in min_lag..=max_lag {
        let mut c = 0.0f64;
        let limit = onsets.len() - lag;
        for k in 0..limit {
            c += onsets[k] * onsets[k + lag];
        }
        if lag * 2 < onsets.len() {
            let mut c2 = 0.0f64;
            let limit2 = onsets.len() - lag * 2;
            for k in 0..limit2 {
                c2 += onsets[k] * onsets[k + lag * 2];
            }
            c += 0.35 * c2;
        }

        corrs[lag] = c;
        if c > best_val {
            best_val = c;
            best_lag = lag;
        }
    }

    if best_val <= 0.0 || best_lag <= min_lag || best_lag >= max_lag {
        let raw_bpm = (fps * 60.0) / (best_lag as f64);
        return Some((raw_bpm * 10.0).round() / 10.0);
    }

    // Parabolic interpolation for fine sub-sample lag resolution
    let c_prev = corrs[best_lag - 1];
    let c_curr = corrs[best_lag];
    let c_next = corrs[best_lag + 1];
    let denom = 2.0 * (2.0 * c_curr - c_prev - c_next);
    let delta = if denom.abs() > 1e-6 {
        (c_next - c_prev) / denom
    } else {
        0.0
    };

    let true_lag = best_lag as f64 + delta;
    if true_lag <= 0.0 {
        return None;
    }

    let bpm = (fps * 60.0) / true_lag;
    let rounded = (bpm * 10.0).round() / 10.0;
    Some(rounded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_bpm_synthetic() {
        // Generate a synthetic click track at 120 BPM:
        // 120 BPM = 2 beats per second = beat every 0.5 sec (every 5512.5 samples at 11025Hz)
        let sr = 11025;
        let duration_secs = 10;
        let total_samples = sr * duration_secs;
        let mut pcm = vec![0.0f32; total_samples];

        let beat_interval = (sr as f64 * 0.5) as usize;
        for b in (0..total_samples).step_by(beat_interval) {
            // Click pulse of 100 samples
            for j in 0..100 {
                if b + j < total_samples {
                    pcm[b + j] = 0.8;
                }
            }
        }

        let mut bytes = Vec::with_capacity(total_samples * 4);
        for s in pcm {
            bytes.extend_from_slice(&s.to_le_bytes());
        }

        let estimated = compute_bpm_from_pcm(&bytes, sr);
        assert!(estimated.is_some(), "Should estimate BPM");
        let bpm = estimated.unwrap();
        assert!((bpm - 120.0).abs() <= 2.0, "Expected ~120 BPM, got {}", bpm);
    }
}
