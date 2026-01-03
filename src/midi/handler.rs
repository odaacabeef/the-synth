use anyhow::{anyhow, Result};
use crossbeam_channel::Sender;
use midir::{MidiInput, MidiInputConnection};
use std::sync::Arc;

use super::message::MidiMessage;
use crate::audio::parameters::SynthParameters;
use crate::types::events::SynthEvent;

/// MIDI input handler
/// Manages MIDI device connection and sends events to audio thread
pub struct MidiHandler {
    _connection: MidiInputConnection<()>,
}

impl MidiHandler {
    /// Connect to a specific MIDI input device by index
    pub fn new_with_device(
        event_tx: Sender<SynthEvent>,
        device_index: usize,
        _parameters: Arc<SynthParameters>,
    ) -> Result<Self> {
        let midi_in = MidiInput::new("the-synth-input")?;
        let ports = midi_in.ports();

        if device_index >= ports.len() {
            return Err(anyhow!("Device index out of range"));
        }

        let selected_port = &ports[device_index];

        // Connect to the selected port
        let connection = midi_in
            .connect(
                selected_port,
                "the-synth-input",
                move |_timestamp, bytes, _| {
                    // Parse MIDI message
                    let message = MidiMessage::parse(bytes);

                    // Convert to synth event (channel filtering happens at engine level)
                    if let Some(event) = message.to_synth_event() {
                        // Use try_send to avoid blocking MIDI thread
                        let _ = event_tx.try_send(event);
                    }
                },
                (),
            )
            .map_err(|e| anyhow!("Failed to connect to MIDI port: {}", e))?;

        Ok(Self {
            _connection: connection,
        })
    }

    /// Connect to MIDI input device and start receiving messages
    /// Auto-selects first available device (legacy method)
    #[allow(dead_code)]
    pub fn new(event_tx: Sender<SynthEvent>, _parameters: Arc<SynthParameters>) -> Result<Self> {
        let midi_in = MidiInput::new("the-synth-input")?;

        // Get available MIDI input ports
        let ports = midi_in.ports();

        if ports.is_empty() {
            return Err(anyhow!("No MIDI input devices found"));
        }

        // Try to find a keyboard or use the first available port
        let port = ports
            .iter()
            .position(|p| {
                midi_in
                    .port_name(p)
                    .ok()
                    .map(|n| {
                        let n_lower = n.to_lowercase();
                        n_lower.contains("keyboard")
                            || n_lower.contains("synth")
                            || n_lower.contains("piano")
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(0);

        let selected_port = &ports[port];

        // Connect to the selected port
        let connection = midi_in
            .connect(
                selected_port,
                "the-synth-input",
                move |_timestamp, bytes, _| {
                    // Parse MIDI message
                    let message = MidiMessage::parse(bytes);

                    // Convert to synth event (channel filtering happens at engine level)
                    if let Some(event) = message.to_synth_event() {
                        // Use try_send to avoid blocking MIDI thread
                        let _ = event_tx.try_send(event);
                    }
                },
                (),
            )
            .map_err(|e| anyhow!("Failed to connect to MIDI port: {}", e))?;

        Ok(Self {
            _connection: connection,
        })
    }

    /// List all available MIDI input devices
    #[allow(dead_code)]
    pub fn list_devices() -> Result<Vec<String>> {
        let midi_in = MidiInput::new("the-synth-list")?;
        let ports = midi_in.ports();

        let mut devices = Vec::new();
        for port in ports.iter() {
            if let Ok(name) = midi_in.port_name(port) {
                devices.push(name);
            }
        }

        Ok(devices)
    }
}
