use std::sync::Arc;
use crate::dsp::{
    envelope::Envelope, filter::OnePoleFilter, noise::NoiseGenerator, oscillator::Oscillator,
    vca::VCA,
};
use crate::types::waveform::Waveform;
use super::parameters::SnareParameters;

/// Snare drum synthesizer
/// Combines tonal component (oscillators) + noise component + snap transient
pub struct SnareDrum {
    // Tonal component (body resonance)
    osc1: Oscillator,
    osc2: Oscillator,
    tone_envelope: Envelope,

    // Noise component (snare wires)
    noise: NoiseGenerator,
    noise_filter: OnePoleFilter,
    noise_envelope: Envelope,

    // Snap transient (stick attack)
    snap_envelope: Envelope,
    snap_noise: NoiseGenerator,

    vca: VCA,
    tone_mix: f32,  // Balance between tone and noise (0.0 = all noise, 1.0 = all tone)
    snap_amount: f32,  // Amount of snap transient and brightness (0.0 to 1.0)
    parameters: Option<Arc<SnareParameters>>,  // Optional for backward compatibility
}

impl SnareDrum {
    /// Create new snare drum synthesizer with default hardcoded parameters
    #[allow(dead_code)]
    pub fn new(sample_rate: f32) -> Self {
        let mut snare = Self {
            osc1: Oscillator::new(sample_rate),
            osc2: Oscillator::new(sample_rate),
            tone_envelope: Envelope::new(sample_rate),
            noise: NoiseGenerator::new(),
            noise_filter: OnePoleFilter::new(sample_rate, 5000.0), // Bright noise
            noise_envelope: Envelope::new(sample_rate),
            snap_envelope: Envelope::new(sample_rate),
            snap_noise: NoiseGenerator::new(),
            vca: VCA::new(),
            tone_mix: 0.65,  // Default: 65% tone, 35% noise
            snap_amount: 0.7,  // Default: snappy
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

        // Snap envelope: very short for stick attack
        snare.snap_envelope.set_adsr(0.0, 0.003, 0.0, 0.0);

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
            snap_envelope: Envelope::new(sample_rate),
            snap_noise: NoiseGenerator::new(),
            vca: VCA::new(),
            tone_mix: 0.65,
            snap_amount: 0.7,
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
            self.tone_mix = params.tone_mix.load(Ordering::Relaxed);
            let decay = params.decay.load(Ordering::Relaxed);
            self.snap_amount = params.snap.load(Ordering::Relaxed);

            // Update oscillator frequencies (maintain harmonic relationship)
            self.osc1.set_frequency(tone_freq);
            self.osc2.set_frequency(tone_freq * 1.83);  // Harmonic ratio

            // Decay affects envelopes
            self.tone_envelope.set_adsr(0.001, decay * 0.5, 0.0, 0.0);  // Tone is shorter
            self.noise_envelope.set_adsr(0.001, decay, 0.0, 0.0);

            // Snap envelope is very short for transient
            self.snap_envelope.set_adsr(0.0, 0.003, 0.0, 0.0);

            // Snap controls noise filter brightness - higher snap = brighter noise
            // Map snap (0.0 to 1.0) to filter cutoff (3000 Hz to 10000 Hz)
            let noise_cutoff = 3000.0 + (self.snap_amount * 7000.0);
            self.noise_filter.set_cutoff(noise_cutoff);
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
        self.snap_envelope.note_on();

        // For one-shot behavior, envelopes will naturally decay to 0 (sustain = 0)
        // No need to call note_off() - that would capture release_level as 0.0
    }

    /// Check if the snare is still active (generating audio)
    pub fn is_active(&self) -> bool {
        self.tone_envelope.is_active() || self.noise_envelope.is_active()
    }

    /// Generate next audio sample
    pub fn next_sample(&mut self) -> f32 {
        // Tonal component (body resonance)
        let tone1 = self.osc1.next_sample();
        let tone2 = self.osc2.next_sample();
        let tone = (tone1 + tone2) * 0.5;
        let tone_env = self.tone_envelope.next_sample();
        let tone_out = tone * tone_env;

        // Noise component (snare wires)
        let noise_sample = self.noise.next_sample();
        let filtered_noise = self.noise_filter.process(noise_sample);
        let noise_env = self.noise_envelope.next_sample();
        let noise_out = filtered_noise * noise_env;

        // Snap transient (stick attack)
        let snap_env = self.snap_envelope.next_sample();
        let snap_noise_sample = self.snap_noise.next_sample();
        let snap_transient = snap_noise_sample * snap_env * self.snap_amount;

        // Mix tone and noise using tone_mix parameter
        // tone_mix = 0.0 -> all noise, tone_mix = 1.0 -> all tone
        let body = tone_out * self.tone_mix + noise_out * (1.0 - self.tone_mix);

        // Add snap transient to the mix
        let mixed = body + snap_transient;

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
