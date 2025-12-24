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
    /// Available MIDI input devices
    pub midi_devices: Vec<String>,
    /// Selected MIDI device index
    pub selected_midi_device: usize,
    /// Available audio output devices
    pub audio_devices: Vec<String>,
    /// Selected audio device index
    pub selected_audio_device: usize,
    /// Currently focused selection (true = MIDI, false = Audio)
    pub selecting_midi: bool,
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
    /// Reverb wet/dry mix (0.0 to 1.0)
    pub reverb_mix: f32,
    /// Reverb room size (0.0 to 1.0)
    pub reverb_room_size: f32,
    /// Reverb damping (0.0 to 1.0)
    pub reverb_damping: f32,
    /// Currently selected parameter for editing
    pub selected_param: Parameter,
    /// Number of active voices (updated from audio thread)
    pub active_voices: usize,
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
    Channel,
    ReverbMix,
    ReverbRoomSize,
    ReverbDamping,
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
            selecting_midi: true, // Start with MIDI selection focused
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
            waveform: Waveform::Sine,
            midi_channel: None, // Omni mode by default
            reverb_mix: 0.0,
            reverb_room_size: 0.5,
            reverb_damping: 0.5,
            selected_param: Parameter::Attack,
            active_voices: 0,
            should_quit: false,
            back_to_device_selection: false,
            show_help: false,
            parameters,
        }
    }

    /// Navigate to next device in currently focused list
    pub fn next_device(&mut self) {
        if self.selecting_midi {
            if !self.midi_devices.is_empty() {
                self.selected_midi_device = (self.selected_midi_device + 1) % self.midi_devices.len();
            }
        } else {
            if !self.audio_devices.is_empty() {
                self.selected_audio_device = (self.selected_audio_device + 1) % self.audio_devices.len();
            }
        }
    }

    /// Navigate to previous device in currently focused list
    pub fn prev_device(&mut self) {
        if self.selecting_midi {
            if !self.midi_devices.is_empty() {
                if self.selected_midi_device == 0 {
                    self.selected_midi_device = self.midi_devices.len() - 1;
                } else {
                    self.selected_midi_device -= 1;
                }
            }
        } else {
            if !self.audio_devices.is_empty() {
                if self.selected_audio_device == 0 {
                    self.selected_audio_device = self.audio_devices.len() - 1;
                } else {
                    self.selected_audio_device -= 1;
                }
            }
        }
    }

    /// Toggle between MIDI and audio device selection
    pub fn toggle_device_focus(&mut self) {
        self.selecting_midi = !self.selecting_midi;
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
            Parameter::Release => Parameter::ReverbMix,
            Parameter::ReverbMix => Parameter::ReverbRoomSize,
            Parameter::ReverbRoomSize => Parameter::ReverbDamping,
            Parameter::ReverbDamping => Parameter::Waveform,
            Parameter::Waveform => Parameter::Channel,
            Parameter::Channel => Parameter::Attack,
        };
    }

    /// Cycle to previous parameter
    pub fn prev_parameter(&mut self) {
        self.selected_param = match self.selected_param {
            Parameter::Attack => Parameter::Channel,
            Parameter::Decay => Parameter::Attack,
            Parameter::Sustain => Parameter::Decay,
            Parameter::Release => Parameter::Sustain,
            Parameter::ReverbMix => Parameter::Release,
            Parameter::ReverbRoomSize => Parameter::ReverbMix,
            Parameter::ReverbDamping => Parameter::ReverbRoomSize,
            Parameter::Waveform => Parameter::ReverbDamping,
            Parameter::Channel => Parameter::Waveform,
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
            Parameter::Channel => {
                self.midi_channel = match self.midi_channel {
                    None => Some(0),           // Omni -> Ch1
                    Some(15) => None,          // Ch16 -> Omni
                    Some(ch) => Some(ch + 1),  // Ch(n) -> Ch(n+1)
                };
                self.sync_to_audio();
            }
            Parameter::ReverbMix => {
                self.reverb_mix = (self.reverb_mix + 0.05).min(1.0);
                self.sync_to_audio();
            }
            Parameter::ReverbRoomSize => {
                self.reverb_room_size = (self.reverb_room_size + 0.05).min(1.0);
                self.sync_to_audio();
            }
            Parameter::ReverbDamping => {
                self.reverb_damping = (self.reverb_damping + 0.05).min(1.0);
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
            Parameter::Channel => {
                self.midi_channel = match self.midi_channel {
                    None => Some(15),          // Omni -> Ch16
                    Some(0) => None,           // Ch1 -> Omni
                    Some(ch) => Some(ch - 1),  // Ch(n) -> Ch(n-1)
                };
                self.sync_to_audio();
            }
            Parameter::ReverbMix => {
                self.reverb_mix = (self.reverb_mix - 0.05).max(0.0);
                self.sync_to_audio();
            }
            Parameter::ReverbRoomSize => {
                self.reverb_room_size = (self.reverb_room_size - 0.05).max(0.0);
                self.sync_to_audio();
            }
            Parameter::ReverbDamping => {
                self.reverb_damping = (self.reverb_damping - 0.05).max(0.0);
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

        // Reverb parameters
        self.parameters.reverb_mix.store(self.reverb_mix, Ordering::Relaxed);
        self.parameters.reverb_room_size.store(self.reverb_room_size, Ordering::Relaxed);
        self.parameters.reverb_damping.store(self.reverb_damping, Ordering::Relaxed);
    }

    /// Mark app for quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Go back to device selection screen
    pub fn go_back(&mut self) {
        self.back_to_device_selection = true;
        self.mode = AppMode::DeviceSelection;
        self.active_voices = 0;
    }

    /// Toggle help screen visibility
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}
