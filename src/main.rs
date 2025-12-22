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

/// List available audio output devices
fn list_audio_devices() -> Result<Vec<String>> {
    let host = cpal::default_host();

    // Collect all devices from iterator
    let mut devices: Vec<String> = host
        .output_devices()?
        .filter_map(|device| {
            device.description()
                .ok()
                .map(|desc| desc.name().to_string())
        })
        .collect();

    // Also try to get the default device explicitly
    if let Some(default_device) = host.default_output_device() {
        if let Ok(default_desc) = default_device.description() {
            let default_name = default_desc.name().to_string();
            // Add default device if not already in list
            if !devices.contains(&default_name) {
                devices.push(default_name);
            }
        }
    }

    if devices.is_empty() {
        return Err(anyhow::anyhow!("No audio output devices found"));
    }

    Ok(devices)
}

fn main() -> Result<()> {
    // List available MIDI and audio devices
    let midi_devices = MidiHandler::list_devices()?;
    let audio_devices = list_audio_devices()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create shared parameters
    let parameters = Arc::new(SynthParameters::new());

    // Create app with device lists
    let mut app = App::new(parameters.clone(), midi_devices.clone(), audio_devices.clone());

    // Main application loop - allows going back to device selection
    loop {
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

        // Get selected device indices
        let selected_midi_device = app.selected_midi_device;
        let selected_audio_device = app.selected_audio_device;

        // Create event channels
        let (event_tx, event_rx) = crossbeam_channel::unbounded();
        let (voice_tx, voice_rx) = crossbeam_channel::unbounded();
        let (waveform_tx, waveform_rx) = crossbeam_channel::unbounded();

        // Connect to selected MIDI device
        let _midi_handler = MidiHandler::new_with_device(event_tx, selected_midi_device, parameters.clone())?;

        // Initialize audio with selected device
        let host = cpal::default_host();
        let device = host
            .output_devices()?
            .nth(selected_audio_device)
            .expect("Selected audio device not available");

        let config = device.default_output_config()?;

        // Start audio stream
        let _stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                start_audio_stream::<f32>(&device, &config.into(), parameters.clone(), event_rx, voice_tx, waveform_tx)?
            }
            cpal::SampleFormat::I16 => {
                start_audio_stream::<i16>(&device, &config.into(), parameters.clone(), event_rx, voice_tx, waveform_tx)?
            }
            cpal::SampleFormat::U16 => {
                start_audio_stream::<u16>(&device, &config.into(), parameters.clone(), event_rx, voice_tx, waveform_tx)?
            }
            _ => panic!("Unsupported sample format"),
        };

        // Run synthesizer UI loop
        run_ui_loop(&mut terminal, &mut app, voice_rx, waveform_rx)?;

        // Check if should quit or go back to device selection
        if app.should_quit {
            // Restore terminal
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
            return Ok(());
        }

        // If back_to_device_selection is true, reset flag and loop continues
        if app.back_to_device_selection {
            app.back_to_device_selection = false;
            // Audio stream and MIDI handler will be dropped here, closing connections
            continue;
        }
    }
}

fn start_audio_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    parameters: Arc<SynthParameters>,
    event_rx: crossbeam_channel::Receiver<types::events::SynthEvent>,
    voice_tx: crossbeam_channel::Sender<usize>,
    waveform_tx: crossbeam_channel::Sender<Vec<f32>>,
) -> Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let sample_rate = config.sample_rate as f32;
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

            // Periodically send voice count and waveform samples to UI (every ~100ms at 44.1kHz)
            frame_counter += frames as u64;
            if frame_counter > 4410 {
                let _ = voice_tx.try_send(engine.active_voice_count());

                // Capture up to 2048 samples for oscilloscope visualization (~46ms at 44.1kHz)
                let sample_count = frames.min(2048);
                let samples: Vec<f32> = temp_buffer[..sample_count].to_vec();
                let _ = waveform_tx.try_send(samples);

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
    waveform_rx: crossbeam_channel::Receiver<Vec<f32>>,
) -> Result<()> {
    loop {
        // Update voice count from audio thread
        while let Ok(count) = voice_rx.try_recv() {
            app.active_voices = count;
        }

        // Update waveform samples from audio thread (accumulate in rolling buffer)
        while let Ok(samples) = waveform_rx.try_recv() {
            // Append new samples to rolling buffer
            app.waveform_samples.extend(samples);

            // Trim to keep only the last 500ms
            while app.waveform_samples.len() > app.max_samples {
                app.waveform_samples.pop_front();
            }
        }

        // Render UI
        terminal.draw(|f| render::render(f, app))?;

        // Handle events
        events::handle_events(app)?;

        // Check if should quit or go back
        if app.should_quit || app.back_to_device_selection {
            break;
        }

        // Small sleep to reduce CPU usage
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    Ok(())
}
