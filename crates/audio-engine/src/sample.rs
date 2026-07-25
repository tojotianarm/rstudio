use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SampleId(u16);
impl SampleId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u16 {
        self.0
    }
}

/// Preloaded interleaved PCM data. It is never decoded or resized by the audio callback.
#[derive(Debug, Clone)]
pub struct PcmAudioBuffer {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: usize,
}
impl PcmAudioBuffer {
    pub fn new(
        samples: Vec<f32>,
        sample_rate: u32,
        channels: usize,
    ) -> Result<Self, SampleLoadError> {
        if sample_rate == 0 || channels == 0 || !samples.len().is_multiple_of(channels) {
            return Err(SampleLoadError::InvalidBuffer);
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(SampleLoadError::InvalidBuffer);
        }
        Ok(Self { samples, sample_rate, channels })
    }
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn channels(&self) -> usize {
        self.channels
    }
    pub fn sample(&self, frame: usize, channel: usize) -> f32 {
        self.samples
            .get(frame * self.channels + channel.min(self.channels - 1))
            .copied()
            .unwrap_or(0.0)
    }
}

pub struct WavLoader;
impl WavLoader {
    pub fn load(path: impl AsRef<Path>) -> Result<PcmAudioBuffer, SampleLoadError> {
        let mut reader = hound::WavReader::open(path).map_err(SampleLoadError::Wav)?;
        let spec = reader.spec();
        let samples = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(SampleLoadError::Wav)?,
            hound::SampleFormat::Int => {
                let scale = (1_i64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
                reader
                    .samples::<i32>()
                    .map(|sample| sample.map(|value| value as f32 / scale))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(SampleLoadError::Wav)?
            }
        };
        PcmAudioBuffer::new(samples, spec.sample_rate, usize::from(spec.channels))
    }
}

pub const MAX_REGISTERED_SAMPLES: usize = 16;
pub struct SampleRegistry {
    samples: [Option<PcmAudioBuffer>; MAX_REGISTERED_SAMPLES],
}
pub struct SampleRegistryBuilder {
    samples: [Option<PcmAudioBuffer>; MAX_REGISTERED_SAMPLES],
}
impl SampleRegistryBuilder {
    pub fn new() -> Self {
        Self { samples: std::array::from_fn(|_| None) }
    }
    pub fn insert(&mut self, id: SampleId, buffer: PcmAudioBuffer) -> Result<(), SampleLoadError> {
        let Some(slot) = self.samples.get_mut(id.value() as usize) else {
            return Err(SampleLoadError::RegistryFull);
        };
        if slot.is_some() {
            return Err(SampleLoadError::RegistryFull);
        }
        *slot = Some(buffer);
        Ok(())
    }
    pub fn build(self) -> SampleRegistry {
        SampleRegistry { samples: self.samples }
    }
}
impl SampleRegistry {
    pub fn get(&self, id: SampleId) -> Option<&PcmAudioBuffer> {
        self.samples.get(id.value() as usize)?.as_ref()
    }
}

/// Audio-thread-owned cursor over a preloaded PCM buffer.
pub struct SamplePlayer<'a> {
    buffer: &'a PcmAudioBuffer,
    position: f64,
}
impl<'a> SamplePlayer<'a> {
    pub fn new(buffer: &'a PcmAudioBuffer) -> Self {
        Self { buffer, position: 0.0 }
    }
    pub fn render_frame(&mut self, output: &mut [f32], output_sample_rate: u32) -> bool {
        let frame = self.position as usize;
        if frame >= self.buffer.frames() {
            return false;
        }
        let next = (frame + 1).min(self.buffer.frames() - 1);
        let fraction = (self.position - frame as f64) as f32;
        for (channel, sample) in output.iter_mut().enumerate() {
            let a = self.buffer.sample(frame, channel);
            let b = self.buffer.sample(next, channel);
            *sample = (a + (b - a) * fraction).clamp(-1.0, 1.0);
        }
        self.position += self.buffer.sample_rate() as f64 / output_sample_rate.max(1) as f64;
        true
    }
}
#[derive(Debug, Error)]
pub enum SampleLoadError {
    #[error("invalid PCM buffer")]
    InvalidBuffer,
    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),
    #[error("sample registry capacity exceeded or slot already occupied")]
    RegistryFull,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_returns_preloaded_sample_by_id() {
        let mut builder = SampleRegistryBuilder::new();
        builder
            .insert(SampleId::new(3), PcmAudioBuffer::new(vec![0.0], 48_000, 1).expect("buffer"))
            .expect("slot");
        let registry = builder.build();
        assert_eq!(registry.get(SampleId::new(3)).map(PcmAudioBuffer::frames), Some(1));
        assert!(registry.get(SampleId::new(4)).is_none());
    }
    #[test]
    fn player_ends_with_silence() {
        let buffer = PcmAudioBuffer::new(vec![0.25, 0.5], 48_000, 1).expect("buffer");
        let mut player = SamplePlayer::new(&buffer);
        let mut frame = [0.0];
        assert!(player.render_frame(&mut frame, 48_000));
        assert_eq!(frame, [0.25]);
        assert!(player.render_frame(&mut frame, 48_000));
        assert_eq!(frame, [0.5]);
        assert!(!player.render_frame(&mut frame, 48_000));
    }
}
