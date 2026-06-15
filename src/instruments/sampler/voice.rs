use std::sync::Arc;

use super::sample::SampleData;

/// A single sample-playback voice: a fractional playhead reading through
/// shared `SampleData` with linear interpolation.
///
/// Linear fades at the start (attack) and end (release) of playback prevent
/// clicks at the sample boundaries and when a voice is stolen mid-playback.
pub struct SamplerVoice {
    sample: Option<Arc<SampleData>>,
    position: f64, // fractional read position in source samples
    rate: f64,     // position increment per output sample
    gain: f32,     // linear gain applied to every sample
    note: Option<u8>,
    active: bool,

    // Fade lengths in output samples.
    attack_samples: f32,
    release_samples: f32,
    samples_played: f32, // output samples since trigger (drives fade-in)
}

impl SamplerVoice {
    pub fn new() -> Self {
        Self {
            sample: None,
            position: 0.0,
            rate: 1.0,
            gain: 1.0,
            note: None,
            active: false,
            attack_samples: 0.0,
            release_samples: 0.0,
            samples_played: 0.0,
        }
    }

    /// Start playback of `sample` from `start_pos` (in source samples).
    #[allow(clippy::too_many_arguments)]
    pub fn trigger(
        &mut self,
        sample: Arc<SampleData>,
        start_pos: f64,
        rate: f64,
        gain: f32,
        attack_samples: f32,
        release_samples: f32,
        note: u8,
    ) {
        self.sample = Some(sample);
        self.position = start_pos.max(0.0);
        self.rate = rate.max(0.0001); // guard against zero/negative rate
        self.gain = gain;
        self.attack_samples = attack_samples.max(0.0);
        self.release_samples = release_samples.max(0.0);
        self.samples_played = 0.0;
        self.note = Some(note);
        self.active = true;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn note(&self) -> Option<u8> {
        self.note
    }

    /// Immediately silence the voice (used for MIDI panic / all-notes-off).
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Generate the next output sample, advancing the playhead.
    pub fn next_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let sample = match &self.sample {
            Some(s) => s,
            None => {
                self.active = false;
                return 0.0;
            }
        };

        let len = sample.samples.len();
        let i = self.position.floor() as usize;

        // Need two samples to interpolate; deactivate once we reach the end.
        if i + 1 >= len {
            self.active = false;
            return 0.0;
        }

        // Linear interpolation between adjacent samples.
        let frac = (self.position - i as f64) as f32;
        let s0 = sample.samples[i];
        let s1 = sample.samples[i + 1];
        let mut out = s0 + (s1 - s0) * frac;

        // Amplitude fade envelope: ramp up over attack, ramp down into the
        // final `release` window so playback ends silently.
        let mut env = 1.0;
        if self.attack_samples > 0.0 && self.samples_played < self.attack_samples {
            env *= self.samples_played / self.attack_samples;
        }
        if self.release_samples > 0.0 {
            let output_remaining = ((len as f64 - self.position) / self.rate) as f32;
            if output_remaining < self.release_samples {
                env *= (output_remaining / self.release_samples).max(0.0);
            }
        }

        out *= env * self.gain;

        self.position += self.rate;
        self.samples_played += 1.0;

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(n: usize) -> Arc<SampleData> {
        Arc::new(SampleData {
            samples: vec![0.5; n],
            sample_rate: 44100.0,
        })
    }

    #[test]
    fn test_inactive_by_default() {
        let v = SamplerVoice::new();
        assert!(!v.is_active());
    }

    #[test]
    fn test_trigger_produces_audio() {
        let mut v = SamplerVoice::new();
        v.trigger(make_sample(1000), 0.0, 1.0, 1.0, 0.0, 0.0, 60);
        assert!(v.is_active());
        // No fades, constant 0.5 source -> 0.5 output.
        assert!((v.next_sample() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_deactivates_at_end() {
        let mut v = SamplerVoice::new();
        v.trigger(make_sample(10), 0.0, 1.0, 1.0, 0.0, 0.0, 60);
        for _ in 0..20 {
            v.next_sample();
        }
        assert!(!v.is_active());
    }

    #[test]
    fn test_rate_advances_faster() {
        let mut v = SamplerVoice::new();
        v.trigger(make_sample(100), 0.0, 2.0, 1.0, 0.0, 0.0, 60);
        let mut count = 0;
        while v.is_active() && count < 1000 {
            v.next_sample();
            count += 1;
        }
        // At rate 2.0 it takes ~50 output samples to cross 100 source samples.
        assert!(count < 60);
    }

    #[test]
    fn test_fade_in_starts_silent() {
        let mut v = SamplerVoice::new();
        v.trigger(make_sample(1000), 0.0, 1.0, 1.0, 100.0, 0.0, 60);
        // First sample is at the very start of the attack ramp -> silent.
        assert!(v.next_sample().abs() < 0.001);
    }

    #[test]
    fn test_stop_silences() {
        let mut v = SamplerVoice::new();
        v.trigger(make_sample(1000), 0.0, 1.0, 1.0, 0.0, 0.0, 60);
        v.stop();
        assert!(!v.is_active());
        assert_eq!(v.next_sample(), 0.0);
    }
}
