use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

/// Thread-safe parameter storage using atomic operations
/// Allows real-time audio thread to read parameters without blocking
pub struct SynthParameters {
    /// ADSR Attack time in seconds
    pub attack: AtomicF32,
    /// ADSR Decay time in seconds
    pub decay: AtomicF32,
    /// ADSR Sustain level (0.0 to 1.0)
    pub sustain: AtomicF32,
    /// ADSR Release time in seconds
    pub release: AtomicF32,
    /// Waveform type (0=Sine, 1=Triangle, 2=Sawtooth, 3=Square)
    pub waveform: AtomicU8,
    /// MIDI channel filter (0-15=specific channel, 255=omni/all channels)
    pub midi_channel: AtomicU8,
}

impl SynthParameters {
    pub fn new() -> Self {
        Self {
            attack: AtomicF32::new(0.01),     // 10ms
            decay: AtomicF32::new(0.1),       // 100ms
            sustain: AtomicF32::new(0.7),     // 70%
            release: AtomicF32::new(0.3),     // 300ms
            waveform: AtomicU8::new(0),       // Sine
            midi_channel: AtomicU8::new(255), // Omni (all channels)
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
