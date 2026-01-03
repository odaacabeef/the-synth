use serde::{Deserialize, Serialize};

/// Drum types supported by the drum synthesizer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DrumType {
    Kick,
    Snare,
    Hat,
}

impl DrumType {
    /// Get a human-readable name for the drum type
    pub fn name(&self) -> &'static str {
        match self {
            DrumType::Kick => "Kick",
            DrumType::Snare => "Snare",
            DrumType::Hat => "Hat",
        }
    }
}
