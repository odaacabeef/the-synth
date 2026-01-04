use std::sync::Arc;
use crate::audio::parameters::AtomicF32;
use super::types::DrumType;

/// Kick drum parameters - thread-safe for real-time audio
pub struct KickParameters {
    pub pitch_start: AtomicF32,
    pub pitch_end: AtomicF32,
    pub pitch_decay: AtomicF32,
    pub decay: AtomicF32,
    pub click: AtomicF32,
}

impl KickParameters {
    pub fn new() -> Self {
        Self {
            pitch_start: AtomicF32::new(150.0),
            pitch_end: AtomicF32::new(40.0),
            pitch_decay: AtomicF32::new(0.05),
            decay: AtomicF32::new(0.3),
            click: AtomicF32::new(0.3),
        }
    }
}

impl Default for KickParameters {
    fn default() -> Self {
        Self::new()
    }
}

/// Snare drum parameters - thread-safe for real-time audio
pub struct SnareParameters {
    pub tone_freq: AtomicF32,
    pub tone_mix: AtomicF32,
    pub decay: AtomicF32,
    pub snap: AtomicF32,
}

impl SnareParameters {
    pub fn new() -> Self {
        Self {
            tone_freq: AtomicF32::new(200.0),
            tone_mix: AtomicF32::new(0.3),
            decay: AtomicF32::new(0.15),
            snap: AtomicF32::new(0.5),
        }
    }
}

impl Default for SnareParameters {
    fn default() -> Self {
        Self::new()
    }
}

/// Hi-hat parameters - thread-safe for real-time audio
pub struct HatParameters {
    pub brightness: AtomicF32,
    pub decay: AtomicF32,
    pub metallic: AtomicF32,
}

impl HatParameters {
    pub fn new() -> Self {
        Self {
            brightness: AtomicF32::new(7000.0),
            decay: AtomicF32::new(0.05),
            metallic: AtomicF32::new(0.4),
        }
    }
}

impl Default for HatParameters {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified drum parameters enum (matches DrumType)
#[derive(Clone)]
pub enum DrumParameters {
    Kick(Arc<KickParameters>),
    Snare(Arc<SnareParameters>),
    Hat(Arc<HatParameters>),
}

impl DrumParameters {
    pub fn new(drum_type: DrumType) -> Self {
        match drum_type {
            DrumType::Kick => DrumParameters::Kick(Arc::new(KickParameters::new())),
            DrumType::Snare => DrumParameters::Snare(Arc::new(SnareParameters::new())),
            DrumType::Hat => DrumParameters::Hat(Arc::new(HatParameters::new())),
        }
    }
}
