use crate::types::waveform::Waveform;
use std::sync::{atomic::Ordering, Arc};
use crate::audio::parameters::SynthParameters;


/// UI application state
/// Tracks all editable parameters and UI state
pub struct App {
    /// ADSR Attack time (0.001 to 2.0 seconds)
    pub attack: f32,
    /// ADSR Decay time (0.001 to 2.0 seconds)
    pub decay: f32,
    /// ADSR Sustain level (0.0 to 1.0)
    pub sustain: f32,
    /// ADSR Release time (0.001 to 5.0 seconds)
    pub release: f32,
    /// Current waveform
    pub waveform: Waveform,
    /// MIDI channel filter (None = omni/all channels, Some(0-15) = specific channel)
    pub midi_channel: Option<u8>,
    /// Currently selected parameter for editing
    pub selected_param: Parameter,
    /// Voice states: note number for each of 16 voices (None if idle)
    pub voice_states: [Option<u8>; 16],
    /// Whether to quit the application
    pub should_quit: bool,
    /// Whether to show help screen
    pub show_help: bool,
    /// Reference to shared parameters
    pub parameters: Arc<SynthParameters>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parameter {
    Attack,
    Decay,
    Sustain,
    Release,
    Waveform,
}

impl App {
    /// Create new app with default values
    pub fn new(parameters: Arc<SynthParameters>, midi_channel: Option<u8>) -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.4,
            release: 0.1,
            waveform: Waveform::Sine,
            midi_channel,
            selected_param: Parameter::Attack,
            voice_states: [None; 16],
            should_quit: false,
            show_help: false,
            parameters,
        }
    }

    /// Cycle to next parameter
    pub fn next_parameter(&mut self) {
        self.selected_param = match self.selected_param {
            Parameter::Attack => Parameter::Decay,
            Parameter::Decay => Parameter::Sustain,
            Parameter::Sustain => Parameter::Release,
            Parameter::Release => Parameter::Waveform,
            Parameter::Waveform => Parameter::Attack,
        };
    }

    /// Cycle to previous parameter
    pub fn prev_parameter(&mut self) {
        self.selected_param = match self.selected_param {
            Parameter::Attack => Parameter::Waveform,
            Parameter::Decay => Parameter::Attack,
            Parameter::Sustain => Parameter::Decay,
            Parameter::Release => Parameter::Sustain,
            Parameter::Waveform => Parameter::Release,
        };
    }

    /// Increase selected parameter value
    pub fn increase_value(&mut self) {
        match self.selected_param {
            Parameter::Attack => {
                self.attack = (self.attack + 0.01).min(2.0);
                self.sync_to_audio();
            }
            Parameter::Decay => {
                self.decay = (self.decay + 0.01).min(2.0);
                self.sync_to_audio();
            }
            Parameter::Sustain => {
                self.sustain = (self.sustain + 0.05).min(1.0);
                self.sync_to_audio();
            }
            Parameter::Release => {
                self.release = (self.release + 0.05).min(5.0);
                self.sync_to_audio();
            }
            Parameter::Waveform => {
                self.waveform = match self.waveform {
                    Waveform::Sine => Waveform::Triangle,
                    Waveform::Triangle => Waveform::Sawtooth,
                    Waveform::Sawtooth => Waveform::Square,
                    Waveform::Square => Waveform::Sine,
                };
                self.sync_to_audio();
            }
        }
    }

    /// Decrease selected parameter value
    pub fn decrease_value(&mut self) {
        match self.selected_param {
            Parameter::Attack => {
                self.attack = (self.attack - 0.01).max(0.001);
                self.sync_to_audio();
            }
            Parameter::Decay => {
                self.decay = (self.decay - 0.01).max(0.001);
                self.sync_to_audio();
            }
            Parameter::Sustain => {
                self.sustain = (self.sustain - 0.05).max(0.0);
                self.sync_to_audio();
            }
            Parameter::Release => {
                self.release = (self.release - 0.05).max(0.001);
                self.sync_to_audio();
            }
            Parameter::Waveform => {
                self.waveform = match self.waveform {
                    Waveform::Sine => Waveform::Square,
                    Waveform::Triangle => Waveform::Sine,
                    Waveform::Sawtooth => Waveform::Triangle,
                    Waveform::Square => Waveform::Sawtooth,
                };
                self.sync_to_audio();
            }
        }
    }

    /// Sync UI parameters to audio thread (via atomics)
    fn sync_to_audio(&self) {
        self.parameters.attack.store(self.attack, Ordering::Relaxed);
        self.parameters.decay.store(self.decay, Ordering::Relaxed);
        self.parameters.sustain.store(self.sustain, Ordering::Relaxed);
        self.parameters.release.store(self.release, Ordering::Relaxed);
        self.parameters.waveform.store(self.waveform.to_u8(), Ordering::Relaxed);

        // Convert Option<u8> to u8: None = 255 (omni), Some(ch) = ch
        let channel_value = self.midi_channel.unwrap_or(255);
        self.parameters.midi_channel.store(channel_value, Ordering::Relaxed);
    }

    /// Mark app for quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Toggle help screen visibility
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}
