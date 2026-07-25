use crate::{
    AudioBlockMut, AudioFormat, AudioGraphCommand, ClipId, ClipPlayback, ClipScheduler, ClipSource,
    Instrument, PARAMETER_CAPACITY, ParameterStore, SampleRegistry, SampleTime, ScheduledEvent,
    SimpleSynth, TrackId, VoiceEvent,
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
    instrument: SimpleSynth,
    sample: Option<(crate::SampleId, f64)>,
    output_sample_rate: u32,
}

impl Voice {
    fn new(id: VoiceId, frequency: f32, format: AudioFormat) -> Self {
        Self {
            id,
            clip_id: None,
            start_position: SampleTime::new(0),
            instrument: SimpleSynth::new(frequency, format),
            sample: None,
            output_sample_rate: format.sample_rate(),
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
        self.sample = match clip.source() {
            ClipSource::Synth => {
                self.instrument.note_on(self.instrument.default_note());
                None
            }
            ClipSource::AudioFile(sample_id) => Some((sample_id, 0.0)),
        };
    }

    fn deactivate(&mut self) {
        self.instrument.reset();
        self.sample = None;
        self.clip_id = None;
    }

    fn note_off(&mut self) {
        self.instrument.note_off();
    }

    fn prepare(&mut self, format: AudioFormat) {
        self.output_sample_rate = format.sample_rate();
        self.instrument.prepare(format);
        self.deactivate();
    }

    fn reset(&mut self) {
        self.deactivate();
    }

    fn render_frame(
        &mut self,
        frame: &mut [f32],
        parameters: &mut ParameterStore<PARAMETER_CAPACITY>,
        samples: &SampleRegistry,
    ) {
        if let Some((sample_id, position)) = &mut self.sample {
            let Some(buffer) = samples.get(*sample_id) else {
                self.deactivate();
                return;
            };
            let index = *position as usize;
            if index >= buffer.frames() {
                self.deactivate();
                return;
            }
            let next = (index + 1).min(buffer.frames() - 1);
            let fraction = (*position - index as f64) as f32;
            for (channel, output) in frame.iter_mut().enumerate() {
                let a = buffer.sample(index, channel);
                let b = buffer.sample(next, channel);
                *output = (*output + a + (b - a) * fraction).clamp(-1.0, 1.0);
            }
            *position += buffer.sample_rate() as f64 / self.output_sample_rate as f64;
        } else if self.is_active() {
            self.instrument.render_frame(frame, parameters);
        }
    }

    fn apply_command(&mut self, command: AudioGraphCommand) {
        self.instrument.apply_command(command);
    }

    fn release_finished(&self) -> bool {
        self.sample.is_none() && self.is_active() && !self.instrument.is_active()
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

    /// Applies events immediately before their target frames and renders the active voice pool.
    /// All searches are bounded by `N` and the fixed snapshot capacity; no collection is built.
    #[allow(clippy::too_many_arguments)] // Fixed real-time render context; avoids heap allocation.
    pub fn process(
        &mut self,
        output: &mut AudioBlockMut<'_>,
        events: &[ScheduledEvent],
        scheduler: &ClipScheduler<'_>,
        track_id: TrackId,
        position: SampleTime,
        parameters: &mut ParameterStore<PARAMETER_CAPACITY>,
        samples: &SampleRegistry,
    ) {
        output.clear();

        for voice in &mut self.voices {
            let still_active = voice.clip_id().is_some_and(|clip_id| {
                scheduler.active_clips(track_id, position).any(|clip| clip.clip_id() == clip_id)
            });
            if !still_active {
                voice.note_off();
            }
        }

        let mut event_index = 0;
        for (frame_index, frame) in output.frames_mut().enumerate() {
            while let Some(event) = events.get(event_index) {
                if event.sample_offset() != frame_index {
                    break;
                }
                self.apply_event(event.event());
                event_index += 1;
            }

            for voice in &mut self.voices {
                voice.render_frame(frame, parameters, samples);
            }
            for voice in &mut self.voices {
                if voice.release_finished() {
                    voice.deactivate();
                }
            }
        }
    }

    fn apply_event(&mut self, event: VoiceEvent) {
        match event {
            VoiceEvent::StartVoice(clip) => self.start_voice(clip),
            VoiceEvent::StopVoice(clip_id) => self.stop_voice(clip_id),
        }
    }

    fn start_voice(&mut self, clip: ClipPlayback) {
        if self.voices.iter().any(|voice| voice.clip_id() == Some(clip.clip_id())) {
            return;
        }
        if let Some(voice) = self.voices.iter_mut().find(|voice| !voice.is_active()) {
            voice.activate(clip);
        }
    }

    fn stop_voice(&mut self, clip_id: ClipId) {
        if let Some(voice) = self.voices.iter_mut().find(|voice| voice.clip_id() == Some(clip_id)) {
            voice.note_off();
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
        AudioBuffer, AudioClip, AudioFormat, AudioSnapshot, ClipId, ClipScheduler,
        ENGINE_PARAMETERS, EventScheduler, PARAMETER_CAPACITY, ParameterStore,
        SampleRegistryBuilder, SampleTime, Timeline, TrackId,
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
        let mut events = EventScheduler::<2>::new();
        let mut parameters = ParameterStore::<PARAMETER_CAPACITY>::new(&ENGINE_PARAMETERS);
        let samples = SampleRegistryBuilder::new().build();

        let scheduled = events.schedule(&scheduler, TrackId::new(1), SampleTime::new(0), 8);
        voices.process(
            &mut output.block_mut(),
            scheduled,
            &scheduler,
            TrackId::new(1),
            SampleTime::new(0),
            &mut parameters,
            &samples,
        );
        assert_eq!(voices.active_voice_count(), 1);
        assert_eq!(
            voices.voice(VoiceId::new(0)).and_then(|voice| voice.clip_id()),
            Some(ClipId::new(1))
        );
        assert!(output.as_slice().iter().any(|sample| *sample != 0.0));

        let scheduled = events.schedule(&scheduler, TrackId::new(1), SampleTime::new(100), 8);
        voices.process(
            &mut output.block_mut(),
            scheduled,
            &scheduler,
            TrackId::new(1),
            SampleTime::new(100),
            &mut parameters,
            &samples,
        );
        assert_eq!(voices.active_voice_count(), 1);
        assert!(output.as_slice().iter().any(|sample| *sample != 0.0));

        for block in 1..=60 {
            let position = SampleTime::new(100 + block * 8);
            let scheduled = events.schedule(&scheduler, TrackId::new(1), position, 8);
            voices.process(
                &mut output.block_mut(),
                scheduled,
                &scheduler,
                TrackId::new(1),
                position,
                &mut parameters,
                &samples,
            );
        }
        assert_eq!(voices.active_voice_count(), 0);
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
        let mut events = EventScheduler::<4>::new();
        let mut parameters = ParameterStore::<PARAMETER_CAPACITY>::new(&ENGINE_PARAMETERS);
        let samples = SampleRegistryBuilder::new().build();

        let scheduled = events.schedule(&scheduler, TrackId::new(1), SampleTime::new(6_000), 8);
        voices.process(
            &mut output.block_mut(),
            scheduled,
            &scheduler,
            TrackId::new(1),
            SampleTime::new(6_000),
            &mut parameters,
            &samples,
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
        let mut events = EventScheduler::<6>::new();
        let mut parameters = ParameterStore::<PARAMETER_CAPACITY>::new(&ENGINE_PARAMETERS);
        let samples = SampleRegistryBuilder::new().build();

        let scheduled = events.schedule(&scheduler, TrackId::new(1), SampleTime::new(0), 8);
        voices.process(
            &mut output.block_mut(),
            scheduled,
            &scheduler,
            TrackId::new(1),
            SampleTime::new(0),
            &mut parameters,
            &samples,
        );
        assert_eq!(voices.active_voice_count(), 2);
        let scheduled = events.schedule(&scheduler, TrackId::new(1), SampleTime::new(100), 8);
        voices.process(
            &mut output.block_mut(),
            scheduled,
            &scheduler,
            TrackId::new(1),
            SampleTime::new(100),
            &mut parameters,
            &samples,
        );
        assert_eq!(voices.active_voice_count(), 2);
        for block in 1..=60 {
            let position = SampleTime::new(100 + block * 8);
            let scheduled = events.schedule(&scheduler, TrackId::new(1), position, 8);
            voices.process(
                &mut output.block_mut(),
                scheduled,
                &scheduler,
                TrackId::new(1),
                position,
                &mut parameters,
                &samples,
            );
        }
        assert_eq!(voices.active_voice_count(), 0);
    }
}
