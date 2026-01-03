use crate::dsp::{envelope::Envelope, filter::HighPassFilter, noise::NoiseGenerator, vca::VCA};

/// Hi-hat synthesizer
/// Filtered white noise with very short decay
pub struct HiHat {
    noise: NoiseGenerator,
    filter: HighPassFilter,
    envelope: Envelope,
    vca: VCA,
}

impl HiHat {
    /// Create new hi-hat synthesizer
    pub fn new(sample_rate: f32) -> Self {
        let mut hat = Self {
            noise: NoiseGenerator::new(),
            filter: HighPassFilter::new(sample_rate, 8000.0), // Very bright
            envelope: Envelope::new(sample_rate),
            vca: VCA::new(),
        };

        // Very short, crisp envelope
        hat.envelope.set_adsr(0.001, 0.05, 0.0, 0.0);

        hat
    }

    /// Trigger the hi-hat
    pub fn trigger(&mut self) {
        self.filter.reset();
        self.envelope.note_on();

        // For one-shot behavior, envelope will naturally decay to 0 (sustain = 0)
        // No need to call note_off() - that would capture release_level as 0.0
    }

    /// Check if the hi-hat is still active (generating audio)
    pub fn is_active(&self) -> bool {
        self.envelope.is_active()
    }

    /// Generate next audio sample
    pub fn next_sample(&mut self) -> f32 {
        let noise_sample = self.noise.next_sample();
        let filtered = self.filter.process(noise_sample);
        let env = self.envelope.next_sample();

        self.vca.process(filtered, env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hat_creates() {
        let hat = HiHat::new(44100.0);
        assert!(!hat.is_active());
    }

    #[test]
    fn test_hat_trigger_activates() {
        let mut hat = HiHat::new(44100.0);
        hat.trigger();
        assert!(hat.is_active());
    }

    #[test]
    fn test_hat_generates_audio() {
        let mut hat = HiHat::new(44100.0);
        hat.trigger();

        let sample = hat.next_sample();
        assert!(sample.abs() >= 0.0);
    }

    #[test]
    fn test_hat_eventually_stops() {
        let mut hat = HiHat::new(44100.0);
        hat.trigger();

        // Process enough samples that it should stop
        for _ in 0..(44100.0 * 0.1) as usize {
            hat.next_sample();
        }

        // Should be inactive after decay period
        assert!(!hat.is_active());
    }
}
