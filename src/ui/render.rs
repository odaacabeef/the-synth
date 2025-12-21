use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use super::app::{App, AppMode, Parameter};

/// Render the TUI
pub fn render(frame: &mut Frame, app: &App) {
    match app.mode {
        AppMode::DeviceSelection => render_device_selection(frame, app),
        AppMode::Synthesizer => render_synthesizer(frame, app),
    }
}

/// Render device selection screen
fn render_device_selection(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(0),     // Device list
            Constraint::Length(5),  // Help (increased for better visibility)
        ])
        .split(frame.size());

    // Title
    let title = Paragraph::new("The Synth - MIDI Device Selection")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Device list
    let devices: Vec<ListItem> = app
        .midi_devices
        .iter()
        .enumerate()
        .map(|(i, device)| {
            let style = if i == app.selected_device_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == app.selected_device_index {
                "► "
            } else {
                "  "
            };
            ListItem::new(format!("{}{}", prefix, device)).style(style)
        })
        .collect();

    let list = List::new(devices).block(
        Block::default()
            .title("Select MIDI Input Device")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, chunks[1]);

    // Help
    let help_text = if app.midi_devices.is_empty() {
        vec![
            Line::from("No MIDI devices found!"),
            Line::from("Press Q to quit or connect a MIDI device and restart."),
        ]
    } else {
        vec![
            Line::from("Controls:"),
            Line::from("  ↑/↓: Select device  |  Enter: Confirm  |  Q/Esc: Quit"),
        ]
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(help, chunks[2]);
}

/// Render synthesizer screen
fn render_synthesizer(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(7),  // ADSR controls
            Constraint::Length(5),  // Waveform
            Constraint::Length(3),  // Voice meter
            Constraint::Length(4),  // Help text (fixed height)
        ])
        .split(frame.size());

    render_title(frame, chunks[0]);
    render_adsr_controls(frame, chunks[1], app);
    render_waveform_selector(frame, chunks[2], app);
    render_voice_meter(frame, chunks[3], app);
    render_help(frame, chunks[4]);
}

/// Render title bar
fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("The Synth - 8-Voice Polyphonic Synthesizer")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(title, area);
}

/// Render ADSR parameter controls
fn render_adsr_controls(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title("ADSR Envelope")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

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

    let color = if selected { Color::Yellow } else { Color::Green };
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
    let color = if selected { Color::Yellow } else { Color::White };

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

    let paragraph = Paragraph::new(waveforms)
        .block(Block::default().title("Waveform").borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render voice activity meter
fn render_voice_meter(frame: &mut Frame, area: Rect, app: &App) {
    let ratio = app.active_voices as f64 / 8.0;
    let label = format!("Active Voices: {}/8", app.active_voices);

    let gauge = Gauge::default()
        .block(Block::default().title("Polyphony").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan))
        .label(label)
        .ratio(ratio);

    frame.render_widget(gauge, area);
}

/// Render help text
fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from("Controls:"),
        Line::from("  Tab/←/→: Select parameter  |  ↑/↓ or +/-: Adjust value  |  Q/Esc: Quit"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(paragraph, area);
}
