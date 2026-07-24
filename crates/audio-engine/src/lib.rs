pub mod buffer;
pub mod device;
pub mod engine;
pub mod graph;
pub mod node;
pub mod stream;

pub use buffer::{AudioBlock, AudioBlockMut, AudioBuffer, AudioFormat};
pub use engine::AudioEngine;
pub use graph::AudioGraph;
pub use node::{AudioGraphCommand, AudioNode, GainNode, MixerNode, OscillatorNode};
pub use stream::{
    AudioCommandSender, AudioEventReceiver, AudioStreamError, CpalOutputStream, DeviceBufferSize,
    OutputDeviceInfo,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioCommand {
    Start,
    Stop,
    SetOscillatorFrequency(f32),
    SetMasterGain(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioEvent {
    ProcessedBlock,
    OutputBufferCapacityExceeded,
    StreamError,
}
