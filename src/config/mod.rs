use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::types::waveform::Waveform;

/// Top-level configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SynthConfig {
    pub devices: DeviceConfig,
    pub synths: Vec<SynthInstanceConfig>,
}

impl SynthConfig {
    /// Load configuration from a YAML file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: SynthConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML config: {}", path.display()))?;

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.synths.is_empty() {
            return Err(anyhow!("Configuration must have at least one synth instance"));
        }

        for (idx, synth) in self.synths.iter().enumerate() {
            synth.validate()
                .with_context(|| format!("Invalid configuration for synth instance {}", idx))?;
        }

        Ok(())
    }
}

/// Device configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceConfig {
    pub midiin: String,
    pub audioout: String,
}

/// Individual synthesizer instance configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SynthInstanceConfig {
    #[serde(default = "default_name", skip_serializing_if = "String::is_empty")]
    pub name: String,

    pub midich: MidiChannelSpec,
    pub audioch: usize,

    #[serde(default = "default_attack")]
    pub attack: f32,

    #[serde(default = "default_decay")]
    pub decay: f32,

    #[serde(default = "default_sustain")]
    pub sustain: f32,

    #[serde(default = "default_release")]
    pub release: f32,

    #[serde(default)]
    pub wave: WaveformSpec,
}

impl SynthInstanceConfig {
    /// Validate this instance configuration
    pub fn validate(&self) -> Result<()> {
        // Validate ADSR envelope parameters
        if self.attack < 0.0 || self.attack > 10.0 {
            return Err(anyhow!("Attack must be between 0.0 and 10.0 seconds"));
        }
        if self.decay < 0.0 || self.decay > 10.0 {
            return Err(anyhow!("Decay must be between 0.0 and 10.0 seconds"));
        }
        if self.sustain < 0.0 || self.sustain > 1.0 {
            return Err(anyhow!("Sustain must be between 0.0 and 1.0"));
        }
        if self.release < 0.0 || self.release > 10.0 {
            return Err(anyhow!("Release must be between 0.0 and 10.0 seconds"));
        }

        // Validate MIDI channel (1-16)
        match &self.midich {
            MidiChannelSpec::Channel(ch) => {
                if *ch < 1 || *ch > 16 {
                    return Err(anyhow!("MIDI channel must be between 1 and 16"));
                }
            }
            MidiChannelSpec::Omni(_) => {
                // Always valid
            }
        }

        // Validate audio channel (1-indexed, must be >= 1)
        if self.audioch < 1 {
            return Err(anyhow!("Audio channel must be >= 1 (channels are 1-indexed)"));
        }

        Ok(())
    }

    /// Get the 0-indexed audio channel for internal use
    pub fn audio_channel_index(&self) -> usize {
        self.audioch.saturating_sub(1)
    }

    /// Get the MIDI channel filter value (0-15 for specific channel, 255 for omni)
    pub fn midi_channel_filter(&self) -> u8 {
        match &self.midich {
            MidiChannelSpec::Channel(ch) => ch - 1, // Convert 1-16 to 0-15
            MidiChannelSpec::Omni(_) => 255, // Omni mode
        }
    }

    /// Convert waveform spec to Waveform enum
    pub fn waveform(&self) -> Waveform {
        match &self.wave {
            WaveformSpec::Sine => Waveform::Sine,
            WaveformSpec::Triangle => Waveform::Triangle,
            WaveformSpec::Sawtooth => Waveform::Sawtooth,
            WaveformSpec::Square => Waveform::Square,
        }
    }
}

/// MIDI channel specification - either a specific channel (1-16) or omni
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MidiChannelSpec {
    Channel(u8),
    Omni(String), // "omni" or "all"
}

/// Waveform specification for deserialization
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WaveformSpec {
    Sine,
    Triangle,
    Sawtooth,
    Square,
}

impl Default for WaveformSpec {
    fn default() -> Self {
        WaveformSpec::Sine
    }
}

// Default value functions for serde
fn default_name() -> String {
    "Untitled".to_string()
}

fn default_attack() -> f32 {
    0.01
}

fn default_decay() -> f32 {
    0.1
}

fn default_sustain() -> f32 {
    0.7
}

fn default_release() -> f32 {
    0.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

synths:
  - name: "Bass"
    midich: 1
    audioch: 1
    attack: 0.01
    decay: 0.1
    sustain: 0.4
    release: 0.1
    wave: sine
  - name: "Lead"
    midich: 2
    audioch: 2
    attack: 0.001
    decay: 0.05
    sustain: 0.7
    release: 0.2
    wave: sawtooth
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.synths.len(), 2);
        assert_eq!(config.synths[0].name, "Bass");
        assert_eq!(config.synths[1].name, "Lead");
    }

    #[test]
    fn test_parse_omni_channel() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

synths:
  - midich: omni
    audioch: 1
    wave: sine
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.synths[0].midi_channel_filter(), 255);
    }

    #[test]
    fn test_validate_midi_channel_range() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

synths:
  - midich: 17
    audioch: 0
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_adsr_ranges() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

synths:
  - midich: 1
    audioch: 0
    attack: -1.0
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_defaults() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

synths:
  - midich: 1
    audioch: 1
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.synths[0].name, "Untitled");
        assert_eq!(config.synths[0].attack, 0.01);
        assert_eq!(config.synths[0].decay, 0.1);
        assert_eq!(config.synths[0].sustain, 0.7);
        assert_eq!(config.synths[0].release, 0.1);
    }
}
