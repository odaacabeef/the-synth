mod audio;
mod midi;
mod types;

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;

use audio::{engine::SynthEngine, parameters::SynthParameters};
use midi::handler::MidiHandler;

fn main() -> Result<()> {
    println!("The Synth - Phase 3: MIDI Control");
    println!("==================================");

    // Create event channel for MIDI → Audio communication
    let (event_tx, event_rx) = crossbeam_channel::bounded(256);

    // Initialize MIDI input
    println!("Initializing MIDI...");
    let _midi_handler = MidiHandler::new(event_tx)?;

    // Initialize audio host
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available");

    println!("\nOutput device: {}", device.name()?);

    let config = device.default_output_config()?;
    println!("Default output config: {:?}", config);

    // Create shared parameters
    let parameters = Arc::new(SynthParameters::new());

    // Build audio stream based on sample format
    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), parameters, event_rx)?,
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), parameters, event_rx)?,
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), parameters, event_rx)?,
        _ => panic!("Unsupported sample format"),
    }

    Ok(())
}

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    parameters: Arc<SynthParameters>,
    event_rx: crossbeam_channel::Receiver<types::events::SynthEvent>,
) -> Result<()>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    println!("\nSample rate: {} Hz", sample_rate);
    println!("Channels: {}", channels);
    println!("\nSynthesizer ready! Play notes on your MIDI keyboard.");
    println!("ADSR: Attack=10ms, Decay=100ms, Sustain=70%, Release=300ms");
    println!("Press Ctrl+C to stop\n");

    // Create synth engine with MIDI event receiver
    let mut engine = SynthEngine::new(sample_rate, parameters, event_rx);

    // Pre-allocate buffer for processing
    let mut temp_buffer = vec![0.0f32; 512];

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
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;

    // Start audio stream
    stream.play()?;

    // Keep running until interrupted
    std::thread::park();

    Ok(())
}
