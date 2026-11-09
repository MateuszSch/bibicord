//! Microphone processing before voice activation and encoding:
//! RNNoise noise suppression (`nnnoiseless`, pure Rust) and dBFS helpers
//! for the manual voice-activation threshold.

use nnnoiseless::DenoiseState;

/// Manual sensitivity range in dBFS (slider bounds).
pub const DB_MIN: i32 = -80;
pub const DB_MAX: i32 = 0;

/// RMS level in dBFS (silence clamps to -100).
pub fn db(rms: f32) -> f32 {
    20.0 * rms.max(1e-5).log10()
}

/// RNNoise suppressor for 48 kHz mono. Works in 10 ms blocks; adds 10 ms delay.
pub struct Denoise {
    st: Box<DenoiseState<'static>>,
}

impl Denoise {
    pub fn new() -> Self {
        Denoise { st: DenoiseState::new() }
    }

    /// Denoises `frame` in place; its length must be a multiple of 480.
    // ponytail: state is kept while disabled, so re-enabling carries ≤10 ms of stale audio.
    pub fn process(&mut self, frame: &mut [f32]) {
        const N: usize = nnnoiseless::FRAME_SIZE;
        let (mut input, mut output) = ([0f32; N], [0f32; N]);
        for chunk in frame.chunks_exact_mut(N) {
            // RNNoise expects 16-bit sample scale.
            for (i, s) in input.iter_mut().zip(chunk.iter()) {
                *i = s * 32768.0;
            }
            self.st.process_frame(&mut output, &input);
            for (s, o) in chunk.iter_mut().zip(output.iter()) {
                *s = o / 32768.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    }

    #[test]
    fn db_scale() {
        assert!(db(1.0).abs() < 1e-4);
        assert!((db(0.1) + 20.0).abs() < 1e-4);
        assert_eq!(db(0.0), -100.0);
    }

    #[test]
    fn suppresses_steady_noise() {
        let mut d = Denoise::new();
        let (mut x, mut brown) = (0x2545f4914f6cdd1du64, 0f32);
        let (mut before, mut after) = (Vec::new(), Vec::new());
        for k in 0..200 {
            // 4 s of low-frequency (brown) noise, like a fan or hum.
            let mut frame: Vec<f32> = (0..960)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    brown = 0.98 * brown + 0.2 * ((x >> 11) as f32 / (1u64 << 53) as f32 * 2.0 - 1.0);
                    0.01 * brown
                })
                .collect();
            let orig = frame.clone();
            d.process(&mut frame);
            if k >= 50 {
                before.extend(orig);
                after.extend(frame);
            }
        }
        let gain = db(rms(&after)) - db(rms(&before));
        assert!(gain < -20.0, "noise reduced by only {gain:.1} dB");
    }
}
