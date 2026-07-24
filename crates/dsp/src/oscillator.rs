use std::f32::consts::PI;

pub struct Oscillator {
    frequency: f32,
    sample_rate: f32,
    phase: f32,
}

impl Oscillator {
    pub fn new(frequency: f32, sample_rate: f32) -> Self {
        Self { frequency, sample_rate, phase: 0.0 }
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    pub fn next_sample(&mut self) -> f32 {
        let sample = (self.phase * 2.0 * PI).sin();

        self.phase += self.frequency / self.sample_rate;

        self.phase = self.phase.fract();

        sample
    }
}
