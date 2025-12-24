use crate::types::waveform::Waveform;
use std::sync::{atomic::Ordering, Arc};
use crate::audio::parameters::SynthParameters;

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    DeviceSelection,
    Synthesizer,
}

/// Device selection focus (which section is currently selected)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSelectionFocus {
    MidiInput,
    MidiChannel,
    AudioOutput,
}

/// UI application state
/// Tracks all editable parameters and UI state
pub struct App {
    /// Current application mode
    pub mode: AppMode,
    /// Available MIDI input devices
    pub midi_devices: Vec<String>,
    /// Selected MIDI device index
    pub selected_midi_device: usize,
    /// Available audio output devices
    pub audio_devices: Vec<String>,
    /// Selected audio device index
    pub selected_audio_device: usize,
    /// Currently focused selection section
    pub device_selection_focus: DeviceSelectionFocus,
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
    /// Whether to go back to device selection
    pub back_to_device_selection: bool,
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
    /// Create new app with default values and device lists
    pub fn new(parameters: Arc<SynthParameters>, midi_devices: Vec<String>, audio_devices: Vec<String>) -> Self {
        Self {
            mode: AppMode::DeviceSelection,
            midi_devices,
            selected_midi_device: 0,
            audio_devices,
            selected_audio_device: 0,
            device_selection_focus: DeviceSelectionFocus::MidiInput,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
            waveform: Waveform::Sine,
            midi_channel: None, // Omni mode by default
            selected_param: Parameter::Attack,
            voice_states: [None; 16],
            should_quit: false,
            back_to_device_selection: false,
            show_help: false,
            parameters,
        }
    }

    /// Navigate to next device/option in currently focused section
    pub fn next_device(&mut self) {
        match self.device_selection_focus {
            DeviceSelectionFocus::MidiInput => {
                if !self.midi_devices.is_empty() {
                    self.selected_midi_device = (self.selected_midi_device + 1) % self.midi_devices.len();
                }
            }
            DeviceSelectionFocus::MidiChannel => {
                // Cycle through MIDI channels: Omni -> Ch1 -> Ch2 -> ... -> Ch16 -> Omni
                self.midi_channel = match self.midi_channel {
                    None => Some(0),           // Omni -> Ch1
                    Some(15) => None,          // Ch16 -> Omni
                    Some(ch) => Some(ch + 1),  // Ch(n) -> Ch(n+1)
                };
                self.sync_to_audio();
            }
            DeviceSelectionFocus::AudioOutput => {
                if !self.audio_devices.is_empty() {
                    self.selected_audio_device = (self.selected_audio_device + 1) % self.audio_devices.len();
                }
            }
        }
    }

    /// Navigate to previous device/option in currently focused section
    pub fn prev_device(&mut self) {
        match self.device_selection_focus {
            DeviceSelectionFocus::MidiInput => {
                if !self.midi_devices.is_empty() {
                    if self.selected_midi_device == 0 {
                        self.selected_midi_device = self.midi_devices.len() - 1;
                    } else {
                        self.selected_midi_device -= 1;
                    }
                }
            }
            DeviceSelectionFocus::MidiChannel => {
                // Cycle backwards through MIDI channels
                self.midi_channel = match self.midi_channel {
                    None => Some(15),          // Omni -> Ch16
                    Some(0) => None,           // Ch1 -> Omni
                    Some(ch) => Some(ch - 1),  // Ch(n) -> Ch(n-1)
                };
                self.sync_to_audio();
            }
            DeviceSelectionFocus::AudioOutput => {
                if !self.audio_devices.is_empty() {
                    if self.selected_audio_device == 0 {
                        self.selected_audio_device = self.audio_devices.len() - 1;
                    } else {
                        self.selected_audio_device -= 1;
                    }
                }
            }
        }
    }

    /// Cycle to next device selection section
    pub fn next_device_section(&mut self) {
        self.device_selection_focus = match self.device_selection_focus {
            DeviceSelectionFocus::MidiInput => DeviceSelectionFocus::MidiChannel,
            DeviceSelectionFocus::MidiChannel => DeviceSelectionFocus::AudioOutput,
            DeviceSelectionFocus::AudioOutput => DeviceSelectionFocus::MidiInput,
        };
    }

    /// Cycle to previous device selection section
    pub fn prev_device_section(&mut self) {
        self.device_selection_focus = match self.device_selection_focus {
            DeviceSelectionFocus::MidiInput => DeviceSelectionFocus::AudioOutput,
            DeviceSelectionFocus::MidiChannel => DeviceSelectionFocus::MidiInput,
            DeviceSelectionFocus::AudioOutput => DeviceSelectionFocus::MidiChannel,
        };
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

        // Convert Option<u8> to u8: None = 255 (omni), Some(ch) = ch
        let channel_value = self.midi_channel.unwrap_or(255);
        self.parameters.midi_channel.store(channel_value, Ordering::Relaxed);
    }

    /// Mark app for quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Go back to device selection screen
    pub fn go_back(&mut self) {
        self.back_to_device_selection = true;
        self.mode = AppMode::DeviceSelection;
        self.voice_states = [None; 16];
    }

    /// Toggle help screen visibility
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}
