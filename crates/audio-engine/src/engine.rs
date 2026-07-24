use crate::AudioBuffer;
use common::config::AppConfig;
use dsp::oscillator::Oscillator;

pub struct AudioEngine {
    config: AppConfig,
    running: bool,
    oscillator: Oscillator,
}

impl AudioEngine {
    pub fn new(config: AppConfig) -> Self {
        Self {
            oscillator: Oscillator::new(440.0, config.sample_rate as f32),
            config,
            running: false,
        }
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn next_sample(&mut self) -> f32 {
        self.oscillator.next_sample()
    }

    pub fn process(&mut self, buffer: &mut AudioBuffer) {
        for sample in buffer.as_mut_slice() {
            *sample = self.next_sample();
        }
    }
}
