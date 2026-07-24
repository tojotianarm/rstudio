use crate::{AudioBlockMut, AudioBuffer, AudioCommand, AudioFormat, AudioGraph, AudioGraphCommand};
use common::{config::AppConfig, error::Result};

pub struct AudioEngine {
    config: AppConfig,
    running: bool,
    graph: AudioGraph,
    render_buffer: AudioBuffer,
}

impl AudioEngine {
    pub fn new(config: AppConfig) -> Result<Self> {
        config.validate()?;
        let format = AudioFormat::new(config.sample_rate, 1)?;
        let render_buffer = AudioBuffer::new(config.buffer_size, format)?;
        Ok(Self {
            graph: AudioGraph::with_oscillator(format, config.buffer_size, 440.0)?,
            render_buffer,
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

    pub fn output_format(&self) -> AudioFormat {
        self.graph.format()
    }

    /// Configures the graph from the format negotiated with the output device.
    ///
    /// This method may allocate its fixed render buffer and must only run before the stream starts.
    pub fn configure_output(
        &mut self,
        format: AudioFormat,
        maximum_callback_frames: usize,
    ) -> Result<()> {
        let render_buffer = AudioBuffer::new(maximum_callback_frames, format)?;
        self.graph.prepare(format, maximum_callback_frames)?;
        self.render_buffer = render_buffer;
        Ok(())
    }

    pub fn process(&mut self, block: &mut AudioBlockMut<'_>) {
        if self.running {
            self.graph.process(block);
        } else {
            block.clear();
        }
    }

    pub(crate) fn apply_command(&mut self, command: AudioCommand) {
        match command {
            AudioCommand::Start => self.start(),
            AudioCommand::Stop => self.stop(),
            AudioCommand::SetOscillatorFrequency(frequency) => {
                self.graph.apply_command(AudioGraphCommand::SetOscillatorFrequency(frequency));
            }
            AudioCommand::SetMasterGain(gain) => {
                self.graph.apply_command(AudioGraphCommand::SetMasterGain(gain));
            }
        }
    }

    pub(crate) fn render_device_buffer<T>(&mut self, output: &mut [T]) -> bool
    where
        T: cpal::FromSample<f32> + cpal::Sample,
    {
        let channels = self.output_format().channels();
        if !output.len().is_multiple_of(channels) {
            for sample in output {
                *sample = T::from_sample(0.0);
            }
            return false;
        }

        let frames = output.len() / channels;
        let Some(mut render_block) = self.render_buffer.block_mut_for_frames(frames) else {
            for sample in output {
                *sample = T::from_sample(0.0);
            }
            return false;
        };
        if self.running {
            self.graph.process(&mut render_block);
        } else {
            render_block.clear();
        }

        for (device_sample, rendered_sample) in output.iter_mut().zip(render_block.as_slice()) {
            *device_sample = T::from_sample(*rendered_sample);
        }
        true
    }
}
