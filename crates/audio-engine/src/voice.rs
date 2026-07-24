use crate::OscillatorNode;
use crate::{
    AudioBlockMut, AudioFormat, AudioGraphCommand, AudioNode, ClipId, ClipPlayback, ClipScheduler,
    SampleTime, TrackId,
};

/// Default bounded polyphony for one fixed track.
pub const MAX_VOICES_PER_TRACK: usize = 8;

/// Stable identifier for a preallocated voice slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceId(u32);

impl VoiceId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Audio-thread-owned playback state for one active clip.
///
/// A voice owns its DSP source and is activated only by assigning an already preallocated slot.
/// It never creates or drops heap-backed state while processing audio.
pub struct Voice {
    id: VoiceId,
    clip_id: Option<ClipId>,
    start_position: SampleTime,
    source: OscillatorNode,
}

impl Voice {
    fn new(id: VoiceId, frequency: f32, format: AudioFormat) -> Self {
        Self {
            id,
            clip_id: None,
            start_position: SampleTime::new(0),
            source: OscillatorNode::new(frequency, format),
        }
    }

    pub const fn id(&self) -> VoiceId {
        self.id
    }

    pub const fn is_active(&self) -> bool {
        self.clip_id.is_some()
    }

    pub const fn clip_id(&self) -> Option<ClipId> {
        self.clip_id
    }

    pub const fn start_position(&self) -> SampleTime {
        self.start_position
    }

    fn activate(&mut self, clip: ClipPlayback) {
        self.clip_id = Some(clip.clip_id());
        self.start_position = clip.start();
        self.source.reset();
    }

    fn deactivate(&mut self) {
        self.clip_id = None;
    }

    fn prepare(&mut self, format: AudioFormat) {
        self.source.prepare(format);
        self.deactivate();
    }

    fn reset(&mut self) {
        self.source.reset();
        self.deactivate();
    }

    fn process(&mut self, output: &mut AudioBlockMut<'_>) {
        if self.is_active() {
            self.source.render_add(output);
        }
    }

    fn apply_command(&mut self, command: AudioGraphCommand) {
        self.source.apply_command(command);
    }
}

/// Fixed-capacity, audio-thread-owned pool of voices for one track.
///
/// `N` is configured when the track is constructed. Growing it at callback time would require
/// allocation, so a later graph/snapshot rebuild must be used for runtime polyphony changes.
pub struct VoiceManager<const N: usize> {
    voices: [Voice; N],
}

impl<const N: usize> VoiceManager<N> {
    pub fn new(frequency: f32, format: AudioFormat) -> Self {
        Self {
            voices: std::array::from_fn(|index| {
                Voice::new(VoiceId::new(index as u32), frequency, format)
            }),
        }
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|voice| voice.is_active()).count()
    }

    pub fn voice(&self, id: VoiceId) -> Option<&Voice> {
        self.voices.get(id.value() as usize)
    }

    pub fn prepare(&mut self, format: AudioFormat) {
        for voice in &mut self.voices {
            voice.prepare(format);
        }
    }

    /// Reconciles the fixed voice pool with active snapshot clips and mixes every active voice.
    /// All searches are bounded by `N` and the fixed snapshot capacity; no collection is built.
    pub fn process(
        &mut self,
        output: &mut AudioBlockMut<'_>,
        scheduler: &ClipScheduler<'_>,
        track_id: TrackId,
        position: SampleTime,
    ) {
        output.clear();

        for voice in &mut self.voices {
            let still_active = voice.clip_id().is_some_and(|clip_id| {
                scheduler.active_clips(track_id, position).any(|clip| clip.clip_id() == clip_id)
            });
            if !still_active {
                voice.deactivate();
            }
        }

        for clip in scheduler.active_clips(track_id, position) {
            let assigned = self.voices.iter().any(|voice| voice.clip_id() == Some(clip.clip_id()));
            if !assigned
                && let Some(voice) = self.voices.iter_mut().find(|voice| !voice.is_active())
            {
                voice.activate(clip);
            }
        }

        for voice in &mut self.voices {
            voice.process(output);
        }
    }

    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.reset();
        }
    }

    pub fn apply_command(&mut self, command: AudioGraphCommand) {
        for voice in &mut self.voices {
            voice.apply_command(command);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{VoiceId, VoiceManager};
    use crate::{
        AudioBuffer, AudioClip, AudioFormat, AudioSnapshot, ClipId, ClipScheduler, SampleTime,
        Timeline, TrackId,
    };

    #[test]
    fn voice_manager_activates_and_releases_a_preallocated_slot() {
        let format = AudioFormat::new(48_000, 1).expect("valid format");
        let mut timeline = Timeline::new();
        timeline.add_clip(AudioClip::new(
            ClipId::new(1),
            TrackId::new(1),
            SampleTime::new(0),
            SampleTime::new(100),
        ));
        let snapshot = AudioSnapshot::compile(&timeline).expect("valid timeline");
        let scheduler = ClipScheduler::new(&snapshot);
        let mut voices = VoiceManager::<2>::new(440.0, format);
        let mut output = AudioBuffer::new(8, format).expect("valid buffer");

        voices.process(&mut output.block_mut(), &scheduler, TrackId::new(1), SampleTime::new(0));
        assert_eq!(voices.active_voice_count(), 1);
        assert_eq!(
            voices.voice(VoiceId::new(0)).and_then(|voice| voice.clip_id()),
            Some(ClipId::new(1))
        );
        assert!(output.as_slice().iter().any(|sample| *sample != 0.0));

        voices.process(&mut output.block_mut(), &scheduler, TrackId::new(1), SampleTime::new(100));
        assert_eq!(voices.active_voice_count(), 0);
        assert!(output.as_slice().iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn overlapping_clips_use_multiple_voices() {
        let format = AudioFormat::new(48_000, 1).expect("valid format");
        let mut timeline = Timeline::new();
        timeline.add_clip(AudioClip::new(
            ClipId::new(1),
            TrackId::new(1),
            SampleTime::new(0),
            SampleTime::new(10_000),
        ));
        timeline.add_clip(AudioClip::new(
            ClipId::new(2),
            TrackId::new(1),
            SampleTime::new(5_000),
            SampleTime::new(10_000),
        ));
        let snapshot = AudioSnapshot::compile(&timeline).expect("valid timeline");
        let scheduler = ClipScheduler::new(&snapshot);
        let mut voices = VoiceManager::<2>::new(440.0, format);
        let mut output = AudioBuffer::new(8, format).expect("valid buffer");

        voices.process(
            &mut output.block_mut(),
            &scheduler,
            TrackId::new(1),
            SampleTime::new(6_000),
        );

        assert_eq!(voices.active_voice_count(), 2);
        assert!(output.as_slice().iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn pool_capacity_is_bounded_and_reusable() {
        let format = AudioFormat::new(48_000, 1).expect("valid format");
        let mut timeline = Timeline::new();
        for id in 0..3 {
            timeline.add_clip(AudioClip::new(
                ClipId::new(id),
                TrackId::new(1),
                SampleTime::new(0),
                SampleTime::new(100),
            ));
        }
        let snapshot = AudioSnapshot::compile(&timeline).expect("valid timeline");
        let scheduler = ClipScheduler::new(&snapshot);
        let mut voices = VoiceManager::<2>::new(440.0, format);
        let mut output = AudioBuffer::new(8, format).expect("valid buffer");

        voices.process(&mut output.block_mut(), &scheduler, TrackId::new(1), SampleTime::new(0));
        assert_eq!(voices.active_voice_count(), 2);
        voices.process(&mut output.block_mut(), &scheduler, TrackId::new(1), SampleTime::new(100));
        assert_eq!(voices.active_voice_count(), 0);
    }
}
