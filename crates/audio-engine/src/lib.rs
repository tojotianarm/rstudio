pub mod buffer;
pub mod device;
pub mod engine;
pub mod graph;
pub mod meter;
pub mod node;
pub mod stream;
pub mod timeline;
pub mod track;
pub mod transport;

pub use buffer::{AudioBlock, AudioBlockMut, AudioBuffer, AudioFormat};
pub use engine::AudioEngine;
pub use graph::AudioGraph;
pub use meter::AudioMeter;
pub use node::{AudioGraphCommand, AudioNode, GainNode, MasterBus, MixerNode, OscillatorNode};
pub use stream::{
    AudioCommandSender, AudioEventReceiver, AudioStreamError, CpalOutputStream, DeviceBufferSize,
    OutputDeviceInfo,
};
pub use timeline::{AudioClip, ClipId, SampleTime, Timeline, TimelineCommand};
pub use track::{Track, TrackId};
pub use transport::{Transport, TransportState};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioCommand {
    Start,
    Stop,
    Play,
    Pause,
    Seek(u64),
    SetBpm(f64),
    SetTrackGain { track_id: TrackId, gain: f32 },
    SetTrackMute { track_id: TrackId, muted: bool },
    SetTrackSolo { track_id: TrackId, solo: bool },
    SetOscillatorFrequency(f32),
    SetMasterGain(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioEvent {
    ProcessedBlock,
    OutputBufferCapacityExceeded,
    MeterUpdate { peak: f32, rms: f32 },
    TransportUpdate { position_samples: u64, playing: bool },
    TrackMeterUpdate { track_id: TrackId, peak: f32, rms: f32 },
    StreamError,
}
