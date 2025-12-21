/// ADSR Envelope Generator
/// Controls amplitude over time: Attack, Decay, Sustain, Release
/// Implemented as a state machine for efficient real-time processing
pub struct Envelope {
    state: EnvelopeState,
    /// Attack time in seconds
    attack: f32,
    /// Decay time in seconds
    decay: f32,
    /// Sustain level (0.0 to 1.0)
    sustain: f32,
    /// Release time in seconds
    release: f32,
    /// Current envelope output level (0.0 to 1.0)
    current_level: f32,
    /// Sample rate in Hz
    sample_rate: f32,
    /// Sample counter for timing
    sample_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EnvelopeState {
    Idle,
    Attack { start_sample: u64 },
    Decay { start_sample: u64 },
    Sustain,
    Release { start_sample: u64, release_level: f32 },
}

impl Envelope {
    /// Create new envelope with default ADSR values
    pub fn new(sample_rate: f32) -> Self {
        Self {
            state: EnvelopeState::Idle,
            attack: 0.01,   // 10ms attack
            decay: 0.1,     // 100ms decay
            sustain: 0.7,   // 70% sustain level
            release: 0.3,   // 300ms release
            current_level: 0.0,
            sample_rate,
            sample_count: 0,
        }
    }

    /// Set ADSR parameters
    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.attack = attack.max(0.001); // Minimum 1ms to avoid clicks
        self.decay = decay.max(0.001);
        self.sustain = sustain.clamp(0.0, 1.0);
        self.release = release.max(0.001);
    }

    /// Trigger note on - start attack phase
    pub fn note_on(&mut self) {
        self.state = EnvelopeState::Attack {
            start_sample: self.sample_count,
        };
    }

    /// Trigger note off - start release phase
    pub fn note_off(&mut self) {
        self.state = EnvelopeState::Release {
            start_sample: self.sample_count,
            release_level: self.current_level,
        };
    }

    /// Check if envelope is active (not idle)
    pub fn is_active(&self) -> bool {
        !matches!(self.state, EnvelopeState::Idle)
    }

    /// Generate next envelope sample
    pub fn next_sample(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Idle => {
                self.current_level = 0.0;
            }

            EnvelopeState::Attack { start_sample } => {
                let elapsed_samples = self.sample_count - start_sample;
                let attack_samples = (self.attack * self.sample_rate) as u64;

                if elapsed_samples >= attack_samples {
                    // Attack complete, move to decay
                    self.current_level = 1.0;
                    self.state = EnvelopeState::Decay {
                        start_sample: self.sample_count,
                    };
                } else {
                    // Linear ramp from 0 to 1
                    let progress = elapsed_samples as f32 / attack_samples as f32;
                    self.current_level = progress;
                }
            }

            EnvelopeState::Decay { start_sample } => {
                let elapsed_samples = self.sample_count - start_sample;
                let decay_samples = (self.decay * self.sample_rate) as u64;

                if elapsed_samples >= decay_samples {
                    // Decay complete, move to sustain
                    self.current_level = self.sustain;
                    self.state = EnvelopeState::Sustain;
                } else {
                    // Linear ramp from 1 to sustain level
                    let progress = elapsed_samples as f32 / decay_samples as f32;
                    self.current_level = 1.0 - progress * (1.0 - self.sustain);
                }
            }

            EnvelopeState::Sustain => {
                self.current_level = self.sustain;
            }

            EnvelopeState::Release {
                start_sample,
                release_level,
            } => {
                let elapsed_samples = self.sample_count - start_sample;
                let release_samples = (self.release * self.sample_rate) as u64;

                if elapsed_samples >= release_samples {
                    // Release complete, go idle
                    self.current_level = 0.0;
                    self.state = EnvelopeState::Idle;
                } else {
                    // Linear ramp from release_level to 0
                    let progress = elapsed_samples as f32 / release_samples as f32;
                    self.current_level = release_level * (1.0 - progress);
                }
            }
        }

        self.sample_count += 1;
        self.current_level
    }

    /// Reset envelope to idle state
    pub fn reset(&mut self) {
        self.state = EnvelopeState::Idle;
        self.current_level = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_starts_idle() {
        let env = Envelope::new(44100.0);
        assert!(!env.is_active());
    }

    #[test]
    fn test_note_on_triggers_attack() {
        let mut env = Envelope::new(44100.0);
        env.note_on();
        assert!(env.is_active());
        assert!(matches!(env.state, EnvelopeState::Attack { .. }));
    }

    #[test]
    fn test_attack_ramps_up() {
        let mut env = Envelope::new(44100.0);
        env.set_adsr(0.1, 0.1, 0.7, 0.3);
        env.note_on();

        let sample1 = env.next_sample();
        let sample2 = env.next_sample();

        assert!(sample2 > sample1);
        assert!(sample1 >= 0.0 && sample1 <= 1.0);
    }
}
