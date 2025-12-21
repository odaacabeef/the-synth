/// Supported waveform types for oscillator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Triangle,
    Sawtooth,
    Square,
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Sine
    }
}

impl Waveform {
    /// Convert to u8 for atomic storage
    pub fn to_u8(self) -> u8 {
        match self {
            Waveform::Sine => 0,
            Waveform::Triangle => 1,
            Waveform::Sawtooth => 2,
            Waveform::Square => 3,
        }
    }

    /// Convert from u8 from atomic storage
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Waveform::Triangle,
            2 => Waveform::Sawtooth,
            3 => Waveform::Square,
            _ => Waveform::Sine, // Default to Sine for invalid values
        }
    }

    /// Generate sample for this waveform at given phase (0.0 to 1.0)
    pub fn generate(&self, phase: f32) -> f32 {
        use std::f32::consts::PI;

        match self {
            Waveform::Sine => {
                // Sine wave: smooth periodic oscillation
                (phase * 2.0 * PI).sin()
            }
            Waveform::Triangle => {
                // Triangle wave: linear rise and fall
                // -1 to 1 over full cycle
                if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                }
            }
            Waveform::Sawtooth => {
                // Sawtooth wave: linear rise, sharp drop
                // -1 to 1 over full cycle
                2.0 * phase - 1.0
            }
            Waveform::Square => {
                // Square wave: alternating high/low
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_at_zero() {
        let wf = Waveform::Sine;
        assert!((wf.generate(0.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_triangle_range() {
        let wf = Waveform::Triangle;
        assert!((wf.generate(0.0) - (-1.0)).abs() < 0.001);
        assert!((wf.generate(0.25) - 0.0).abs() < 0.001);
        assert!((wf.generate(0.5) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_sawtooth_range() {
        let wf = Waveform::Sawtooth;
        assert!((wf.generate(0.0) - (-1.0)).abs() < 0.001);
        assert!((wf.generate(0.5) - 0.0).abs() < 0.001);
        assert!((wf.generate(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_square() {
        let wf = Waveform::Square;
        assert_eq!(wf.generate(0.0), 1.0);
        assert_eq!(wf.generate(0.49), 1.0);
        assert_eq!(wf.generate(0.5), -1.0);
        assert_eq!(wf.generate(0.99), -1.0);
    }
}
