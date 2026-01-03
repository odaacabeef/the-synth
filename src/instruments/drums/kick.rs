use crate::dsp::{envelope::Envelope, oscillator::Oscillator, vca::VCA};
use crate::types::waveform::Waveform;

/// Kick drum synthesizer
/// Uses pitch-swept sine wave: high frequency → low frequency
pub struct KickDrum {
    oscillator: Oscillator,
    pitch_envelope: Envelope,  // Controls pitch sweep
    amp_envelope: Envelope,    // Controls amplitude
    vca: VCA,
    base_frequency: f32,  // Target low frequency (typically 40-60 Hz)
    start_frequency: f32, // Start high frequency (typically 150-200 Hz)
}

impl KickDrum {
    /// Create new kick drum synthesizer
    pub fn new(sample_rate: f32) -> Self {
        let mut kick = Self {
            oscillator: Oscillator::new(sample_rate),
            pitch_envelope: Envelope::new(sample_rate),
            amp_envelope: Envelope::new(sample_rate),
            vca: VCA::new(),
            base_frequency: 50.0,
            start_frequency: 180.0,
        };

        // Set oscillator to sine wave for smooth kick sound
        kick.oscillator.set_waveform(Waveform::Sine);

        // Configure envelopes for kick sound
        // Pitch envelope: fast decay for frequency sweep
        kick.pitch_envelope.set_adsr(0.0, 0.05, 0.0, 0.0);

        // Amp envelope: punchy attack, medium decay
        kick.amp_envelope.set_adsr(0.001, 0.3, 0.0, 0.0);

        kick
    }

    /// Trigger the kick drum
    pub fn trigger(&mut self) {
        self.oscillator.reset();
        self.pitch_envelope.note_on();
        self.amp_envelope.note_on();

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

        // Generate oscillator sample
        let osc_sample = self.oscillator.next_sample();

        // Apply amplitude envelope
        let amp_env = self.amp_envelope.next_sample();

        self.vca.process(osc_sample, amp_env)
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
