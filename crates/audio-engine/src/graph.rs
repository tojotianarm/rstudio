use common::error::Result;

use crate::snapshot::AudioSnapshotReceiver;
use crate::{
    AudioBlockMut, AudioBuffer, AudioFormat, AudioGraphCommand, AudioMeter, AudioNode,
    AudioSnapshot, ClipScheduler, MasterBus, MixerNode, SampleTime, Track, TrackId,
};

/// Fixed first-stage DAW graph with two independent track channels.
///
/// The graph reads only its immutable compiled snapshot during rendering. Timeline editing and
/// snapshot compilation happen outside the audio callback.
pub struct AudioGraph {
    format: AudioFormat,
    tracks: [Track; 2],
    track_buffers: [AudioBuffer; 2],
    mixer: MixerNode,
    mix_buffer: AudioBuffer,
    master_bus: MasterBus,
    maximum_block_frames: usize,
    meter: AudioMeter,
    snapshot: AudioSnapshot,
    snapshot_receiver: Option<AudioSnapshotReceiver>,
}

impl AudioGraph {
    /// Creates two source tracks routed through a mixer and master bus.
    pub fn with_oscillator(
        format: AudioFormat,
        maximum_block_frames: usize,
        frequency: f32,
    ) -> Result<Self> {
        let tracks = [
            Track::new(TrackId::new(1), String::from("Track 1"), frequency, format),
            Track::new(TrackId::new(2), String::from("Track 2"), frequency * 0.5, format),
        ];
        let mut graph = Self {
            format,
            tracks,
            track_buffers: [
                AudioBuffer::new(maximum_block_frames, format)?,
                AudioBuffer::new(maximum_block_frames, format)?,
            ],
            mixer: MixerNode::new(2),
            mix_buffer: AudioBuffer::new(maximum_block_frames, format)?,
            master_bus: MasterBus::new(1.0),
            maximum_block_frames,
            meter: AudioMeter::default(),
            snapshot: AudioSnapshot::prototype(),
            snapshot_receiver: None,
        };
        graph.prepare(format, maximum_block_frames)?;
        Ok(graph)
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }

    pub fn meter(&self) -> AudioMeter {
        self.meter
    }

    pub fn set_snapshot(&mut self, snapshot: AudioSnapshot) {
        self.snapshot = snapshot;
    }

    pub(crate) fn set_snapshot_receiver(&mut self, receiver: AudioSnapshotReceiver) {
        self.snapshot_receiver = Some(receiver);
    }

    /// Allocates every render buffer before the audio stream begins.
    pub fn prepare(&mut self, format: AudioFormat, maximum_block_frames: usize) -> Result<()> {
        let track_buffers = [
            AudioBuffer::new(maximum_block_frames, format)?,
            AudioBuffer::new(maximum_block_frames, format)?,
        ];
        let mix_buffer = AudioBuffer::new(maximum_block_frames, format)?;

        self.format = format;
        for track in &mut self.tracks {
            track.prepare(format);
        }
        self.master_bus.prepare(format);
        self.track_buffers = track_buffers;
        self.mix_buffer = mix_buffer;
        self.maximum_block_frames = maximum_block_frames;
        self.meter = AudioMeter::default();
        Ok(())
    }

    pub fn process(&mut self, output: &mut AudioBlockMut<'_>, position: SampleTime) {
        if let Some(receiver) = &self.snapshot_receiver
            && let Some(snapshot) = receiver.take_latest()
        {
            self.snapshot = snapshot;
        }
        if output.format() != self.format || output.frames() > self.maximum_block_frames {
            self.clear_and_measure(output);
            return;
        }

        let frames = output.frames();
        let scheduler = ClipScheduler::new(&self.snapshot);
        for (track, buffer) in self.tracks.iter_mut().zip(&mut self.track_buffers) {
            let Some(mut track_output) = buffer.block_mut_for_frames(frames) else {
                self.clear_and_measure(output);
                return;
            };
            track.process(&mut track_output, &scheduler, position);
        }

        let Some(track_one) = self.track_buffers[0].block_for_frames(frames) else {
            self.clear_and_measure(output);
            return;
        };
        let Some(track_two) = self.track_buffers[1].block_for_frames(frames) else {
            self.clear_and_measure(output);
            return;
        };
        let inputs = [track_one, track_two];
        let Some(mut mix_output) = self.mix_buffer.block_mut_for_frames(frames) else {
            self.clear_and_measure(output);
            return;
        };
        self.mixer.process(&inputs, std::slice::from_mut(&mut mix_output));

        let Some(mix_input) = self.mix_buffer.block_for_frames(frames) else {
            self.clear_and_measure(output);
            return;
        };
        self.master_bus
            .process(std::slice::from_ref(&mix_input), std::slice::from_mut(&mut *output));
        self.meter.measure(output.as_block());
    }

    pub fn reset(&mut self) {
        for track in &mut self.tracks {
            track.reset();
        }
        self.mixer.reset();
        self.master_bus.reset();
        self.meter = AudioMeter::default();
    }

    pub fn apply_command(&mut self, command: AudioGraphCommand) {
        match command {
            AudioGraphCommand::SetMasterGain(_) => self.master_bus.apply_command(command),
            AudioGraphCommand::SetOscillatorFrequency(_) => {
                for track in &mut self.tracks {
                    track.apply_command(command);
                }
            }
        }
    }

    pub fn set_track_gain(&mut self, track_id: TrackId, gain: f32) {
        for track in &mut self.tracks {
            if track.id() == track_id {
                track.set_gain(gain);
            }
        }
    }

    pub fn set_track_mute(&mut self, track_id: TrackId, muted: bool) {
        for track in &mut self.tracks {
            if track.id() == track_id {
                track.set_mute(muted);
            }
        }
    }

    pub fn set_track_solo(&mut self, track_id: TrackId, solo: bool) {
        for track in &mut self.tracks {
            if track.id() == track_id {
                track.set_solo(solo);
            }
        }
    }

    fn clear_and_measure(&mut self, output: &mut AudioBlockMut<'_>) {
        output.clear();
        self.meter.measure(output.as_block());
    }
}
