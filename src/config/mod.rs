use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::instruments::drums::DrumType;
use crate::types::waveform::Waveform;

/// Top-level configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SynthConfig {
    pub devices: DeviceConfig,

    #[serde(default)]
    pub poly16s: Vec<SynthInstanceConfig>,

    #[serde(default)]
    pub drums: Vec<DrumInstanceConfig>,
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
        // Allow empty poly16s if we have drums
        if self.poly16s.is_empty() && self.drums.is_empty() {
            return Err(anyhow!(
                "Configuration must have at least one poly16 or drum instance"
            ));
        }

        for (idx, synth) in self.poly16s.iter().enumerate() {
            synth
                .validate()
                .with_context(|| format!("Invalid configuration for synth instance {}", idx))?;
        }

        for (idx, drum) in self.drums.iter().enumerate() {
            drum.validate()
                .with_context(|| format!("Invalid configuration for drum instance {}", idx))?;
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

/// Drum instance configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DrumInstanceConfig {
    pub midich: MidiChannelSpec,
    pub audioch: usize,
    #[serde(rename = "type")]
    pub drum_type: DrumType,
    pub note: String, // Note name like "c1", "d1", "gb1"

    // Kick parameters
    #[serde(default = "default_kick_pitch_start", rename = "pitchstart")]
    pub kick_pitch_start: f32,
    #[serde(default = "default_kick_pitch_end", rename = "pitchend")]
    pub kick_pitch_end: f32,
    #[serde(default = "default_kick_pitch_decay", rename = "pitchdecay")]
    pub kick_pitch_decay: f32,
    #[serde(default = "default_kick_decay", rename = "kdecay")]
    pub kick_decay: f32,
    #[serde(default = "default_kick_click", rename = "click")]
    pub kick_click: f32,

    // Snare parameters
    #[serde(default = "default_snare_tone_freq", rename = "tonefreq")]
    pub snare_tone_freq: f32,
    #[serde(default = "default_snare_tone_mix", rename = "tonemix")]
    pub snare_tone_mix: f32,
    #[serde(default = "default_snare_decay", rename = "sdecay")]
    pub snare_decay: f32,
    #[serde(default = "default_snare_snap", rename = "snap")]
    pub snare_snap: f32,

    // Hat parameters
    #[serde(default = "default_hat_brightness", rename = "brightness")]
    pub hat_brightness: f32,
    #[serde(default = "default_hat_decay", rename = "hdecay")]
    pub hat_decay: f32,
    #[serde(default = "default_hat_metallic", rename = "metallic")]
    pub hat_metallic: f32,
}

impl DrumInstanceConfig {
    /// Validate this drum instance configuration
    pub fn validate(&self) -> Result<()> {
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

        // Validate note string can be parsed
        self.parse_note()?;

        // Validate drum-specific parameters
        match self.drum_type {
            DrumType::Kick => {
                if self.kick_pitch_start < 100.0 || self.kick_pitch_start > 300.0 {
                    return Err(anyhow!("Kick pitch_start must be between 100 and 300 Hz"));
                }
                if self.kick_pitch_end < 30.0 || self.kick_pitch_end > 100.0 {
                    return Err(anyhow!("Kick pitch_end must be between 30 and 100 Hz"));
                }
                if self.kick_pitch_decay < 0.01 || self.kick_pitch_decay > 0.2 {
                    return Err(anyhow!("Kick pitch_decay must be between 0.01 and 0.2 seconds"));
                }
                if self.kick_decay < 0.1 || self.kick_decay > 1.0 {
                    return Err(anyhow!("Kick decay must be between 0.1 and 1.0 seconds"));
                }
                if self.kick_click < 0.0 || self.kick_click > 1.0 {
                    return Err(anyhow!("Kick click must be between 0.0 and 1.0"));
                }
            }
            DrumType::Snare => {
                if self.snare_tone_freq < 150.0 || self.snare_tone_freq > 300.0 {
                    return Err(anyhow!("Snare tone_freq must be between 150 and 300 Hz"));
                }
                if self.snare_tone_mix < 0.0 || self.snare_tone_mix > 1.0 {
                    return Err(anyhow!("Snare tone_mix must be between 0.0 and 1.0"));
                }
                if self.snare_decay < 0.05 || self.snare_decay > 0.5 {
                    return Err(anyhow!("Snare decay must be between 0.05 and 0.5 seconds"));
                }
                if self.snare_snap < 0.0 || self.snare_snap > 1.0 {
                    return Err(anyhow!("Snare snap must be between 0.0 and 1.0"));
                }
            }
            DrumType::Hat => {
                if self.hat_brightness < 5000.0 || self.hat_brightness > 12000.0 {
                    return Err(anyhow!("Hat brightness must be between 5000 and 12000 Hz"));
                }
                if self.hat_decay < 0.02 || self.hat_decay > 0.5 {
                    return Err(anyhow!("Hat decay must be between 0.02 and 0.5 seconds"));
                }
                if self.hat_metallic < 0.0 || self.hat_metallic > 1.0 {
                    return Err(anyhow!("Hat metallic must be between 0.0 and 1.0"));
                }
            }
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
            MidiChannelSpec::Omni(_) => 255,        // Omni mode
        }
    }

    /// Parse note string to MIDI note number
    /// Examples: "c1" -> 24, "d1" -> 26, "gb1" -> 30
    pub fn parse_note(&self) -> Result<u8> {
        let note_str = self.note.to_lowercase();

        // Parse note name (c, c#, d, etc.)
        let mut chars = note_str.chars();
        let note_char = chars
            .next()
            .ok_or_else(|| anyhow!("Empty note string"))?;

        let base_note = match note_char {
            'c' => 0,
            'd' => 2,
            'e' => 4,
            'f' => 5,
            'g' => 7,
            'a' => 9,
            'b' => 11,
            _ => return Err(anyhow!("Invalid note name: {}", note_char)),
        };

        // Check for sharp/flat
        let mut offset = 0;
        let mut octave_str = String::new();
        for ch in chars {
            match ch {
                '#' | 's' => offset = 1,  // Sharp
                'b' | 'f' => offset = -1, // Flat
                '0'..='9' | '-' => octave_str.push(ch),
                _ => return Err(anyhow!("Invalid character in note: {}", ch)),
            }
        }

        // Parse octave
        let octave: i32 = octave_str
            .parse()
            .map_err(|_| anyhow!("Invalid octave: {}", octave_str))?;

        // Calculate MIDI note: C-1 = 0, C0 = 12, C1 = 24, etc.
        let midi_note = (octave + 1) * 12 + base_note + offset;

        if midi_note < 0 || midi_note > 127 {
            return Err(anyhow!("Note out of range: {}", midi_note));
        }

        Ok(midi_note as u8)
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

// Kick drum defaults
fn default_kick_pitch_start() -> f32 {
    150.0
}

fn default_kick_pitch_end() -> f32 {
    40.0
}

fn default_kick_pitch_decay() -> f32 {
    0.05
}

fn default_kick_decay() -> f32 {
    0.3
}

fn default_kick_click() -> f32 {
    0.3
}

// Snare drum defaults
fn default_snare_tone_freq() -> f32 {
    200.0
}

fn default_snare_tone_mix() -> f32 {
    0.65
}

fn default_snare_decay() -> f32 {
    0.15
}

fn default_snare_snap() -> f32 {
    0.7
}

// Hi-hat defaults
fn default_hat_brightness() -> f32 {
    7000.0
}

fn default_hat_decay() -> f32 {
    0.05
}

fn default_hat_metallic() -> f32 {
    0.4
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

poly16s:
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
        assert_eq!(config.poly16s.len(), 2);
        assert_eq!(config.poly16s[0].name, "Bass");
        assert_eq!(config.poly16s[1].name, "Lead");
    }

    #[test]
    fn test_parse_omni_channel() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

poly16s:
  - midich: omni
    audioch: 1
    wave: sine
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.poly16s[0].midi_channel_filter(), 255);
    }

    #[test]
    fn test_validate_midi_channel_range() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

poly16s:
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

poly16s:
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

poly16s:
  - midich: 1
    audioch: 1
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.poly16s[0].name, "Untitled");
        assert_eq!(config.poly16s[0].attack, 0.01);
        assert_eq!(config.poly16s[0].decay, 0.1);
        assert_eq!(config.poly16s[0].sustain, 0.7);
        assert_eq!(config.poly16s[0].release, 0.1);
    }
}
