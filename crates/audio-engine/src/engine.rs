use crate::{AudioBlockMut, AudioCommand};
use common::{config::AppConfig, error::Result};
use dsp::oscillator::Oscillator;

pub struct AudioEngine {
    config: AppConfig,
    running: bool,
    oscillator: Oscillator,
}

impl AudioEngine {
    pub fn new(config: AppConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            oscillator: Oscillator::new(440.0, config.sample_rate as f32),
            config,
            running: false,
        })
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

    pub fn process(&mut self, block: &mut AudioBlockMut<'_>) {
        for frame in block.frames_mut() {
            frame.fill(self.next_output_sample());
        }
    }

    pub(crate) fn apply_command(&mut self, command: AudioCommand) {
        match command {
            AudioCommand::Start => self.start(),
            AudioCommand::Stop => self.stop(),
            AudioCommand::SetOscillatorFrequency(frequency)
                if frequency.is_finite() && frequency >= 0.0 =>
            {
                self.oscillator.set_frequency(frequency)
            }
            AudioCommand::SetOscillatorFrequency(_) => {}
        }
    }

    pub(crate) fn next_output_sample(&mut self) -> f32 {
        if self.running { self.oscillator.next_sample() } else { 0.0 }
    }
}
