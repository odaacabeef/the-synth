use std::sync::Arc;
use crate::dsp::{envelope::Envelope, filter::HighPassFilter, noise::NoiseGenerator, vca::VCA};
use super::parameters::HatParameters;

/// Hi-hat synthesizer
/// Filtered white noise with very short decay
pub struct HiHat {
    noise: NoiseGenerator,
    filter: HighPassFilter,
    envelope: Envelope,
    vca: VCA,
    parameters: Option<Arc<HatParameters>>,  // Optional for backward compatibility
}

impl HiHat {
    /// Create new hi-hat synthesizer with default hardcoded parameters
    pub fn new(sample_rate: f32) -> Self {
        let mut hat = Self {
            noise: NoiseGenerator::new(),
            filter: HighPassFilter::new(sample_rate, 8000.0), // Very bright
            envelope: Envelope::new(sample_rate),
            vca: VCA::new(),
            parameters: None,
        };

        // Very short, crisp envelope
        hat.envelope.set_adsr(0.001, 0.05, 0.0, 0.0);

        hat
    }

    /// Create new hi-hat synthesizer with parameters for real-time control
    pub fn new_with_parameters(sample_rate: f32, parameters: Arc<HatParameters>) -> Self {
        let mut hat = Self {
            noise: NoiseGenerator::new(),
            filter: HighPassFilter::new(sample_rate, 7000.0), // Will be updated from parameters
            envelope: Envelope::new(sample_rate),
            vca: VCA::new(),
            parameters: Some(parameters),
        };

        // Load initial parameters
        hat.update_from_parameters();

        hat
    }

    /// Update internal state from parameters (called once per trigger for efficiency)
    fn update_from_parameters(&mut self) {
        if let Some(params) = &self.parameters {
            use std::sync::atomic::Ordering;

            // Load all parameters atomically
            let brightness = params.brightness.load(Ordering::Relaxed);
            let decay = params.decay.load(Ordering::Relaxed);
            let metallic = params.metallic.load(Ordering::Relaxed);

            // Update filter cutoff
            self.filter.set_cutoff(brightness);

            // Metallic affects attack time - higher metallic = faster attack for sharper sound
            let attack_time = 0.001 * (1.0 - metallic * 0.8);  // Range: 0.001s to 0.0002s

            // Update envelope with parameters
            self.envelope.set_adsr(attack_time, decay, 0.0, 0.0);
        }
    }

    /// Trigger the hi-hat
    pub fn trigger(&mut self) {
        // Update parameters before triggering if using parameter control
        self.update_from_parameters();

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
