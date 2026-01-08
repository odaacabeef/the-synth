use std::sync::atomic::AtomicI8;

use crate::instruments::poly16::parameters::AtomicF32;

/// CV output parameters - thread-safe for real-time audio
pub struct CVParameters {
    pub transpose: AtomicI8, // Transpose in semitones
    pub glide: AtomicF32,     // Glide time in seconds
}

impl CVParameters {
    pub fn new() -> Self {
        Self {
            transpose: AtomicI8::new(0),
            glide: AtomicF32::new(0.0),
        }
    }

    pub fn new_with_config(transpose: i8, glide: f32) -> Self {
        Self {
            transpose: AtomicI8::new(transpose),
            glide: AtomicF32::new(glide),
        }
    }
}

impl Default for CVParameters {
    fn default() -> Self {
        Self::new()
    }
}
