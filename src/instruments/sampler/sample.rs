/// Immutable, decoded sample data shared across all voices of a sampler.
///
/// Audio is stored mono, normalized to [-1.0, 1.0], at the file's native
/// sample rate. Playback handles sample-rate conversion and pitch via a
/// variable read rate, so we never resample at load time.
pub struct SampleData {
    pub samples: Vec<f32>,
    pub sample_rate: f32,
}
