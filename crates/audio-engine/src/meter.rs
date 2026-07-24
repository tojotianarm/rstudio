use crate::AudioBlock;

/// Peak and RMS levels measured over the most recently rendered audio block.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AudioMeter {
    peak: f32,
    rms: f32,
}

impl AudioMeter {
    /// Measures interleaved samples without allocating temporary storage.
    pub fn measure(&mut self, block: AudioBlock<'_>) {
        let samples = block.as_slice();
        if samples.is_empty() {
            *self = Self::default();
            return;
        }

        let mut peak = 0.0_f32;
        let mut sum_of_squares = 0.0_f32;
        for sample in samples {
            let sample = if sample.is_finite() { sample.clamp(-1.0, 1.0) } else { 0.0 };
            peak = peak.max(sample.abs());
            sum_of_squares += sample * sample;
        }

        self.peak = peak;
        self.rms = (sum_of_squares / samples.len() as f32).sqrt();
    }

    pub fn peak(self) -> f32 {
        self.peak
    }

    pub fn rms(self) -> f32 {
        self.rms
    }
}
