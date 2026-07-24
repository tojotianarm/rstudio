pub mod buffer;
pub mod device;
pub mod engine;
pub mod stream;

pub use buffer::{AudioBlockMut, AudioBuffer, AudioFormat};
pub use engine::AudioEngine;
pub use stream::{
    AudioCommandSender, AudioEventReceiver, AudioStreamError, CpalOutputStream, DeviceBufferSize,
    OutputDeviceInfo,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioCommand {
    Start,
    Stop,
    SetOscillatorFrequency(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioEvent {
    ProcessedBlock,
    StreamError,
}
