/// VCA (Voltage Controlled Amplifier)
/// Applies amplitude modulation to audio signal
/// In modular synthesis, this is where envelope shapes the sound
pub struct VCA {
    /// Master gain/volume (0.0 to 1.0)
    gain: f32,
}

impl VCA {
    /// Create new VCA with default gain
    pub fn new() -> Self {
        Self { gain: 0.8 }
    }

    /// Set master gain level
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 1.0);
    }

    /// Process audio sample with envelope modulation
    /// signal: audio input from oscillator
    /// modulation: envelope level (0.0 to 1.0)
    pub fn process(&self, signal: f32, modulation: f32) -> f32 {
        signal * modulation * self.gain
    }
}

impl Default for VCA {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vca_full_modulation() {
        let vca = VCA::new();
        let result = vca.process(1.0, 1.0);
        assert_eq!(result, 0.8); // gain is 0.8
    }

    #[test]
    fn test_vca_zero_modulation() {
        let vca = VCA::new();
        let result = vca.process(1.0, 0.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_vca_half_modulation() {
        let vca = VCA::new();
        let result = vca.process(1.0, 0.5);
        assert_eq!(result, 0.4); // 1.0 * 0.5 * 0.8
    }
}
