use common::error::{RStudioError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    sample_rate: u32,
    channels: usize,
}

impl AudioFormat {
    pub fn new(sample_rate: u32, channels: usize) -> Result<Self> {
        if sample_rate == 0 {
            return Err(RStudioError::InvalidAudioBuffer("sample rate must be greater than zero"));
        }
        if channels == 0 {
            return Err(RStudioError::InvalidAudioBuffer(
                "channel count must be greater than zero",
            ));
        }
        Ok(Self { sample_rate, channels })
    }

    pub fn sample_rate(self) -> u32 {
        self.sample_rate
    }
    pub fn channels(self) -> usize {
        self.channels
    }
}

#[derive(Debug)]
pub struct AudioBuffer {
    format: AudioFormat,
    frames: usize,
    samples: Vec<f32>,
}

impl AudioBuffer {
    pub fn new(frames: usize, format: AudioFormat) -> Result<Self> {
        let sample_count = frames
            .checked_mul(format.channels())
            .ok_or(RStudioError::InvalidAudioBuffer("buffer size overflows"))?;
        Ok(Self { format, frames, samples: vec![0.0; sample_count] })
    }

    pub fn clear(&mut self) {
        self.samples.fill(0.0);
    }
    pub fn frames(&self) -> usize {
        self.frames
    }
    pub fn format(&self) -> AudioFormat {
        self.format
    }
    pub fn len(&self) -> usize {
        self.samples.len()
    }
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
    pub fn as_slice(&self) -> &[f32] {
        &self.samples
    }
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.samples
    }
    pub fn block_mut(&mut self) -> AudioBlockMut<'_> {
        AudioBlockMut { samples: &mut self.samples, format: self.format }
    }

    pub fn block_mut_for_frames(&mut self, frames: usize) -> Option<AudioBlockMut<'_>> {
        let sample_count = frames.checked_mul(self.format.channels())?;
        let samples = self.samples.get_mut(..sample_count)?;
        Some(AudioBlockMut { samples, format: self.format })
    }
}

pub struct AudioBlock<'a> {
    samples: &'a [f32],
    format: AudioFormat,
}

impl<'a> AudioBlock<'a> {
    pub fn as_slice(&self) -> &[f32] {
        self.samples
    }

    pub fn frames(&self) -> usize {
        self.samples.len() / self.format.channels()
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }
}

pub struct AudioBlockMut<'a> {
    samples: &'a mut [f32],
    format: AudioFormat,
}

impl<'a> AudioBlockMut<'a> {
    pub fn from_interleaved(samples: &'a mut [f32], format: AudioFormat) -> Option<Self> {
        samples.len().is_multiple_of(format.channels()).then_some(Self { samples, format })
    }

    pub fn clear(&mut self) {
        self.samples.fill(0.0);
    }
    pub fn as_slice(&self) -> &[f32] {
        self.samples
    }
    pub fn as_block(&self) -> AudioBlock<'_> {
        AudioBlock { samples: self.samples, format: self.format }
    }
    pub fn frames(&self) -> usize {
        self.samples.len() / self.format.channels()
    }
    pub fn format(&self) -> AudioFormat {
        self.format
    }
    pub fn frames_mut(&mut self) -> impl Iterator<Item = &mut [f32]> {
        self.samples.chunks_exact_mut(self.format.channels())
    }
}
