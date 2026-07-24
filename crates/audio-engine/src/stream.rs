use std::sync::Arc;

use cpal::{
    SampleFormat, Stream, SupportedBufferSize,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_queue::ArrayQueue;
use thiserror::Error;

use crate::{AudioCommand, AudioEngine, AudioEvent};

const COMMAND_QUEUE_CAPACITY: usize = 256;
const EVENT_QUEUE_CAPACITY: usize = 256;
const MAX_COMMANDS_PER_CALLBACK: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceBufferSize {
    Unknown,
    Range { min_frames: u32, max_frames: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputDeviceInfo {
    sample_rate: u32,
    channels: u16,
    buffer_size: DeviceBufferSize,
}

impl OutputDeviceInfo {
    pub fn sample_rate(self) -> u32 {
        self.sample_rate
    }
    pub fn channels(self) -> u16 {
        self.channels
    }
    pub fn buffer_size(self) -> DeviceBufferSize {
        self.buffer_size
    }
}

#[derive(Debug, Error)]
pub enum AudioStreamError {
    #[error("no default output device is available")]
    NoDefaultOutputDevice,
    #[error("could not query the default output configuration: {0}")]
    DefaultOutputConfig(#[source] cpal::DefaultStreamConfigError),
    #[error("unsupported output sample format: {0:?}")]
    UnsupportedSampleFormat(SampleFormat),
    #[error("could not build the output stream: {0}")]
    BuildStream(#[source] cpal::BuildStreamError),
    #[error("could not start the output stream: {0}")]
    PlayStream(#[source] cpal::PlayStreamError),
    #[error("the real-time command queue is full")]
    CommandQueueFull,
}

#[derive(Clone)]
pub struct AudioCommandSender {
    commands: Arc<ArrayQueue<AudioCommand>>,
}
impl AudioCommandSender {
    pub fn try_send(&self, command: AudioCommand) -> Result<(), AudioStreamError> {
        self.commands.push(command).map_err(|_| AudioStreamError::CommandQueueFull)
    }
}

pub struct AudioEventReceiver {
    events: Arc<ArrayQueue<AudioEvent>>,
}
impl AudioEventReceiver {
    pub fn try_recv(&self) -> Option<AudioEvent> {
        self.events.pop()
    }
}

pub struct CpalOutputStream {
    stream: Stream,
    device_info: OutputDeviceInfo,
    commands: AudioCommandSender,
    events: AudioEventReceiver,
}

impl CpalOutputStream {
    pub fn open_default(engine: AudioEngine) -> Result<Self, AudioStreamError> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(AudioStreamError::NoDefaultOutputDevice)?;
        let supported_config =
            device.default_output_config().map_err(AudioStreamError::DefaultOutputConfig)?;
        let device_info = OutputDeviceInfo {
            sample_rate: supported_config.sample_rate().0,
            channels: supported_config.channels(),
            buffer_size: device_buffer_size(supported_config.buffer_size()),
        };
        let config = supported_config.config();
        let commands = Arc::new(ArrayQueue::new(COMMAND_QUEUE_CAPACITY));
        let events = Arc::new(ArrayQueue::new(EVENT_QUEUE_CAPACITY));
        let stream = build_stream(
            &device,
            &config,
            supported_config.sample_format(),
            engine,
            Arc::clone(&commands),
            Arc::clone(&events),
        )?;
        Ok(Self {
            stream,
            device_info,
            commands: AudioCommandSender { commands },
            events: AudioEventReceiver { events },
        })
    }

    pub fn device_info(&self) -> OutputDeviceInfo {
        self.device_info
    }
    pub fn commands(&self) -> AudioCommandSender {
        self.commands.clone()
    }
    pub fn events(&self) -> &AudioEventReceiver {
        &self.events
    }
    pub fn start(&self) -> Result<(), AudioStreamError> {
        self.commands.try_send(AudioCommand::Start)?;
        self.stream.play().map_err(AudioStreamError::PlayStream)
    }
    pub fn stop(&self) -> Result<(), AudioStreamError> {
        self.commands.try_send(AudioCommand::Stop)
    }
}

fn device_buffer_size(size: &SupportedBufferSize) -> DeviceBufferSize {
    match size {
        SupportedBufferSize::Unknown => DeviceBufferSize::Unknown,
        SupportedBufferSize::Range { min, max } => {
            DeviceBufferSize::Range { min_frames: *min, max_frames: *max }
        }
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: SampleFormat,
    engine: AudioEngine,
    commands: Arc<ArrayQueue<AudioCommand>>,
    events: Arc<ArrayQueue<AudioEvent>>,
) -> Result<Stream, AudioStreamError> {
    let channels = usize::from(config.channels);
    match sample_format {
        SampleFormat::F32 => {
            build_typed_stream::<f32>(device, config, engine, commands, events, channels)
        }
        SampleFormat::I16 => {
            build_typed_stream::<i16>(device, config, engine, commands, events, channels)
        }
        SampleFormat::U16 => {
            build_typed_stream::<u16>(device, config, engine, commands, events, channels)
        }
        unsupported => Err(AudioStreamError::UnsupportedSampleFormat(unsupported)),
    }
}

fn build_typed_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut engine: AudioEngine,
    commands: Arc<ArrayQueue<AudioCommand>>,
    events: Arc<ArrayQueue<AudioEvent>>,
    channels: usize,
) -> Result<Stream, AudioStreamError>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let callback_events = Arc::clone(&events);
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| {
                for _ in 0..MAX_COMMANDS_PER_CALLBACK {
                    match commands.pop() {
                        Some(command) => engine.apply_command(command),
                        None => break,
                    }
                }
                for frame in output.chunks_exact_mut(channels) {
                    let sample = engine.next_output_sample();
                    for output_sample in frame {
                        *output_sample = T::from_sample(sample);
                    }
                }
                let remainder = output.len() % channels;
                if remainder != 0 {
                    let output_len = output.len();
                    for output_sample in &mut output[output_len - remainder..] {
                        *output_sample = T::from_sample(0.0);
                    }
                }
                let _ = callback_events.push(AudioEvent::ProcessedBlock);
            },
            move |_| {
                let _ = events.push(AudioEvent::StreamError);
            },
            None,
        )
        .map_err(AudioStreamError::BuildStream)
}
