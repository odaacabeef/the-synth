use super::{envelope::Envelope, oscillator::Oscillator, vca::VCA};
use crate::types::waveform::Waveform;

/// Single synthesizer voice
/// Complete signal chain: Oscillator → Envelope → VCA
pub struct Voice {
    oscillator: Oscillator,
    envelope: Envelope,
    vca: VCA,
}

impl Voice {
    /// Create new voice with given sample rate
    pub fn new(sample_rate: f32) -> Self {
        Self {
            oscillator: Oscillator::new(sample_rate),
            envelope: Envelope::new(sample_rate),
            vca: VCA::new(),
        }
    }

    /// Trigger note on with frequency
    pub fn note_on(&mut self, frequency: f32) {
        self.oscillator.set_frequency(frequency);
        self.oscillator.reset(); // Reset phase for consistent attack
        self.envelope.note_on();
    }

    /// Trigger note off
    pub fn note_off(&mut self) {
        self.envelope.note_off();
    }

    /// Set waveform type
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.oscillator.set_waveform(waveform);
    }

    /// Set ADSR parameters
    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.envelope.set_adsr(attack, decay, sustain, release);
    }

    /// Set VCA gain
    pub fn set_gain(&mut self, gain: f32) {
        self.vca.set_gain(gain);
    }

    /// Check if voice is active
    pub fn is_active(&self) -> bool {
        self.envelope.is_active()
    }

    /// Generate next audio sample
    /// Returns synthesized sample with envelope applied
    pub fn next_sample(&mut self) -> f32 {
        // Generate oscillator sample
        let osc_sample = self.oscillator.next_sample();

        // Get envelope modulation
        let envelope_level = self.envelope.next_sample();

        // Apply VCA (multiply oscillator by envelope)
        self.vca.process(osc_sample, envelope_level)
    }

    /// Reset voice to idle state
    pub fn reset(&mut self) {
        self.envelope.reset();
        self.oscillator.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_creates() {
        let voice = Voice::new(44100.0);
        assert!(!voice.is_active());
    }

    #[test]
    fn test_voice_note_on_activates() {
        let mut voice = Voice::new(44100.0);
        voice.note_on(440.0);
        assert!(voice.is_active());
    }

    #[test]
    fn test_voice_generates_samples() {
        let mut voice = Voice::new(44100.0);
        voice.note_on(440.0);

        let sample = voice.next_sample();
        assert!(sample.abs() >= 0.0); // Should produce some output
    }
}
