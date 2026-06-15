use crate::instruments::poly16::parameters::AtomicF32;

/// Sampler parameters - thread-safe for real-time audio.
///
/// The UI thread stores new values; the audio thread reads them once per
/// trigger. Pitch is stored in semitones; gain in decibels.
pub struct SamplerParameters {
    pub gain_db: AtomicF32,
    pub pitch: AtomicF32,
    pub start: AtomicF32,
    pub attack: AtomicF32,
    pub release: AtomicF32,
}

impl SamplerParameters {
    pub fn new() -> Self {
        Self {
            gain_db: AtomicF32::new(0.0),
            pitch: AtomicF32::new(0.0),
            start: AtomicF32::new(0.0),
            attack: AtomicF32::new(0.0),
            release: AtomicF32::new(0.05),
        }
    }

    pub fn new_with_config(gain_db: f32, pitch: f32, start: f32, attack: f32, release: f32) -> Self {
        Self {
            gain_db: AtomicF32::new(gain_db),
            pitch: AtomicF32::new(pitch),
            start: AtomicF32::new(start),
            attack: AtomicF32::new(attack),
            release: AtomicF32::new(release),
        }
    }
}

impl Default for SamplerParameters {
    fn default() -> Self {
        Self::new()
    }
}
