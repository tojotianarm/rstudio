use std::sync::Arc;

use crossbeam_queue::ArrayQueue;
use thiserror::Error;

use crate::{AudioClip, ClipId, SampleTime, Timeline, TrackId};

pub const SNAPSHOT_TRACK_COUNT: usize = 2;
pub const MAX_CLIPS_PER_TRACK: usize = 16;
const SNAPSHOT_QUEUE_CAPACITY: usize = 4;

/// Clip timing precomputed for real-time playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipPlayback {
    clip_id: ClipId,
    start: SampleTime,
    end: SampleTime,
}

impl ClipPlayback {
    pub const fn clip_id(self) -> ClipId {
        self.clip_id
    }

    pub const fn start(self) -> SampleTime {
        self.start
    }

    pub const fn end(self) -> SampleTime {
        self.end
    }

    pub fn is_active_at(self, position: SampleTime) -> bool {
        position >= self.start && position < self.end
    }
}

/// Fixed-capacity playback data for one track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioTrackSnapshot {
    track_id: TrackId,
    clips: [Option<ClipPlayback>; MAX_CLIPS_PER_TRACK],
    clip_count: usize,
}

impl AudioTrackSnapshot {
    pub const fn track_id(self) -> TrackId {
        self.track_id
    }

    pub fn clips(&self) -> &[Option<ClipPlayback>] {
        &self.clips[..self.clip_count]
    }

    fn new(track_id: TrackId) -> Self {
        Self { track_id, clips: [None; MAX_CLIPS_PER_TRACK], clip_count: 0 }
    }

    fn push(&mut self, clip: ClipPlayback) -> Result<(), SnapshotCompileError> {
        if self.clip_count == MAX_CLIPS_PER_TRACK {
            return Err(SnapshotCompileError::TooManyClips { track_id: self.track_id });
        }
        self.clips[self.clip_count] = Some(clip);
        self.clip_count += 1;
        Ok(())
    }
}

/// Immutable, fixed-size project view consumed by the audio thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioSnapshot {
    tracks: [AudioTrackSnapshot; SNAPSHOT_TRACK_COUNT],
}

impl AudioSnapshot {
    /// Compiles editable timeline data outside the audio callback.
    pub fn compile(timeline: &Timeline) -> Result<Self, SnapshotCompileError> {
        SnapshotCompiler::compile(timeline)
    }

    /// Keeps the prototype oscillator audible until an application snapshot is published.
    pub fn prototype() -> Self {
        let playback = ClipPlayback {
            clip_id: ClipId::new(0),
            start: SampleTime::new(0),
            end: SampleTime::new(u64::MAX),
        };
        let mut first = AudioTrackSnapshot::new(TrackId::new(1));
        let mut second = AudioTrackSnapshot::new(TrackId::new(2));
        first.clips[0] = Some(playback);
        first.clip_count = 1;
        second.clips[0] = Some(playback);
        second.clip_count = 1;
        Self { tracks: [first, second] }
    }

    pub fn track(&self, track_id: TrackId) -> Option<&AudioTrackSnapshot> {
        self.tracks.iter().find(|track| track.track_id == track_id)
    }

    pub fn tracks(&self) -> &[AudioTrackSnapshot; SNAPSHOT_TRACK_COUNT] {
        &self.tracks
    }
}

/// Compiles editable project data into an immutable real-time representation.
pub struct SnapshotCompiler;

impl SnapshotCompiler {
    pub fn compile(timeline: &Timeline) -> Result<AudioSnapshot, SnapshotCompileError> {
        let mut snapshot = AudioSnapshot {
            tracks: [
                AudioTrackSnapshot::new(TrackId::new(1)),
                AudioTrackSnapshot::new(TrackId::new(2)),
            ],
        };

        for clip in timeline.clips() {
            let track = snapshot
                .tracks
                .iter_mut()
                .find(|track| track.track_id == clip.track_id())
                .ok_or(SnapshotCompileError::UnknownTrack { track_id: clip.track_id() })?;
            track.push(compile_clip(*clip))?;
        }
        Ok(snapshot)
    }
}

fn compile_clip(clip: AudioClip) -> ClipPlayback {
    ClipPlayback {
        clip_id: clip.id(),
        start: clip.start_position(),
        end: SampleTime::new(
            clip.start_position().samples().saturating_add(clip.length().samples()),
        ),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SnapshotCompileError {
    #[error("timeline contains a clip for unknown track {track_id:?}")]
    UnknownTrack { track_id: TrackId },
    #[error("track {track_id:?} exceeds the snapshot clip capacity")]
    TooManyClips { track_id: TrackId },
}

/// Queries precompiled playback data. It never accesses `Timeline`.
pub struct ClipScheduler<'a> {
    snapshot: &'a AudioSnapshot,
}

impl<'a> ClipScheduler<'a> {
    pub fn new(snapshot: &'a AudioSnapshot) -> Self {
        Self { snapshot }
    }

    pub fn active_clips(
        &self,
        track_id: TrackId,
        position: SampleTime,
    ) -> impl Iterator<Item = ClipPlayback> + '_ {
        self.track_clips(track_id).filter(move |clip| clip.is_active_at(position))
    }

    /// Iterates precompiled clips for one track without accessing the editable timeline.
    pub fn track_clips(&self, track_id: TrackId) -> impl Iterator<Item = ClipPlayback> + '_ {
        self.snapshot
            .track(track_id)
            .into_iter()
            .flat_map(AudioTrackSnapshot::clips)
            .flatten()
            .copied()
    }

    pub fn is_track_active(&self, track_id: TrackId, position: SampleTime) -> bool {
        self.active_clips(track_id, position).next().is_some()
    }
}

/// Application-side publisher for immutable, fixed-size snapshots.
#[derive(Clone)]
pub struct AudioSnapshotSender {
    snapshots: Arc<ArrayQueue<AudioSnapshot>>,
}

impl AudioSnapshotSender {
    pub fn try_send(&self, snapshot: AudioSnapshot) -> Result<(), SnapshotPublishError> {
        self.snapshots.push(snapshot).map_err(|_| SnapshotPublishError::QueueFull)
    }
}

/// Audio-thread endpoint. Consuming a snapshot only copies a fixed-size value.
pub struct AudioSnapshotReceiver {
    snapshots: Arc<ArrayQueue<AudioSnapshot>>,
}

impl AudioSnapshotReceiver {
    pub(crate) fn take_latest(&self) -> Option<AudioSnapshot> {
        let mut latest = None;
        for _ in 0..SNAPSHOT_QUEUE_CAPACITY {
            let Some(snapshot) = self.snapshots.pop() else {
                break;
            };
            latest = Some(snapshot);
        }
        latest
    }
}

pub fn audio_snapshot_exchange() -> (AudioSnapshotSender, AudioSnapshotReceiver) {
    let snapshots = Arc::new(ArrayQueue::new(SNAPSHOT_QUEUE_CAPACITY));
    (AudioSnapshotSender { snapshots: Arc::clone(&snapshots) }, AudioSnapshotReceiver { snapshots })
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotPublishError {
    #[error("the audio snapshot queue is full")]
    QueueFull,
}

#[cfg(test)]
mod tests {
    use super::{
        AudioSnapshot, ClipScheduler, SampleTime, SnapshotCompiler, audio_snapshot_exchange,
    };
    use crate::{AudioClip, ClipId, Timeline, TrackId};

    fn timeline_with_two_clips() -> Timeline {
        let mut timeline = Timeline::new();
        timeline.add_clip(AudioClip::new(
            ClipId::new(1),
            TrackId::new(1),
            SampleTime::new(0),
            SampleTime::new(1_000),
        ));
        timeline.add_clip(AudioClip::new(
            ClipId::new(2),
            TrackId::new(2),
            SampleTime::new(2_000),
            SampleTime::new(500),
        ));
        timeline
    }

    #[test]
    fn snapshot_compiles_tracks_and_clip_end_positions() {
        let snapshot =
            SnapshotCompiler::compile(&timeline_with_two_clips()).expect("valid timeline");

        let first = snapshot.track(TrackId::new(1)).expect("first track exists");
        let second = snapshot.track(TrackId::new(2)).expect("second track exists");
        assert_eq!(first.clips()[0].expect("clip exists").end(), SampleTime::new(1_000));
        assert_eq!(second.clips()[0].expect("clip exists").end(), SampleTime::new(2_500));
    }

    #[test]
    fn scheduler_returns_only_active_clips() {
        let snapshot = AudioSnapshot::compile(&timeline_with_two_clips()).expect("valid timeline");
        let scheduler = ClipScheduler::new(&snapshot);

        assert_eq!(scheduler.active_clips(TrackId::new(1), SampleTime::new(100)).count(), 1);
        assert_eq!(scheduler.active_clips(TrackId::new(1), SampleTime::new(1_500)).count(), 0);
        assert_eq!(scheduler.active_clips(TrackId::new(2), SampleTime::new(2_200)).count(), 1);
    }

    #[test]
    fn snapshot_exchange_transfers_a_fixed_value() {
        let (sender, receiver) = audio_snapshot_exchange();
        let snapshot = AudioSnapshot::compile(&timeline_with_two_clips()).expect("valid timeline");

        sender.try_send(snapshot).expect("queue has capacity");

        assert_eq!(receiver.take_latest(), Some(snapshot));
    }
}
