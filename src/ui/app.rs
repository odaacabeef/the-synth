use crate::types::waveform::Waveform;
use std::sync::{atomic::Ordering, Arc};
use crate::audio::parameters::SynthParameters;

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    DeviceSelection,
    Synthesizer,
}

/// UI application state
/// Tracks all editable parameters and UI state
pub struct App {
    /// Current application mode
    pub mode: AppMode,
    /// Available MIDI devices
    pub midi_devices: Vec<String>,
    /// Selected device index
    pub selected_device_index: usize,
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
    /// Currently selected parameter for editing
    pub selected_param: Parameter,
    /// Number of active voices (updated from audio thread)
    pub active_voices: usize,
    /// Whether to quit the application
    pub should_quit: bool,
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
    /// Create new app with default values and MIDI device list
    pub fn new(parameters: Arc<SynthParameters>, midi_devices: Vec<String>) -> Self {
        Self {
            mode: AppMode::DeviceSelection,
            midi_devices,
            selected_device_index: 0,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
            waveform: Waveform::Sine,
            selected_param: Parameter::Attack,
            active_voices: 0,
            should_quit: false,
            parameters,
        }
    }

    /// Navigate to next device in list
    pub fn next_device(&mut self) {
        if !self.midi_devices.is_empty() {
            self.selected_device_index = (self.selected_device_index + 1) % self.midi_devices.len();
        }
    }

    /// Navigate to previous device in list
    pub fn prev_device(&mut self) {
        if !self.midi_devices.is_empty() {
            if self.selected_device_index == 0 {
                self.selected_device_index = self.midi_devices.len() - 1;
            } else {
                self.selected_device_index -= 1;
            }
        }
    }

    /// Confirm device selection and switch to synthesizer mode
    pub fn confirm_device(&mut self) {
        self.mode = AppMode::Synthesizer;
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
    }

    /// Mark app for quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
