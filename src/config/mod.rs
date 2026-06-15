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
    pub poly16: Vec<SynthInstanceConfig>,

    #[serde(default)]
    pub drums: Vec<DrumInstanceConfig>,

    #[serde(default)]
    pub cv: Vec<CVInstanceConfig>,

    #[serde(default)]
    pub es5: Vec<ES5InstanceConfig>,

    #[serde(default)]
    pub sampler: Vec<SamplerInstanceConfig>,
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
        // Allow empty poly16 if we have drums, CVs, ES5s, or samplers
        if self.poly16.is_empty()
            && self.drums.is_empty()
            && self.cv.is_empty()
            && self.es5.is_empty()
            && self.sampler.is_empty()
        {
            return Err(anyhow!(
                "Configuration must have at least one poly16, drum, CV, ES5, or sampler instance"
            ));
        }

        for (idx, synth) in self.poly16.iter().enumerate() {
            synth
                .validate()
                .with_context(|| format!("Invalid configuration for synth instance {}", idx))?;
        }

        for (idx, drum) in self.drums.iter().enumerate() {
            drum.validate()
                .with_context(|| format!("Invalid configuration for drum instance {}", idx))?;
        }

        for (idx, cv) in self.cv.iter().enumerate() {
            cv.validate()
                .with_context(|| format!("Invalid configuration for CV instance {}", idx))?;
        }

        for (idx, es5) in self.es5.iter().enumerate() {
            es5.validate()
                .with_context(|| format!("Invalid configuration for ES5 instance {}", idx))?;
        }

        for (idx, sampler) in self.sampler.iter().enumerate() {
            sampler
                .validate()
                .with_context(|| format!("Invalid configuration for sampler instance {}", idx))?;
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
        parse_note_str(&self.note)
    }
}

/// Parse a note string to a MIDI note number.
/// Examples: "c1" -> 24, "d1" -> 26, "gb1" -> 30
pub fn parse_note_str(note: &str) -> Result<u8> {
    let note_str = note.to_lowercase();

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

    let mut offset = 0i32;
    let mut octave_str = String::new();
    for ch in chars {
        match ch {
            '#' | 's' => offset = 1,
            'b' | 'f' => offset = -1,
            '0'..='9' | '-' => octave_str.push(ch),
            _ => return Err(anyhow!("Invalid character in note: {}", ch)),
        }
    }

    let octave: i32 = octave_str
        .parse()
        .map_err(|_| anyhow!("Invalid octave: {}", octave_str))?;

    let midi_note = (octave + 1) * 12 + base_note + offset;

    if midi_note < 0 || midi_note > 127 {
        return Err(anyhow!("Note out of range: {}", midi_note));
    }

    Ok(midi_note as u8)
}

/// CV instance configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CVInstanceConfig {
    pub midich: MidiChannelSpec,
    pub audioch: usize, // Gate CV output; pitch voices occupy audioch+1, audioch+2, ...

    #[serde(default = "default_cv_voices")]
    pub voices: usize, // Number of pitch CV voices (0 = gate only)

    #[serde(default)]
    pub note: Option<String>, // When set, only this note triggers CV output

    #[serde(default = "default_cv_transpose")]
    pub transpose: i8, // Transpose in semitones (-24 to +24)

    #[serde(default = "default_cv_glide")]
    pub glide: f32, // Glide time in seconds (0.0 to 2.0)
}

impl CVInstanceConfig {
    /// Validate this CV instance configuration
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

        // Validate transpose range
        if self.transpose < -24 || self.transpose > 24 {
            return Err(anyhow!("Transpose must be between -24 and +24 semitones"));
        }

        // Validate glide time
        if self.glide < 0.0 || self.glide > 2.0 {
            return Err(anyhow!("Glide must be between 0.0 and 2.0 seconds"));
        }

        // Validate note string if present
        if let Some(ref note) = self.note {
            parse_note_str(note).context("Invalid CV note filter")?;
        }

        Ok(())
    }

    /// Parse the note filter string to a MIDI note number, if set
    pub fn parse_note(&self) -> Option<Result<u8>> {
        self.note.as_deref().map(parse_note_str)
    }

    /// Get the 0-indexed audio channel for internal use (gate CV; pitch voices follow from audioch+1)
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
}

/// ES-5 output configuration - maps a MIDI note to one of the 6 gate outputs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ES5OutputConfig {
    pub note: String, // Note name like "c1", "d1"
}

/// ES-5 gate encoder instance configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ES5InstanceConfig {
    pub midich: MidiChannelSpec,
    pub audioch: usize, // Stereo pair: this channel and the next
    pub outputs: Vec<ES5OutputConfig>, // 1-6 gate outputs
}

impl ES5InstanceConfig {
    pub fn validate(&self) -> Result<()> {
        match &self.midich {
            MidiChannelSpec::Channel(ch) => {
                if *ch < 1 || *ch > 16 {
                    return Err(anyhow!("MIDI channel must be between 1 and 16"));
                }
            }
            MidiChannelSpec::Omni(_) => {}
        }

        if self.audioch < 1 {
            return Err(anyhow!("Audio channel must be >= 1 (channels are 1-indexed)"));
        }

        if self.outputs.is_empty() || self.outputs.len() > 6 {
            return Err(anyhow!("ES5 must have between 1 and 6 outputs"));
        }

        for (i, output) in self.outputs.iter().enumerate() {
            parse_note_str(&output.note)
                .with_context(|| format!("Invalid note for ES5 output {}", i + 1))?;
        }

        Ok(())
    }

    pub fn audio_channel_index(&self) -> usize {
        self.audioch.saturating_sub(1)
    }

    pub fn midi_channel_filter(&self) -> u8 {
        match &self.midich {
            MidiChannelSpec::Channel(ch) => ch - 1,
            MidiChannelSpec::Omni(_) => 255,
        }
    }

    pub fn parse_output_notes(&self) -> Result<Vec<u8>> {
        self.outputs
            .iter()
            .enumerate()
            .map(|(i, output)| {
                parse_note_str(&output.note)
                    .with_context(|| format!("Invalid note for ES5 output {}", i + 1))
            })
            .collect()
    }
}

/// Sampler instance configuration - plays a WAV file triggered by MIDI notes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SamplerInstanceConfig {
    pub midich: MidiChannelSpec,
    pub audioch: usize,

    /// Path to the WAV file (resolved relative to the config file's directory)
    pub file: String,

    /// Note that plays the sample at its recorded pitch (e.g. "c3")
    pub root: String,

    /// Optional [low, high] note span for melodic playback; must surround root.
    /// When omitted, only `root` triggers the sample.
    #[serde(default)]
    pub range: Option<Vec<String>>,

    #[serde(default = "default_sampler_voices")]
    pub voices: usize,

    #[serde(default = "default_sampler_gain")]
    pub gain: f32, // dB trim

    #[serde(default = "default_sampler_pitch")]
    pub pitch: i8, // semitone offset

    #[serde(default = "default_sampler_start")]
    pub start: f32, // 0..1 offset into the sample

    #[serde(default = "default_sampler_attack")]
    pub attack: f32, // fade-in seconds

    #[serde(default = "default_sampler_release")]
    pub release: f32, // fade-out seconds
}

impl SamplerInstanceConfig {
    /// Validate this sampler instance configuration
    pub fn validate(&self) -> Result<()> {
        // Validate MIDI channel (1-16)
        match &self.midich {
            MidiChannelSpec::Channel(ch) => {
                if *ch < 1 || *ch > 16 {
                    return Err(anyhow!("MIDI channel must be between 1 and 16"));
                }
            }
            MidiChannelSpec::Omni(_) => {}
        }

        // Validate audio channel (1-indexed, must be >= 1)
        if self.audioch < 1 {
            return Err(anyhow!("Audio channel must be >= 1 (channels are 1-indexed)"));
        }

        // Root note must parse
        let root = parse_note_str(&self.root).context("Invalid sampler root note")?;

        // Range, if present, must be two parseable notes that surround root
        if let Some(range) = &self.range {
            if range.len() != 2 {
                return Err(anyhow!(
                    "Sampler range must have exactly two notes: [low, high]"
                ));
            }
            let lo = parse_note_str(&range[0]).context("Invalid sampler range low note")?;
            let hi = parse_note_str(&range[1]).context("Invalid sampler range high note")?;
            if lo > hi {
                return Err(anyhow!("Sampler range low note must be <= high note"));
            }
            if root < lo || root > hi {
                return Err(anyhow!("Sampler range must surround root note"));
            }
        }

        if self.voices < 1 || self.voices > 16 {
            return Err(anyhow!("Sampler voices must be between 1 and 16"));
        }
        if self.gain < -60.0 || self.gain > 24.0 {
            return Err(anyhow!("Sampler gain must be between -60 and +24 dB"));
        }
        if self.pitch < -24 || self.pitch > 24 {
            return Err(anyhow!("Sampler pitch must be between -24 and +24 semitones"));
        }
        if self.start < 0.0 || self.start > 1.0 {
            return Err(anyhow!("Sampler start must be between 0.0 and 1.0"));
        }
        if self.attack < 0.0 || self.attack > 10.0 {
            return Err(anyhow!("Sampler attack must be between 0.0 and 10.0 seconds"));
        }
        if self.release < 0.0 || self.release > 10.0 {
            return Err(anyhow!("Sampler release must be between 0.0 and 10.0 seconds"));
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
            MidiChannelSpec::Channel(ch) => ch - 1,
            MidiChannelSpec::Omni(_) => 255,
        }
    }

    /// Parse the root note string to a MIDI note number
    pub fn parse_root(&self) -> Result<u8> {
        parse_note_str(&self.root)
    }

    /// Parse the optional range to (low, high) MIDI note numbers
    pub fn parse_range(&self) -> Result<Option<(u8, u8)>> {
        match &self.range {
            None => Ok(None),
            Some(range) => {
                if range.len() != 2 {
                    return Err(anyhow!(
                        "Sampler range must have exactly two notes: [low, high]"
                    ));
                }
                let lo = parse_note_str(&range[0])?;
                let hi = parse_note_str(&range[1])?;
                Ok(Some((lo, hi)))
            }
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

// Sampler defaults
fn default_sampler_voices() -> usize {
    1
}

fn default_sampler_gain() -> f32 {
    0.0
}

fn default_sampler_pitch() -> i8 {
    0
}

fn default_sampler_start() -> f32 {
    0.0
}

fn default_sampler_attack() -> f32 {
    0.0
}

fn default_sampler_release() -> f32 {
    0.05
}

// CV defaults
fn default_cv_voices() -> usize {
    1
}

fn default_cv_transpose() -> i8 {
    0
}

fn default_cv_glide() -> f32 {
    0.0
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

poly16:
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
        assert_eq!(config.poly16.len(), 2);
        assert_eq!(config.poly16[0].name, "Bass");
        assert_eq!(config.poly16[1].name, "Lead");
    }

    #[test]
    fn test_parse_omni_channel() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

poly16:
  - midich: omni
    audioch: 1
    wave: sine
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.poly16[0].midi_channel_filter(), 255);
    }

    #[test]
    fn test_validate_midi_channel_range() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

poly16:
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

poly16:
  - midich: 1
    audioch: 0
    attack: -1.0
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_parse_sampler_config() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

sampler:
  - file: "kick.wav"
    midich: 10
    audioch: 3
    root: "c2"
  - file: "piano.wav"
    midich: 1
    audioch: 4
    root: "c3"
    range: ["c2", "c5"]
    voices: 8
    gain: -3.0
"#;
        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.sampler.len(), 2);
        assert_eq!(config.sampler[0].voices, 1); // default
        assert!(config.sampler[0].range.is_none());
        assert_eq!(config.sampler[1].voices, 8);
        assert_eq!(config.sampler[1].parse_range().unwrap(), Some((36, 72)));
    }

    #[test]
    fn test_sampler_range_must_surround_root() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

sampler:
  - file: "x.wav"
    midich: 1
    audioch: 1
    root: "c5"
    range: ["c2", "c4"]
"#;
        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err()); // root c5 is above range c2..c4
    }

    #[test]
    fn test_sampler_defaults() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

sampler:
  - file: "x.wav"
    midich: 1
    audioch: 1
    root: "c3"
"#;
        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        let s = &config.sampler[0];
        assert_eq!(s.voices, 1);
        assert_eq!(s.gain, 0.0);
        assert_eq!(s.pitch, 0);
        assert_eq!(s.start, 0.0);
        assert_eq!(s.attack, 0.0);
        assert_eq!(s.release, 0.05);
        assert_eq!(s.parse_root().unwrap(), 48); // c3 = 48
    }

    #[test]
    fn test_defaults() {
        let yaml = r#"
devices:
  midiin: "test-midi"
  audioout: "test-audio"

poly16:
  - midich: 1
    audioch: 1
"#;

        let config: SynthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.poly16[0].name, "Untitled");
        assert_eq!(config.poly16[0].attack, 0.01);
        assert_eq!(config.poly16[0].decay, 0.1);
        assert_eq!(config.poly16[0].sustain, 0.7);
        assert_eq!(config.poly16[0].release, 0.1);
    }
}
