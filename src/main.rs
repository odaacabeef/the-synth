mod audio;
mod midi;
mod types;
mod ui;

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    sync::Arc,
    time::Duration,
};

use audio::{engine::SynthEngine, parameters::SynthParameters};
use midi::handler::MidiHandler;
use ui::{app::{App, AppMode}, events, render};

fn main() -> Result<()> {
    // List available MIDI devices
    let midi_devices = MidiHandler::list_devices()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create shared parameters
    let parameters = Arc::new(SynthParameters::new());

    // Create app with device list
    let mut app = App::new(parameters.clone(), midi_devices.clone());

    // Device selection loop
    loop {
        // Render UI
        terminal.draw(|f| render::render(f, &app))?;

        // Handle events
        events::handle_events(&mut app)?;

        // Check if should quit
        if app.should_quit {
            // Restore terminal
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
            return Ok(());
        }

        // Check if device selected
        if app.mode == AppMode::Synthesizer {
            break;
        }

        std::thread::sleep(Duration::from_millis(16));
    }

    // Get selected device index
    let selected_device_index = app.selected_device_index;

    // Create event channels
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let (voice_tx, voice_rx) = crossbeam_channel::unbounded();

    // Connect to selected MIDI device
    let _midi_handler = MidiHandler::new_with_device(event_tx, selected_device_index, parameters.clone())?;

    // Initialize audio
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available");

    let config = device.default_output_config()?;

    // Start audio stream
    let _stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            start_audio_stream::<f32>(&device, &config.into(), parameters.clone(), event_rx, voice_tx)?
        }
        cpal::SampleFormat::I16 => {
            start_audio_stream::<i16>(&device, &config.into(), parameters.clone(), event_rx, voice_tx)?
        }
        cpal::SampleFormat::U16 => {
            start_audio_stream::<u16>(&device, &config.into(), parameters.clone(), event_rx, voice_tx)?
        }
        _ => panic!("Unsupported sample format"),
    };

    // Run synthesizer UI loop
    let result = run_ui_loop(&mut terminal, &mut app, voice_rx);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn start_audio_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    parameters: Arc<SynthParameters>,
    event_rx: crossbeam_channel::Receiver<types::events::SynthEvent>,
    voice_tx: crossbeam_channel::Sender<usize>,
) -> Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    // Create synth engine with MIDI event receiver
    let mut engine = SynthEngine::new(sample_rate, parameters, event_rx);

    // Pre-allocate buffer for processing
    let mut temp_buffer = vec![0.0f32; 512];
    let mut frame_counter = 0u64;

    // Build output stream
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            // Calculate number of frames
            let frames = data.len() / channels;

            // Ensure temp buffer is large enough
            if temp_buffer.len() < frames {
                temp_buffer.resize(frames, 0.0);
            }

            // Process audio (generate samples)
            engine.process(&mut temp_buffer[..frames]);

            // Convert and write to output (duplicate for all channels)
            for (frame_idx, frame) in data.chunks_mut(channels).enumerate() {
                let sample = temp_buffer[frame_idx];
                for channel_sample in frame.iter_mut() {
                    *channel_sample = T::from_sample(sample);
                }
            }

            // Periodically send voice count to UI (every ~100ms at 44.1kHz)
            frame_counter += frames as u64;
            if frame_counter > 4410 {
                let _ = voice_tx.try_send(engine.active_voice_count());
                frame_counter = 0;
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;

    // Start audio stream
    stream.play()?;

    Ok(stream)
}

fn run_ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    voice_rx: crossbeam_channel::Receiver<usize>,
) -> Result<()> {
    loop {
        // Update voice count from audio thread
        while let Ok(count) = voice_rx.try_recv() {
            app.active_voices = count;
        }

        // Render UI
        terminal.draw(|f| render::render(f, app))?;

        // Handle events
        events::handle_events(app)?;

        // Check if should quit
        if app.should_quit {
            break;
        }

        // Small sleep to reduce CPU usage
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    Ok(())
}
