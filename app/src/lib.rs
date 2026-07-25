//! Application-side API for the fixed-topology RSTUDIO MVP.
//!
//! This module owns editable project data and communicates with the real-time engine exclusively
//! through command, event, and snapshot queues. It must never be used from the audio callback.

use std::path::Path;

use audio_engine::{
    AudioClip, AudioCommand, AudioEvent, AudioSnapshot, AudioStreamError, ClipId, ClipSource,
    CpalOutputStream, SampleId, SampleLoadError, SampleRegistry, SampleRegistryBuilder, SampleTime,
    SnapshotCompileError, Timeline, TrackId, WavLoader,
};
use common::config::AppConfig;

const MVP_TRACK_COUNT: u32 = 2;

/// Editable project state, owned solely by the application/UI thread.
pub struct Project {
    timeline: Timeline,
    tracks: Vec<ProjectTrack>,
    samples: SampleRegistryBuilder,
    next_clip_id: u64,
    next_sample_id: u16,
}

/// UI-facing metadata for a fixed render track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTrack {
    id: TrackId,
    name: String,
}

impl ProjectTrack {
    pub const fn id(&self) -> TrackId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectError {
    TrackCapacityReached,
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the fixed MVP renderer supports at most two tracks")
    }
}

impl std::error::Error for ProjectError {}

impl Default for Project {
    fn default() -> Self {
        Self::new()
    }
}

impl Project {
    pub fn new() -> Self {
        Self {
            timeline: Timeline::new(),
            tracks: Vec::with_capacity(MVP_TRACK_COUNT as usize),
            samples: SampleRegistryBuilder::new(),
            next_clip_id: 1,
            next_sample_id: 0,
        }
    }

    pub fn tracks(&self) -> &[ProjectTrack] {
        &self.tracks
    }

    /// Adds one of the two tracks supported by the fixed real-time graph.
    pub fn add_track(&mut self, name: impl Into<String>) -> Result<TrackId, ProjectError> {
        if self.tracks.len() >= MVP_TRACK_COUNT as usize {
            return Err(ProjectError::TrackCapacityReached);
        }
        let id = TrackId::new((self.tracks.len() + 1) as u32);
        self.tracks.push(ProjectTrack { id, name: name.into() });
        Ok(id)
    }

    pub fn add_synth_clip(
        &mut self,
        track_id: TrackId,
        start: SampleTime,
        length: SampleTime,
    ) -> ClipId {
        self.add_clip(track_id, start, length, ClipSource::Synth)
    }

    pub fn add_audio_clip(
        &mut self,
        track_id: TrackId,
        start: SampleTime,
        length: SampleTime,
        sample_id: SampleId,
    ) -> ClipId {
        self.add_clip(track_id, start, length, ClipSource::AudioFile(sample_id))
    }

    pub fn load_wav(&mut self, path: impl AsRef<Path>) -> Result<SampleId, SampleLoadError> {
        let id = SampleId::new(self.next_sample_id);
        self.samples.insert(id, WavLoader::load(path)?)?;
        self.next_sample_id = self.next_sample_id.saturating_add(1);
        Ok(id)
    }

    pub fn compile_snapshot(&self) -> Result<AudioSnapshot, SnapshotCompileError> {
        AudioSnapshot::compile(&self.timeline)
    }

    /// Finalizes assets before the engine is created. The registry becomes immutable afterwards.
    pub fn into_sample_registry(self) -> SampleRegistry {
        self.samples.build()
    }

    fn add_clip(
        &mut self,
        track_id: TrackId,
        start: SampleTime,
        length: SampleTime,
        source: ClipSource,
    ) -> ClipId {
        let id = ClipId::new(self.next_clip_id);
        self.next_clip_id = self.next_clip_id.saturating_add(1);
        self.timeline.add_clip(AudioClip::new(id, track_id, start, length).with_source(source));
        id
    }
}

/// UI-safe control surface over the CPAL adapter's bounded queues.
pub struct EngineController {
    stream: CpalOutputStream,
}

impl EngineController {
    pub fn open(
        config: AppConfig,
        registry: SampleRegistry,
        snapshot: AudioSnapshot,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let engine = audio_engine::AudioEngine::new(config)?.with_sample_registry(registry);
        let stream = CpalOutputStream::open_default(engine)?;
        stream.snapshots().try_send(snapshot)?;
        Ok(Self { stream })
    }

    pub fn start(&self) -> Result<(), AudioStreamError> {
        self.stream.start()
    }

    pub fn play(&self) -> Result<(), AudioStreamError> {
        self.command(AudioCommand::Play)
    }

    pub fn pause(&self) -> Result<(), AudioStreamError> {
        self.command(AudioCommand::Pause)
    }

    pub fn stop(&self) -> Result<(), AudioStreamError> {
        self.stream.stop()
    }

    pub fn seek(&self, position_samples: u64) -> Result<(), AudioStreamError> {
        self.command(AudioCommand::Seek(position_samples))
    }

    pub fn set_track_gain(&self, track_id: TrackId, gain: f32) -> Result<(), AudioStreamError> {
        self.command(AudioCommand::SetTrackGain { track_id, gain })
    }

    pub fn set_track_mute(&self, track_id: TrackId, muted: bool) -> Result<(), AudioStreamError> {
        self.command(AudioCommand::SetTrackMute { track_id, muted })
    }

    pub fn set_track_solo(&self, track_id: TrackId, solo: bool) -> Result<(), AudioStreamError> {
        self.command(AudioCommand::SetTrackSolo { track_id, solo })
    }

    pub fn publish_snapshot(&self, project: &Project) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.snapshots().try_send(project.compile_snapshot()?)?;
        Ok(())
    }

    pub fn poll_event(&self) -> Option<AudioEvent> {
        self.stream.events().try_recv()
    }

    fn command(&self, command: AudioCommand) -> Result<(), AudioStreamError> {
        self.stream.commands().try_send(command)
    }
}

#[cfg(test)]
mod tests {
    use super::{Project, ProjectError};
    use audio_engine::{SampleTime, TrackId};

    #[test]
    fn project_exposes_two_fixed_tracks_and_compiles_ui_edits() {
        let mut project = Project::new();
        let synth = project.add_track("Synth").expect("first track");
        let audio = project.add_track("Audio").expect("second track");
        assert_eq!(synth, TrackId::new(1));
        assert_eq!(audio, TrackId::new(2));
        assert_eq!(project.add_track("Third"), Err(ProjectError::TrackCapacityReached));

        project.add_synth_clip(synth, SampleTime::new(0), SampleTime::new(480));
        let snapshot = project.compile_snapshot().expect("fixed project compiles");
        assert!(snapshot.track(synth).is_some());
    }
}
