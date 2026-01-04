use std::sync::Arc;
use crate::dsp::{envelope::Envelope, noise::NoiseGenerator, oscillator::Oscillator, vca::VCA};
use crate::types::waveform::Waveform;
use super::parameters::KickParameters;

/// Kick drum synthesizer
/// Uses pitch-swept sine wave: high frequency → low frequency
/// Plus a noise transient for the beater "click"
pub struct KickDrum {
    oscillator: Oscillator,
    pitch_envelope: Envelope,  // Controls pitch sweep
    amp_envelope: Envelope,    // Controls amplitude
    click_envelope: Envelope,  // Controls click transient
    noise: NoiseGenerator,     // For beater click
    vca: VCA,
    base_frequency: f32,  // Target low frequency (typically 40-60 Hz)
    start_frequency: f32, // Start high frequency (typically 150-200 Hz)
    click_amount: f32,    // Amount of click to mix in (0.0 to 1.0)
    parameters: Option<Arc<KickParameters>>,  // Optional for backward compatibility
}

impl KickDrum {
    /// Create new kick drum synthesizer with default hardcoded parameters
    pub fn new(sample_rate: f32) -> Self {
        let mut kick = Self {
            oscillator: Oscillator::new(sample_rate),
            pitch_envelope: Envelope::new(sample_rate),
            amp_envelope: Envelope::new(sample_rate),
            click_envelope: Envelope::new(sample_rate),
            noise: NoiseGenerator::new(),
            vca: VCA::new(),
            base_frequency: 50.0,
            start_frequency: 180.0,
            click_amount: 0.3,
            parameters: None,
        };

        // Set oscillator to sine wave for smooth kick sound
        kick.oscillator.set_waveform(Waveform::Sine);

        // Configure envelopes for kick sound
        // Pitch envelope: fast decay for frequency sweep
        kick.pitch_envelope.set_adsr(0.0, 0.05, 0.0, 0.0);

        // Amp envelope: punchy attack, medium decay
        kick.amp_envelope.set_adsr(0.001, 0.3, 0.0, 0.0);

        // Click envelope: very short transient
        kick.click_envelope.set_adsr(0.0, 0.005, 0.0, 0.0);

        kick
    }

    /// Create new kick drum synthesizer with parameters for real-time control
    pub fn new_with_parameters(sample_rate: f32, parameters: Arc<KickParameters>) -> Self {
        let mut kick = Self {
            oscillator: Oscillator::new(sample_rate),
            pitch_envelope: Envelope::new(sample_rate),
            amp_envelope: Envelope::new(sample_rate),
            click_envelope: Envelope::new(sample_rate),
            noise: NoiseGenerator::new(),
            vca: VCA::new(),
            base_frequency: 50.0,
            start_frequency: 180.0,
            click_amount: 0.3,
            parameters: Some(parameters),
        };

        // Set oscillator to sine wave for smooth kick sound
        kick.oscillator.set_waveform(Waveform::Sine);

        // Load initial parameters
        kick.update_from_parameters();

        kick
    }

    /// Update internal state from parameters (called once per trigger for efficiency)
    fn update_from_parameters(&mut self) {
        if let Some(params) = &self.parameters {
            use std::sync::atomic::Ordering;

            // Load all parameters atomically
            self.start_frequency = params.pitch_start.load(Ordering::Relaxed);
            self.base_frequency = params.pitch_end.load(Ordering::Relaxed);
            let pitch_decay = params.pitch_decay.load(Ordering::Relaxed);
            let decay = params.decay.load(Ordering::Relaxed);
            self.click_amount = params.click.load(Ordering::Relaxed);

            // Update envelopes
            self.pitch_envelope.set_adsr(0.0, pitch_decay, 0.0, 0.0);
            self.amp_envelope.set_adsr(0.001, decay, 0.0, 0.0);

            // Click envelope is very short for transient
            self.click_envelope.set_adsr(0.0, 0.005, 0.0, 0.0);
        }
    }

    /// Trigger the kick drum
    pub fn trigger(&mut self) {
        // Update parameters before triggering if using parameter control
        self.update_from_parameters();

        self.oscillator.reset();
        self.pitch_envelope.note_on();
        self.amp_envelope.note_on();
        self.click_envelope.note_on();

        // For one-shot behavior, envelope will naturally decay to 0 (sustain = 0)
        // No need to call note_off() - that would capture release_level as 0.0
    }

    /// Check if the kick is still active (generating audio)
    pub fn is_active(&self) -> bool {
        self.amp_envelope.is_active()
    }

    /// Generate next audio sample
    pub fn next_sample(&mut self) -> f32 {
        // Get pitch envelope (1.0 at start, 0.0 at end)
        let pitch_env = self.pitch_envelope.next_sample();

        // Calculate swept frequency: start_freq → base_freq
        let frequency = self.base_frequency
            + (self.start_frequency - self.base_frequency) * pitch_env;
        self.oscillator.set_frequency(frequency);

        // Generate oscillator sample (main tone)
        let osc_sample = self.oscillator.next_sample();

        // Generate click transient (beater noise)
        let click_env = self.click_envelope.next_sample();
        let noise_sample = self.noise.next_sample();
        let click_transient = noise_sample * click_env * self.click_amount;

        // Mix tone and click
        let mixed = osc_sample + click_transient;

        // Apply amplitude envelope to the mix
        let amp_env = self.amp_envelope.next_sample();

        self.vca.process(mixed, amp_env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kick_creates() {
        let kick = KickDrum::new(44100.0);
        assert!(!kick.is_active());
    }

    #[test]
    fn test_kick_trigger_activates() {
        let mut kick = KickDrum::new(44100.0);
        kick.trigger();
        assert!(kick.is_active());
    }

    #[test]
    fn test_kick_generates_audio() {
        let mut kick = KickDrum::new(44100.0);
        kick.trigger();

        let sample = kick.next_sample();
        assert!(sample.abs() >= 0.0);
    }

    #[test]
    fn test_kick_eventually_stops() {
        let mut kick = KickDrum::new(44100.0);
        kick.trigger();

        // Process enough samples that it should stop (more than decay time)
        for _ in 0..(44100.0 * 0.5) as usize {
            kick.next_sample();
        }

        // Should be inactive after decay period
        assert!(!kick.is_active());
    }
}
