use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use hound::SampleFormat;

use super::sample::SampleData;

/// Decode a WAV file into shared, mono, normalized sample data.
///
/// Supports integer and float WAV formats of any bit depth and channel
/// count; multi-channel files are averaged down to mono. This performs
/// file I/O and must only be called at startup, never from the audio thread.
pub fn load_wav(path: impl AsRef<Path>) -> Result<Arc<SampleData>> {
    let path = path.as_ref();
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("Failed to open WAV file: {}", path.display()))?;

    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;
    let sample_rate = spec.sample_rate as f32;

    // Decode all samples to interleaved f32 in [-1.0, 1.0].
    let interleaved: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<f32>, _>>()
            .with_context(|| format!("Failed to decode float WAV: {}", path.display()))?,
        SampleFormat::Int => {
            // Integer samples are sign-extended; normalize by the format's
            // full-scale magnitude (2^(bits-1)).
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max))
                .collect::<std::result::Result<Vec<f32>, _>>()
                .with_context(|| format!("Failed to decode int WAV: {}", path.display()))?
        }
    };

    if interleaved.is_empty() {
        return Err(anyhow!("WAV file contains no samples: {}", path.display()));
    }

    // Sum/average channels down to mono.
    let samples: Vec<f32> = if channels == 1 {
        interleaved
    } else {
        interleaved
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
            .collect()
    };

    Ok(Arc::new(SampleData {
        samples,
        sample_rate,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};

    fn write_test_wav(path: &Path, channels: u16, samples: &[i16]) {
        let spec = WavSpec {
            channels,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();
        for &s in samples {
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn test_load_mono() {
        let path = std::env::temp_dir().join("the_synth_test_load_mono.wav");
        write_test_wav(&path, 1, &[0, 16384, -16384, 32767]);

        let data = load_wav(&path).unwrap();
        assert_eq!(data.sample_rate, 44100.0);
        assert_eq!(data.samples.len(), 4);
        // 16384 / 32768 == 0.5
        assert!((data.samples[1] - 0.5).abs() < 0.001);
        assert!((data.samples[2] + 0.5).abs() < 0.001);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_stereo_to_mono() {
        let path = std::env::temp_dir().join("the_synth_test_load_stereo.wav");
        // Interleaved L,R frames: (0.5, -0.5) -> 0.0; (0.5, 0.5) -> 0.5
        write_test_wav(&path, 2, &[16384, -16384, 16384, 16384]);

        let data = load_wav(&path).unwrap();
        assert_eq!(data.samples.len(), 2);
        assert!(data.samples[0].abs() < 0.001);
        assert!((data.samples[1] - 0.5).abs() < 0.001);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_missing_file() {
        let path = std::env::temp_dir().join("the_synth_test_missing_xyzzy.wav");
        let _ = std::fs::remove_file(&path);
        assert!(load_wav(&path).is_err());
    }
}
