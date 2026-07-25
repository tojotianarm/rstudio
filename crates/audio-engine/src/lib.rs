pub mod buffer;
pub mod device;
pub mod engine;
pub mod event;
pub mod graph;
pub mod instrument;
pub mod meter;
pub mod node;
pub mod parameter;
pub mod snapshot;
pub mod stream;
pub mod timeline;
pub mod track;
pub mod transport;
pub mod voice;

pub use buffer::{AudioBlock, AudioBlockMut, AudioBuffer, AudioFormat};
pub use engine::AudioEngine;
pub use event::{EventScheduler, MAX_SCHEDULED_EVENTS_PER_TRACK, ScheduledEvent, VoiceEvent};
pub use graph::AudioGraph;
pub use instrument::{Instrument, NoteEvent, SimpleSynth};
pub use meter::AudioMeter;
pub use node::{AudioGraphCommand, AudioNode, GainNode, MasterBus, MixerNode, OscillatorNode};
pub use parameter::*;
pub use snapshot::{
    AudioSnapshot, AudioSnapshotSender, AudioTrackSnapshot, ClipPlayback, ClipScheduler,
    MAX_CLIPS_PER_TRACK, SNAPSHOT_TRACK_COUNT, SnapshotCompileError, SnapshotCompiler,
    SnapshotPublishError, audio_snapshot_exchange,
};
pub use stream::{
    AudioCommandSender, AudioEventReceiver, AudioStreamError, CpalOutputStream, DeviceBufferSize,
    OutputDeviceInfo,
};
pub use timeline::{AudioClip, ClipId, SampleTime, Timeline, TimelineCommand};
pub use track::{Track, TrackId};
pub use transport::{Transport, TransportState};
pub use voice::{MAX_VOICES_PER_TRACK, Voice, VoiceId, VoiceManager};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioCommand {
    Start,
    Stop,
    Play,
    Pause,
    Seek(u64),
    SetBpm(f64),
    SetParameter { id: ParameterId, value: ParameterValue },
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
