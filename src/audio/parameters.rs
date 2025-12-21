use std::sync::atomic::{AtomicU32, Ordering};

/// Thread-safe parameter storage using atomic operations
/// Allows real-time audio thread to read parameters without blocking
pub struct SynthParameters {
    /// Oscillator frequency in Hz (stored as f32 bits)
    pub frequency: AtomicF32,
    /// ADSR Attack time in seconds
    pub attack: AtomicF32,
    /// ADSR Decay time in seconds
    pub decay: AtomicF32,
    /// ADSR Sustain level (0.0 to 1.0)
    pub sustain: AtomicF32,
    /// ADSR Release time in seconds
    pub release: AtomicF32,
}

impl SynthParameters {
    pub fn new() -> Self {
        Self {
            frequency: AtomicF32::new(440.0), // A4 default
            attack: AtomicF32::new(0.01),     // 10ms
            decay: AtomicF32::new(0.1),       // 100ms
            sustain: AtomicF32::new(0.7),     // 70%
            release: AtomicF32::new(0.3),     // 300ms
        }
    }
}

impl Default for SynthParameters {
    fn default() -> Self {
        Self::new()
    }
}

/// Atomic f32 wrapper for lock-free parameter updates
pub struct AtomicF32 {
    storage: AtomicU32,
}

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        Self {
            storage: AtomicU32::new(value.to_bits()),
        }
    }

    pub fn load(&self, ordering: Ordering) -> f32 {
        f32::from_bits(self.storage.load(ordering))
    }

    pub fn store(&self, value: f32, ordering: Ordering) {
        self.storage.store(value.to_bits(), ordering);
    }
}
