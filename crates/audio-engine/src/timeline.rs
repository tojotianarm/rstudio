use crate::TrackId;

/// Sample-accurate position or duration used by project-time models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SampleTime(u64);

impl SampleTime {
    pub const fn new(samples: u64) -> Self {
        Self(samples)
    }

    pub const fn samples(self) -> u64 {
        self.0
    }
}

impl From<u64> for SampleTime {
    fn from(samples: u64) -> Self {
        Self::new(samples)
    }
}

/// Stable identifier for a timeline clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipId(u64);

impl ClipId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

/// A future audio event placed on one track, independent of audio-file playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioClip {
    id: ClipId,
    track_id: TrackId,
    start_position: SampleTime,
    length: SampleTime,
}

impl AudioClip {
    pub const fn new(
        id: ClipId,
        track_id: TrackId,
        start_position: SampleTime,
        length: SampleTime,
    ) -> Self {
        Self { id, track_id, start_position, length }
    }

    pub const fn id(self) -> ClipId {
        self.id
    }

    pub const fn track_id(self) -> TrackId {
        self.track_id
    }

    pub const fn start_position(self) -> SampleTime {
        self.start_position
    }

    pub const fn length(self) -> SampleTime {
        self.length
    }

    pub fn is_active_at(self, position: SampleTime) -> bool {
        let end_position = self.start_position.samples().saturating_add(self.length.samples());
        position.samples() >= self.start_position.samples() && position.samples() < end_position
    }
}

/// Non-real-time editing operations for the project timeline.
///
/// They intentionally do not use `AudioCommand`: applying them can grow or compact the timeline
/// vector, which must never happen in the audio callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineCommand {
    AddClip { track_id: TrackId, clip: AudioClip },
    RemoveClip { clip_id: ClipId },
}

/// Editable project-time clip model. It is not accessed from the audio callback yet.
#[derive(Debug, Default)]
pub struct Timeline {
    clips: Vec<AudioClip>,
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_clip(&mut self, clip: AudioClip) {
        self.clips.push(clip);
    }

    pub fn remove_clip(&mut self, clip_id: ClipId) -> Option<AudioClip> {
        let index = self.clips.iter().position(|clip| clip.id() == clip_id)?;
        Some(self.clips.remove(index))
    }

    /// Iterates over active clips without allocating an intermediate collection.
    ///
    /// A slice cannot represent arbitrary active clips because overlapping clip ranges may be
    /// non-contiguous in the timeline's storage order.
    pub fn active_clips_at(&self, position: SampleTime) -> impl Iterator<Item = &AudioClip> {
        self.clips.iter().filter(move |clip| clip.is_active_at(position))
    }

    pub fn apply_command(&mut self, command: TimelineCommand) {
        match command {
            TimelineCommand::AddClip { track_id, clip } if clip.track_id() == track_id => {
                self.add_clip(clip);
            }
            TimelineCommand::AddClip { .. } => {}
            TimelineCommand::RemoveClip { clip_id } => {
                let _ = self.remove_clip(clip_id);
            }
        }
    }

    pub fn clips(&self) -> &[AudioClip] {
        &self.clips
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioClip, ClipId, SampleTime, Timeline, TimelineCommand};
    use crate::{TrackId, Transport};

    #[test]
    fn audio_clip_retains_sample_accurate_fields() {
        let clip = AudioClip::new(
            ClipId::new(3),
            TrackId::new(1),
            SampleTime::new(480),
            SampleTime::new(960),
        );

        assert_eq!(clip.id(), ClipId::new(3));
        assert_eq!(clip.start_position(), SampleTime::new(480));
        assert_eq!(clip.length(), SampleTime::new(960));
    }

    #[test]
    fn timeline_finds_clips_active_at_a_position() {
        let mut timeline = Timeline::new();
        let clip_a = AudioClip::new(
            ClipId::new(1),
            TrackId::new(1),
            SampleTime::new(0),
            SampleTime::new(1_000),
        );
        let clip_b = AudioClip::new(
            ClipId::new(2),
            TrackId::new(1),
            SampleTime::new(2_000),
            SampleTime::new(1_000),
        );
        timeline.add_clip(clip_a);
        timeline.add_clip(clip_b);

        let at_500: Vec<_> = timeline.active_clips_at(SampleTime::new(500)).collect();
        let at_2500: Vec<_> = timeline.active_clips_at(SampleTime::new(2_500)).collect();

        assert_eq!(at_500, [&clip_a]);
        assert_eq!(at_2500, [&clip_b]);
    }

    #[test]
    fn timeline_commands_apply_outside_real_time_rendering() {
        let mut timeline = Timeline::new();
        let clip = AudioClip::new(
            ClipId::new(1),
            TrackId::new(2),
            SampleTime::new(0),
            SampleTime::new(128),
        );

        timeline.apply_command(TimelineCommand::AddClip { track_id: TrackId::new(2), clip });
        assert_eq!(timeline.clips(), &[clip]);

        timeline.apply_command(TimelineCommand::RemoveClip { clip_id: ClipId::new(1) });
        assert!(timeline.clips().is_empty());
    }

    #[test]
    fn transport_position_can_query_the_timeline() {
        let transport = Transport::new(48_000);
        let clip = AudioClip::new(
            ClipId::new(1),
            TrackId::new(1),
            SampleTime::new(0),
            SampleTime::new(48_000),
        );
        let mut timeline = Timeline::new();
        timeline.add_clip(clip);

        let active: Vec<_> =
            timeline.active_clips_at(SampleTime::from(transport.position_samples())).collect();
        assert_eq!(active, [&clip]);
    }
}
