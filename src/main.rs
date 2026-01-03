mod audio;
mod config;
mod dsp;
mod midi;
mod types;
mod ui;

use anyhow::Result;
use clap::Parser;
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

use audio::{engine::SynthEngine, multi_engine::MultiEngineSynth, parameters::SynthParameters};
use config::SynthConfig;
use midi::handler::MidiHandler;
use ui::{app::App, events, render};

/// A 16-voice polyphonic synthesizer with ADSR envelope
#[derive(Parser, Debug)]
#[command(name = "the-synth")]
#[command(about = "16-voice polyphonic synthesizer", long_about = None)]
struct Args {
    /// Configuration file (YAML) - when provided, other args are ignored
    #[arg(short = 'c', long = "config")]
    config: Option<std::path::PathBuf>,

    /// MIDI input device (index or name)
    #[arg(short = 'm', long = "midi-device", required_unless_present_any = ["list_devices", "config"])]
    midi_input: Option<String>,

    /// MIDI channel (1-16 or 'omni' for all channels)
    #[arg(long = "midi-channel", default_value = "omni")]
    midi_channel: String,

    /// Audio output device (index or name)
    #[arg(short = 'a', long = "audio-device", required_unless_present_any = ["list_devices", "config"])]
    audio_output: Option<String>,

    /// Audio output channels (e.g., "0" for left, "1" for right, "0,1" for stereo)
    #[arg(long = "audio-channels", default_value = "0")]
    channels: String,

    /// List available devices and exit
    #[arg(short = 'l', long = "list")]
    list_devices: bool,
}

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

/// Find MIDI device index by name or index string
fn find_midi_device(devices: &[String], search: &str) -> Result<usize> {
    // Try to parse as index first
    if let Ok(index) = search.parse::<usize>() {
        if index < devices.len() {
            return Ok(index);
        } else {
            return Err(anyhow::anyhow!("MIDI device index {} out of range (0-{})", index, devices.len() - 1));
        }
    }

    // Search by name (case-insensitive substring match)
    let search_lower = search.to_lowercase();
    for (i, device) in devices.iter().enumerate() {
        if device.to_lowercase().contains(&search_lower) {
            return Ok(i);
        }
    }

    Err(anyhow::anyhow!("MIDI device '{}' not found", search))
}

/// Find audio device index by name or index string
fn find_audio_device(devices: &[String], search: &str) -> Result<usize> {
    // Try to parse as index first
    if let Ok(index) = search.parse::<usize>() {
        if index < devices.len() {
            return Ok(index);
        } else {
            return Err(anyhow::anyhow!("Audio device index {} out of range (0-{})", index, devices.len() - 1));
        }
    }

    // Search by name (case-insensitive substring match)
    let search_lower = search.to_lowercase();
    for (i, device) in devices.iter().enumerate() {
        if device.to_lowercase().contains(&search_lower) {
            return Ok(i);
        }
    }

    Err(anyhow::anyhow!("Audio device '{}' not found", search))
}

/// Parse MIDI channel from string ("omni" or "1"-"16")
fn parse_midi_channel(channel: &str) -> Result<Option<u8>> {
    if channel.eq_ignore_ascii_case("omni") || channel.eq_ignore_ascii_case("all") {
        return Ok(None);
    }

    let ch = channel.parse::<u8>()
        .map_err(|_| anyhow::anyhow!("Invalid MIDI channel '{}' (expected 1-16 or 'omni')", channel))?;

    if ch < 1 || ch > 16 {
        return Err(anyhow::anyhow!("MIDI channel {} out of range (1-16)", ch));
    }

    // Convert to 0-15 range
    Ok(Some(ch - 1))
}

/// Parse output channels specification ("all" or comma-separated channel indices like "0" or "0,1")
fn parse_output_channels(channels_str: &str, device_channels: usize) -> Result<Vec<usize>> {
    if channels_str.eq_ignore_ascii_case("all") {
        return Ok((0..device_channels).collect());
    }

    let mut channels = Vec::new();
    for ch_str in channels_str.split(',') {
        let ch_str = ch_str.trim();
        let ch = ch_str.parse::<usize>()
            .map_err(|_| anyhow::anyhow!("Invalid channel '{}' (expected number or 'all')", ch_str))?;

        if ch >= device_channels {
            return Err(anyhow::anyhow!("Channel {} out of range (device has {} channels: 0-{})",
                ch, device_channels, device_channels - 1));
        }

        channels.push(ch);
    }

    if channels.is_empty() {
        return Err(anyhow::anyhow!("No channels specified"));
    }

    Ok(channels)
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // List available MIDI and audio devices
    let midi_devices = MidiHandler::list_devices()?;
    let audio_devices = list_audio_devices()?;

    // Handle --list flag
    if args.list_devices {
        println!("Available MIDI Input Devices:");
        for (i, device) in midi_devices.iter().enumerate() {
            println!("  {}: {}", i, device);
        }
        println!("\nAvailable Audio Output Devices:");
        for (i, device) in audio_devices.iter().enumerate() {
            println!("  {}: {}", i, device);
        }
        return Ok(());
    }

    // Check if config mode is requested
    if let Some(config_path) = args.config {
        return run_config_mode(config_path, midi_devices, audio_devices);
    }

    // Legacy mode - single synthesizer
    // Find selected devices (unwrap is safe because args are required when not listing)
    let selected_midi_device = find_midi_device(&midi_devices, &args.midi_input.unwrap())?;
    let selected_audio_device = find_audio_device(&audio_devices, &args.audio_output.unwrap())?;
    let midi_channel = parse_midi_channel(&args.midi_channel)?;

    // Create shared parameters
    let parameters = Arc::new(SynthParameters::new());

    // Set MIDI channel in parameters
    let channel_value = midi_channel.unwrap_or(255);
    parameters.midi_channel.store(channel_value, std::sync::atomic::Ordering::Relaxed);

    // Create event channels
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let (voice_tx, voice_rx) = crossbeam_channel::unbounded::<[Option<u8>; 16]>();

    // Connect to selected MIDI device
    let _midi_handler = MidiHandler::new_with_device(event_tx, selected_midi_device, parameters.clone())?;

    // Initialize audio with selected device
    let host = cpal::default_host();
    let device = host
        .output_devices()?
        .nth(selected_audio_device)
        .ok_or_else(|| anyhow::anyhow!("Selected audio device not available"))?;

    let config = device.default_output_config()?;

    // Parse output channels
    let output_channels = parse_output_channels(&args.channels, config.channels() as usize)?;

    // Start audio stream
    let _stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            start_audio_stream::<f32>(&device, &config.into(), parameters.clone(), event_rx, voice_tx, output_channels)?
        }
        cpal::SampleFormat::I16 => {
            start_audio_stream::<i16>(&device, &config.into(), parameters.clone(), event_rx, voice_tx, output_channels)?
        }
        cpal::SampleFormat::U16 => {
            start_audio_stream::<u16>(&device, &config.into(), parameters.clone(), event_rx, voice_tx, output_channels)?
        }
        _ => panic!("Unsupported sample format"),
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app (starts in Synthesizer mode)
    let mut app = App::new(parameters.clone(), midi_channel);

    // Run synthesizer UI loop
    run_ui_loop(&mut terminal, &mut app, voice_rx)?;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Run in config mode - multi-instance synthesizers from YAML config
fn run_config_mode(
    config_path: std::path::PathBuf,
    midi_devices: Vec<String>,
    audio_devices: Vec<String>,
) -> Result<()> {
    // Load configuration
    println!("Loading configuration from: {}", config_path.display());
    let config = SynthConfig::load(&config_path)?;

    println!("Configuration loaded successfully!");
    println!("  Synth instances: {}", config.synths.len());
    println!("  MIDI input: {}", config.devices.midiin);
    println!("  Audio output: {}", config.devices.audioout);

    // Find devices
    let selected_midi_device = find_midi_device(&midi_devices, &config.devices.midiin)?;
    let selected_audio_device = find_audio_device(&audio_devices, &config.devices.audioout)?;

    println!("\nUsing devices:");
    println!("  MIDI: {} (index {})", midi_devices[selected_midi_device], selected_midi_device);
    println!("  Audio: {} (index {})", audio_devices[selected_audio_device], selected_audio_device);

    // Create main event channel for MIDI
    let (event_tx, event_rx) = crossbeam_channel::unbounded();

    // Create parameters and channel for each synth instance
    let mut instances = Vec::new();
    let mut all_parameters = Vec::new();

    for synth_config in &config.synths {
        let params = Arc::new(SynthParameters::default());

        // Set ADSR parameters
        params.attack.store(synth_config.attack, std::sync::atomic::Ordering::Relaxed);
        params.decay.store(synth_config.decay, std::sync::atomic::Ordering::Relaxed);
        params.sustain.store(synth_config.sustain, std::sync::atomic::Ordering::Relaxed);
        params.release.store(synth_config.release, std::sync::atomic::Ordering::Relaxed);

        // Set waveform
        let waveform = synth_config.waveform();
        params.waveform.store(waveform.to_u8(), std::sync::atomic::Ordering::Relaxed);

        // Create instance tuple: (name, parameters, midi_channel, audio_channel)
        // Note: audio_channel_index() converts 1-indexed config value to 0-indexed
        instances.push((
            synth_config.name.clone(),
            params.clone(),
            synth_config.midi_channel_filter(),
            synth_config.audio_channel_index(),
        ));

        all_parameters.push(params);
    }

    // Print synth instances
    println!("\nSynth instances:");
    for (name, _, midi_ch, audio_ch) in &instances {
        let midi_ch_str = if *midi_ch == 255 {
            "omni".to_string()
        } else {
            format!("{}", midi_ch + 1)
        };
        println!("  {} - MIDI CH{} → Audio CH{}", name, midi_ch_str, audio_ch);
    }

    // Connect to MIDI device (no channel filtering - filtering happens per-engine)
    let dummy_params = Arc::new(SynthParameters::default());
    let _midi_handler = MidiHandler::new_with_device(event_tx, selected_midi_device, dummy_params)?;

    println!("\nMIDI connected!");

    // Initialize audio with selected device
    let host = cpal::default_host();
    let device = host
        .output_devices()?
        .nth(selected_audio_device)
        .ok_or_else(|| anyhow::anyhow!("Selected audio device not available"))?;

    let audio_config = device.default_output_config()?;
    let num_channels = audio_config.channels() as usize;

    println!("Audio device has {} channels", num_channels);

    // Create voice state channels for UI
    let (voice_tx, voice_rx) = crossbeam_channel::unbounded::<Vec<[Option<u8>; 16]>>();

    // Start audio stream with multi-engine
    let _stream = match audio_config.sample_format() {
        cpal::SampleFormat::F32 => {
            start_multi_audio_stream::<f32>(&device, &audio_config.into(), instances, event_rx, voice_tx, num_channels)?
        }
        cpal::SampleFormat::I16 => {
            start_multi_audio_stream::<i16>(&device, &audio_config.into(), instances, event_rx, voice_tx, num_channels)?
        }
        cpal::SampleFormat::U16 => {
            start_multi_audio_stream::<u16>(&device, &audio_config.into(), instances, event_rx, voice_tx, num_channels)?
        }
        _ => panic!("Unsupported sample format"),
    };

    println!("Audio started!");
    println!("\nPress 'q' to quit, Page Up/Down to switch instances\n");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create multi-instance app
    let mut app = App::new_multi_instance(all_parameters, config.synths.clone());

    // Run UI loop
    run_multi_ui_loop(&mut terminal, &mut app, voice_rx)?;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Start audio stream with multi-engine support
fn start_multi_audio_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    instances: Vec<(String, Arc<SynthParameters>, u8, usize)>,
    event_rx: crossbeam_channel::Receiver<types::events::SynthEvent>,
    voice_tx: crossbeam_channel::Sender<Vec<[Option<u8>; 16]>>,
    num_channels: usize,
) -> Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let sample_rate = config.sample_rate as f32;

    // Create multi-engine synth
    let mut multi_engine = MultiEngineSynth::new(sample_rate, instances, event_rx);

    // Pre-allocate buffer for processing
    let mut temp_buffer = vec![0.0f32; 512 * num_channels];
    let mut frame_counter = 0u64;

    // Build output stream
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let frames = data.len() / num_channels;

            // Ensure temp buffer is large enough
            if temp_buffer.len() < frames * num_channels {
                temp_buffer.resize(frames * num_channels, 0.0);
            }

            // Process all engines and mix to multi-channel output
            multi_engine.process(&mut temp_buffer[..frames * num_channels], num_channels);

            // Convert to output sample format
            for (i, sample) in temp_buffer[..frames * num_channels].iter().enumerate() {
                data[i] = T::from_sample(*sample);
            }

            // Periodically send voice states to UI
            frame_counter += frames as u64;
            if frame_counter > 4410 {
                let _ = voice_tx.try_send(multi_engine.all_voice_states());
                frame_counter = 0;
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;

    stream.play()?;

    Ok(stream)
}

/// Run multi-instance UI loop
fn run_multi_ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    voice_rx: crossbeam_channel::Receiver<Vec<[Option<u8>; 16]>>,
) -> Result<()> {
    loop {
        // Update voice states from audio thread
        while let Ok(states) = voice_rx.try_recv() {
            app.update_multi_voice_states(states);
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

fn start_audio_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    parameters: Arc<SynthParameters>,
    event_rx: crossbeam_channel::Receiver<types::events::SynthEvent>,
    voice_tx: crossbeam_channel::Sender<[Option<u8>; 16]>,
    output_channels: Vec<usize>,
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

            // Write to specified output channels only
            for (frame_idx, frame) in data.chunks_mut(channels).enumerate() {
                let sample = temp_buffer[frame_idx];
                for (channel_idx, channel_sample) in frame.iter_mut().enumerate() {
                    if output_channels.contains(&channel_idx) {
                        *channel_sample = T::from_sample(sample);
                    } else {
                        *channel_sample = T::from_sample(0.0);
                    }
                }
            }

            // Periodically send voice states to UI (every ~100ms at 44.1kHz)
            frame_counter += frames as u64;
            if frame_counter > 4410 {
                let _ = voice_tx.try_send(engine.voice_states());
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
    voice_rx: crossbeam_channel::Receiver<[Option<u8>; 16]>,
) -> Result<()> {
    loop {
        // Update voice states from audio thread
        while let Ok(states) = voice_rx.try_recv() {
            app.voice_states = states;
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
