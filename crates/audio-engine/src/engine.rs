use crate::{
    AudioBlockMut, AudioBuffer, AudioCommand, AudioFormat, AudioGraph, AudioGraphCommand,
    AudioMeter, Transport, TransportState,
};
use common::{config::AppConfig, error::Result};

pub struct AudioEngine {
    config: AppConfig,
    running: bool,
    graph: AudioGraph,
    render_buffer: AudioBuffer,
    transport: Transport,
}

impl AudioEngine {
    pub fn new(config: AppConfig) -> Result<Self> {
        config.validate()?;
        let format = AudioFormat::new(config.sample_rate, 1)?;
        let render_buffer = AudioBuffer::new(config.buffer_size, format)?;
        Ok(Self {
            graph: AudioGraph::with_oscillator(format, config.buffer_size, 440.0)?,
            render_buffer,
            transport: Transport::new(config.sample_rate),
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

    pub fn transport_state(&self) -> TransportState {
        self.transport.state()
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
        self.transport.set_sample_rate(format.sample_rate());
        Ok(())
    }

    pub fn process(&mut self, block: &mut AudioBlockMut<'_>) {
        if self.running {
            self.graph.process(block);
            self.transport.advance(block.frames());
        } else {
            block.clear();
        }
    }

    pub(crate) fn apply_command(&mut self, command: AudioCommand) {
        match command {
            AudioCommand::Start => self.start(),
            AudioCommand::Stop => {
                self.stop();
                self.transport.stop();
            }
            AudioCommand::Play => self.transport.play(),
            AudioCommand::Pause => self.transport.pause(),
            AudioCommand::Seek(position_samples) => self.transport.seek(position_samples),
            AudioCommand::SetBpm(bpm) => self.transport.set_bpm(bpm),
            AudioCommand::SetOscillatorFrequency(frequency) => {
                self.graph.apply_command(AudioGraphCommand::SetOscillatorFrequency(frequency));
            }
            AudioCommand::SetMasterGain(gain) => {
                self.graph.apply_command(AudioGraphCommand::SetMasterGain(gain));
            }
        }
    }

    pub(crate) fn render_device_buffer<T>(&mut self, output: &mut [T]) -> Option<RenderResult>
    where
        T: cpal::FromSample<f32> + cpal::Sample,
    {
        let channels = self.output_format().channels();
        if !output.len().is_multiple_of(channels) {
            for sample in output {
                *sample = T::from_sample(0.0);
            }
            return None;
        }

        let frames = output.len() / channels;
        let Some(mut render_block) = self.render_buffer.block_mut_for_frames(frames) else {
            for sample in output {
                *sample = T::from_sample(0.0);
            }
            return None;
        };
        let meter = if self.running {
            self.graph.process(&mut render_block);
            self.transport.advance(frames);
            self.graph.meter()
        } else {
            render_block.clear();
            AudioMeter::default()
        };

        for (device_sample, rendered_sample) in output.iter_mut().zip(render_block.as_slice()) {
            *device_sample = T::from_sample(*rendered_sample);
        }
        Some(RenderResult { meter, transport: self.transport.state() })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RenderResult {
    pub meter: AudioMeter,
    pub transport: TransportState,
}

#[cfg(test)]
mod tests {
    use super::AudioEngine;
    use crate::{AudioBuffer, AudioCommand, AudioFormat};
    use common::config::AppConfig;

    #[test]
    fn processing_a_playing_block_advances_transport_by_its_frame_count() {
        let config = AppConfig { sample_rate: 48_000, buffer_size: 256 };
        let mut engine = AudioEngine::new(config).expect("valid configuration");
        let format = AudioFormat::new(48_000, 1).expect("valid mono format");
        let mut output = AudioBuffer::new(256, format).expect("buffer dimensions are valid");

        engine.start();
        engine.apply_command(AudioCommand::Play);
        engine.process(&mut output.block_mut());

        assert_eq!(engine.transport_state().position_samples(), 256);
        assert!(engine.transport_state().is_playing());
    }
}
