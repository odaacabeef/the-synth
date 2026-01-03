use super::{hat::HiHat, kick::KickDrum, snare::SnareDrum, types::DrumType};

/// Unified drum voice that wraps all drum types
pub enum DrumVoice {
    Kick(KickDrum),
    Snare(SnareDrum),
    Hat(HiHat),
}

impl DrumVoice {
    /// Create new drum voice of the specified type
    pub fn new(drum_type: DrumType, sample_rate: f32) -> Self {
        match drum_type {
            DrumType::Kick => DrumVoice::Kick(KickDrum::new(sample_rate)),
            DrumType::Snare => DrumVoice::Snare(SnareDrum::new(sample_rate)),
            DrumType::Hat => DrumVoice::Hat(HiHat::new(sample_rate)),
        }
    }

    /// Trigger the drum
    pub fn trigger(&mut self) {
        match self {
            DrumVoice::Kick(k) => k.trigger(),
            DrumVoice::Snare(s) => s.trigger(),
            DrumVoice::Hat(h) => h.trigger(),
        }
    }

    /// Check if the drum is still active (generating audio)
    pub fn is_active(&self) -> bool {
        match self {
            DrumVoice::Kick(k) => k.is_active(),
            DrumVoice::Snare(s) => s.is_active(),
            DrumVoice::Hat(h) => h.is_active(),
        }
    }

    /// Generate next audio sample
    pub fn next_sample(&mut self) -> f32 {
        match self {
            DrumVoice::Kick(k) => k.next_sample(),
            DrumVoice::Snare(s) => s.next_sample(),
            DrumVoice::Hat(h) => h.next_sample(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drum_voice_kick() {
        let mut voice = DrumVoice::new(DrumType::Kick, 44100.0);
        assert!(!voice.is_active());

        voice.trigger();
        assert!(voice.is_active());

        let sample = voice.next_sample();
        assert!(sample.abs() >= 0.0);
    }

    #[test]
    fn test_drum_voice_snare() {
        let mut voice = DrumVoice::new(DrumType::Snare, 44100.0);
        voice.trigger();
        assert!(voice.is_active());
    }

    #[test]
    fn test_drum_voice_hat() {
        let mut voice = DrumVoice::new(DrumType::Hat, 44100.0);
        voice.trigger();
        assert!(voice.is_active());
    }
}
