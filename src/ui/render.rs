use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use super::app::{App, AppMode, Parameter, DeviceSelectionFocus};

/// Render the TUI
pub fn render(frame: &mut Frame, app: &App) {
    // Show help screen if toggled
    if app.show_help {
        match app.mode {
            AppMode::DeviceSelection => render_device_selection_help(frame),
            AppMode::Synthesizer => render_synthesizer_help(frame),
        }
    } else {
        match app.mode {
            AppMode::DeviceSelection => render_device_selection(frame, app),
            AppMode::Synthesizer => render_synthesizer(frame, app),
        }
    }
}

/// Render device selection screen
fn render_device_selection(frame: &mut Frame, app: &App) {
    // Calculate heights based on number of devices (+3 for borders and title)
    let midi_height = (app.midi_devices.len() + 3).min(15) as u16; // Cap at 15 lines
    let audio_height = (app.audio_devices.len() + 3).min(15) as u16; // Cap at 15 lines

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(midi_height), // MIDI devices (sized to content)
            Constraint::Length(4),           // MIDI channel selector
            Constraint::Length(audio_height),// Audio devices (sized to content)
        ])
        .split(frame.size());

    // MIDI device list
    let midi_devices: Vec<ListItem> = app
        .midi_devices
        .iter()
        .enumerate()
        .map(|(i, device)| {
            let style = if app.device_selection_focus == DeviceSelectionFocus::MidiInput && i == app.selected_midi_device {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == app.selected_midi_device {
                "► "
            } else {
                "  "
            };
            ListItem::new(format!("{}{}", prefix, device)).style(style)
        })
        .collect();

    let midi_title = "MIDI Input";
    let midi_border_style = if app.device_selection_focus == DeviceSelectionFocus::MidiInput {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let midi_list = List::new(midi_devices).block(
        Block::default()
            .title(midi_title)
            .borders(Borders::ALL)
            .border_style(midi_border_style),
    );
    frame.render_widget(midi_list, chunks[0]);

    // MIDI channel selector
    let channel_text = match app.midi_channel {
        None => "MIDI Channel: Omni (All)".to_string(),
        Some(ch) => format!("MIDI Channel: {}", ch + 1), // Display as 1-16
    };

    let channel_style = if app.device_selection_focus == DeviceSelectionFocus::MidiChannel {
        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let channel_border_style = if app.device_selection_focus == DeviceSelectionFocus::MidiChannel {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let channel_paragraph = Paragraph::new(channel_text)
        .style(channel_style)
        .alignment(Alignment::Center)
        .block(Block::default()
            .title("MIDI Channel")
            .borders(Borders::ALL)
            .border_style(channel_border_style));
    frame.render_widget(channel_paragraph, chunks[1]);

    // Audio device list
    let audio_devices: Vec<ListItem> = app
        .audio_devices
        .iter()
        .enumerate()
        .map(|(i, device)| {
            let style = if app.device_selection_focus == DeviceSelectionFocus::AudioOutput && i == app.selected_audio_device {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == app.selected_audio_device {
                "► "
            } else {
                "  "
            };
            ListItem::new(format!("{}{}", prefix, device)).style(style)
        })
        .collect();

    let audio_title = "Audio Output";
    let audio_border_style = if app.device_selection_focus == DeviceSelectionFocus::AudioOutput {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let audio_list = List::new(audio_devices).block(
        Block::default()
            .title(audio_title)
            .borders(Borders::ALL)
            .border_style(audio_border_style),
    );
    frame.render_widget(audio_list, chunks[2]);
}

/// Render synthesizer screen
fn render_synthesizer(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // ADSR controls
            Constraint::Length(5),  // Waveform
            Constraint::Length(3),  // Voice meter
        ])
        .split(frame.size());

    render_adsr_controls(frame, chunks[0], app);
    render_waveform_selector(frame, chunks[1], app);
    render_voice_meter(frame, chunks[2], app);
}

/// Render ADSR parameter controls
fn render_adsr_controls(frame: &mut Frame, area: Rect, app: &App) {
    let is_active = matches!(app.selected_param,
        Parameter::Attack | Parameter::Decay | Parameter::Sustain | Parameter::Release);
    let border_color = if is_active { Color::Magenta } else { Color::DarkGray };

    let block = Block::default()
        .title("ADSR Envelope")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout for 4 parameters
    let param_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner);

    render_parameter(frame, param_chunks[0], "Attack", app.attack, 0.001, 2.0, "s", app.selected_param == Parameter::Attack);
    render_parameter(frame, param_chunks[1], "Decay", app.decay, 0.001, 2.0, "s", app.selected_param == Parameter::Decay);
    render_parameter(frame, param_chunks[2], "Sustain", app.sustain, 0.0, 1.0, "", app.selected_param == Parameter::Sustain);
    render_parameter(frame, param_chunks[3], "Release", app.release, 0.001, 5.0, "s", app.selected_param == Parameter::Release);
}

/// Render a single parameter with gauge
fn render_parameter(
    frame: &mut Frame,
    area: Rect,
    name: &str,
    value: f32,
    min: f32,
    max: f32,
    unit: &str,
    selected: bool,
) {
    let ratio = ((value - min) / (max - min)).clamp(0.0, 1.0);

    let color = if selected { Color::Magenta } else { Color::DarkGray };
    let style = if selected {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color)
    };

    // Label with value
    let label = if unit.is_empty() {
        format!("{}: {:.2}", name, value)
    } else {
        format!("{}: {:.3}{}", name, value, unit)
    };

    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(style)
        .label(label)
        .ratio(ratio as f64);

    frame.render_widget(gauge, area);
}

/// Render waveform selector
fn render_waveform_selector(frame: &mut Frame, area: Rect, app: &App) {
    let selected = app.selected_param == Parameter::Waveform;
    let color = if selected { Color::Magenta } else { Color::DarkGray };

    let waveform_text = format!("Waveform: {:?}", app.waveform);

    let style = if selected {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color)
    };

    let waveforms = vec![
        Line::from(Span::styled(waveform_text, style)),
        Line::from(""),
        Line::from("Quick select: 1=Sine 2=Triangle 3=Sawtooth 4=Square"),
    ];

    let border_color = if selected { Color::Magenta } else { Color::DarkGray };

    let paragraph = Paragraph::new(waveforms)
        .block(Block::default()
            .title("Waveform")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Convert MIDI note number to note name (e.g., 60 -> "C4")
fn midi_note_to_name(note: u8) -> String {
    const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (note / 12) as i32 - 1;
    let note_name = NOTE_NAMES[(note % 12) as usize];
    format!("{}{}", note_name, octave)
}

/// Render voice activity display (16 slots showing note names or "-")
fn render_voice_meter(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title("Polyphony (16 Voices)")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build the voice display string
    let mut voice_display = String::new();
    for (i, voice_state) in app.voice_states.iter().enumerate() {
        if i > 0 {
            voice_display.push(' ');
        }

        match voice_state {
            Some(note) => {
                // Show note name (e.g., "C4", "A#3")
                let note_name = midi_note_to_name(*note);
                voice_display.push_str(&note_name);
            }
            None => {
                // Show placeholder with padding to match note name width
                voice_display.push_str("--");
            }
        }
    }

    let paragraph = Paragraph::new(voice_display)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, inner);
}

/// Render device selection help screen
fn render_device_selection_help(frame: &mut Frame) {
    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("Device Selection - Controls", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  h, l, ←, →     Switch between MIDI Input, MIDI Channel, and Audio Output"),
        Line::from("  k, j, ↑, ↓     Navigate/change selection in the focused section"),
        Line::from("  Enter          Confirm selection and start synthesizer"),
        Line::from("  ?              Toggle this help screen"),
        Line::from("  q, Ctrl+C      Quit application"),
        Line::from(""),
        Line::from(Span::styled("Usage", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  1. Use h/l or arrow keys to switch between sections"),
        Line::from("  2. The focused section has a magenta border"),
        Line::from("  3. Use k/j or arrow keys to select device or change MIDI channel"),
        Line::from("  4. Press Enter to confirm and start the synthesizer"),
        Line::from(""),
        Line::from(Span::styled("Press ? to close this help screen", Style::default().fg(Color::Gray))),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)))
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, frame.size());
}

/// Render synthesizer help screen
fn render_synthesizer_help(frame: &mut Frame) {
    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("Synthesizer - Controls", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  h, l, ←, →     Navigate between parameters (Attack/Decay/Sustain/Release/Waveform)"),
        Line::from("  k, j, ↑, ↓     Adjust the selected parameter value"),
        Line::from("  1              Set waveform to Sine"),
        Line::from("  2              Set waveform to Triangle"),
        Line::from("  3              Set waveform to Sawtooth"),
        Line::from("  4              Set waveform to Square"),
        Line::from("  Esc            Return to device selection screen"),
        Line::from("  ?              Toggle this help screen"),
        Line::from("  q, Ctrl+C      Quit application"),
        Line::from(""),
        Line::from(Span::styled("Parameters", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  Attack         Envelope attack time (0.001s - 2.0s)"),
        Line::from("  Decay          Envelope decay time (0.001s - 2.0s)"),
        Line::from("  Sustain        Envelope sustain level (0.0 - 1.0)"),
        Line::from("  Release        Envelope release time (0.001s - 5.0s)"),
        Line::from("  Waveform       Oscillator waveform (Sine/Triangle/Sawtooth/Square)"),
        Line::from(""),
        Line::from(Span::styled("Display", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  Voice Meter    Active voice count (16-voice polyphony)"),
        Line::from(""),
        Line::from(Span::styled("Press ? to close this help screen", Style::default().fg(Color::Gray))),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)))
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, frame.size());
}
