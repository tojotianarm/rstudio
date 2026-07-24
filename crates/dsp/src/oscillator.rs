use std::f32::consts::PI;

pub struct Oscillator {
    frequency: f32,
    sample_rate: f32,
    phase: f32,
}

impl Oscillator {
    pub fn new(frequency: f32, sample_rate: f32) -> Self {
        Self {
            frequency,
            sample_rate,
            phase: 0.0,
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        let sample = (self.phase * 2.0 * PI).sin();

        self.phase += self.frequency / self.sample_rate;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }
}