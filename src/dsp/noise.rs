/// White noise generator using linear congruential generator (LCG)
/// Fast, deterministic pseudo-random noise for real-time audio
pub struct NoiseGenerator {
    state: u32,
}

impl NoiseGenerator {
    /// Create new noise generator with default seed
    pub fn new() -> Self {
        Self {
            state: 0x12345678, // Default seed value
        }
    }

    /// Create new noise generator with custom seed
    #[allow(dead_code)]
    pub fn new_with_seed(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Generate next white noise sample in range [-1.0, 1.0]
    pub fn next_sample(&mut self) -> f32 {
        // LCG: next = (a * current + c) mod m
        // Using constants from Numerical Recipes
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);

        // Convert to float in range [-1.0, 1.0]
        let normalized = (self.state as f32 / u32::MAX as f32) * 2.0 - 1.0;
        normalized
    }
}

impl Default for NoiseGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_in_range() {
        let mut noise = NoiseGenerator::new();

        for _ in 0..1000 {
            let sample = noise.next_sample();
            assert!(sample >= -1.0 && sample <= 1.0, "Sample out of range: {}", sample);
        }
    }

    #[test]
    fn test_noise_has_variance() {
        let mut noise = NoiseGenerator::new();
        let mut samples = Vec::new();

        for _ in 0..100 {
            samples.push(noise.next_sample());
        }

        // Calculate mean
        let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;

        // Calculate variance
        let variance: f32 = samples.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f32>() / samples.len() as f32;

        // Noise should have significant variance (not all zeros or constant)
        assert!(variance > 0.1, "Noise variance too low: {}", variance);
    }

    #[test]
    fn test_deterministic_with_seed() {
        let mut noise1 = NoiseGenerator::new_with_seed(42);
        let mut noise2 = NoiseGenerator::new_with_seed(42);

        for _ in 0..10 {
            assert_eq!(noise1.next_sample(), noise2.next_sample());
        }
    }
}
