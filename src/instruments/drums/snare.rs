use std::sync::Arc;
use crate::dsp::{
    envelope::Envelope, filter::OnePoleFilter, noise::NoiseGenerator, oscillator::Oscillator,
    vca::VCA,
};
use crate::types::waveform::Waveform;
use super::parameters::SnareParameters;

/// Snare drum synthesizer
/// Combines tonal component (oscillators) + noise component
pub struct SnareDrum {
    // Tonal component (body resonance)
    osc1: Oscillator,
    osc2: Oscillator,
    tone_envelope: Envelope,

    // Noise component (snare wires)
    noise: NoiseGenerator,
    noise_filter: OnePoleFilter,
    noise_envelope: Envelope,

    vca: VCA,
    parameters: Option<Arc<SnareParameters>>,  // Optional for backward compatibility
}

impl SnareDrum {
    /// Create new snare drum synthesizer with default hardcoded parameters
    pub fn new(sample_rate: f32) -> Self {
        let mut snare = Self {
            osc1: Oscillator::new(sample_rate),
            osc2: Oscillator::new(sample_rate),
            tone_envelope: Envelope::new(sample_rate),
            noise: NoiseGenerator::new(),
            noise_filter: OnePoleFilter::new(sample_rate, 5000.0), // Bright noise
            noise_envelope: Envelope::new(sample_rate),
            vca: VCA::new(),
            parameters: None,
        };

        // Set oscillators to sine wave for tonal body
        snare.osc1.set_waveform(Waveform::Sine);
        snare.osc2.set_waveform(Waveform::Sine);

        // Set oscillator frequencies (snare body)
        snare.osc1.set_frequency(180.0); // Fundamental
        snare.osc2.set_frequency(330.0); // Harmonic

        // Tone envelope: short, punchy
        snare.tone_envelope.set_adsr(0.001, 0.08, 0.0, 0.0);

        // Noise envelope: slightly longer for rattle
        snare.noise_envelope.set_adsr(0.001, 0.15, 0.0, 0.0);

        snare
    }

    /// Create new snare drum synthesizer with parameters for real-time control
    pub fn new_with_parameters(sample_rate: f32, parameters: Arc<SnareParameters>) -> Self {
        let mut snare = Self {
            osc1: Oscillator::new(sample_rate),
            osc2: Oscillator::new(sample_rate),
            tone_envelope: Envelope::new(sample_rate),
            noise: NoiseGenerator::new(),
            noise_filter: OnePoleFilter::new(sample_rate, 5000.0),
            noise_envelope: Envelope::new(sample_rate),
            vca: VCA::new(),
            parameters: Some(parameters),
        };

        // Set oscillators to sine wave for tonal body
        snare.osc1.set_waveform(Waveform::Sine);
        snare.osc2.set_waveform(Waveform::Sine);

        // Load initial parameters
        snare.update_from_parameters();

        snare
    }

    /// Update internal state from parameters (called once per trigger for efficiency)
    fn update_from_parameters(&mut self) {
        if let Some(params) = &self.parameters {
            use std::sync::atomic::Ordering;

            // Load all parameters atomically
            let tone_freq = params.tone_freq.load(Ordering::Relaxed);
            let _tone_mix = params.tone_mix.load(Ordering::Relaxed);
            let decay = params.decay.load(Ordering::Relaxed);
            let snap = params.snap.load(Ordering::Relaxed);

            // Update oscillator frequencies (maintain harmonic relationship)
            self.osc1.set_frequency(tone_freq);
            self.osc2.set_frequency(tone_freq * 1.83);  // Harmonic ratio

            // Decay affects both envelopes
            self.tone_envelope.set_adsr(0.001, decay * 0.5, 0.0, 0.0);  // Tone is shorter
            self.noise_envelope.set_adsr(0.001, decay, 0.0, 0.0);

            // Snap affects noise attack time - higher snap = faster attack
            let noise_attack = 0.001 * (1.0 - snap * 0.8);  // Range: 0.001s to 0.0002s
            self.noise_envelope.set_adsr(noise_attack, decay, 0.0, 0.0);

            // Note: tone_mix will be applied in next_sample during mixing
        }
    }

    /// Trigger the snare drum
    pub fn trigger(&mut self) {
        // Update parameters before triggering if using parameter control
        self.update_from_parameters();

        self.osc1.reset();
        self.osc2.reset();
        self.noise_filter.reset();

        self.tone_envelope.note_on();
        self.noise_envelope.note_on();

        // For one-shot behavior, envelopes will naturally decay to 0 (sustain = 0)
        // No need to call note_off() - that would capture release_level as 0.0
    }

    /// Check if the snare is still active (generating audio)
    pub fn is_active(&self) -> bool {
        self.tone_envelope.is_active() || self.noise_envelope.is_active()
    }

    /// Generate next audio sample
    pub fn next_sample(&mut self) -> f32 {
        // Tonal component
        let tone1 = self.osc1.next_sample();
        let tone2 = self.osc2.next_sample();
        let tone_mix = (tone1 + tone2) * 0.5;
        let tone_env = self.tone_envelope.next_sample();
        let tone_out = tone_mix * tone_env;

        // Noise component
        let noise_sample = self.noise.next_sample();
        let filtered_noise = self.noise_filter.process(noise_sample);
        let noise_env = self.noise_envelope.next_sample();
        let noise_out = filtered_noise * noise_env;

        // Mix tone and noise (60% tone, 40% noise)
        let mixed = tone_out * 0.6 + noise_out * 0.4;

        self.vca.process(mixed, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snare_creates() {
        let snare = SnareDrum::new(44100.0);
        assert!(!snare.is_active());
    }

    #[test]
    fn test_snare_trigger_activates() {
        let mut snare = SnareDrum::new(44100.0);
        snare.trigger();
        assert!(snare.is_active());
    }

    #[test]
    fn test_snare_generates_audio() {
        let mut snare = SnareDrum::new(44100.0);
        snare.trigger();

        let sample = snare.next_sample();
        assert!(sample.abs() >= 0.0);
    }

    #[test]
    fn test_snare_eventually_stops() {
        let mut snare = SnareDrum::new(44100.0);
        snare.trigger();

        // Process enough samples that it should stop
        for _ in 0..(44100.0 * 0.3) as usize {
            snare.next_sample();
        }

        // Should be inactive after decay period
        assert!(!snare.is_active());
    }
}
