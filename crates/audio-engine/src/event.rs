use crate::{ClipId, ClipPlayback, ClipScheduler, MAX_CLIPS_PER_TRACK, SampleTime, TrackId};

/// Maximum start and stop events that can be scheduled for one track in one block.
pub const MAX_SCHEDULED_EVENTS_PER_TRACK: usize = MAX_CLIPS_PER_TRACK * 2;

/// A voice-pool change applied immediately before rendering `sample_offset` in a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceEvent {
    StartVoice(ClipPlayback),
    StopVoice(ClipId),
}

/// A bounded real-time event with a frame offset relative to the current callback block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledEvent {
    sample_offset: usize,
    event: VoiceEvent,
}

impl ScheduledEvent {
    pub const fn new(sample_offset: usize, event: VoiceEvent) -> Self {
        Self { sample_offset, event }
    }

    pub const fn sample_offset(self) -> usize {
        self.sample_offset
    }

    pub const fn event(self) -> VoiceEvent {
        self.event
    }
}

/// Produces a sorted fixed-size event list from immutable clip playback data.
///
/// The scheduler is audio-thread-owned. It retains its storage between callbacks and only scans
/// the bounded snapshot clip list, so scheduling cannot allocate or block.
pub struct EventScheduler<const N: usize> {
    events: [ScheduledEvent; N],
    event_count: usize,
}

impl<const N: usize> Default for EventScheduler<N> {
    fn default() -> Self {
        Self { events: [EMPTY_EVENT; N], event_count: 0 }
    }
}

impl<const N: usize> EventScheduler<N> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedules all start and stop events in `[block_start, block_start + frames)`.
    ///
    /// An already-active clip is represented by a start event at offset zero. The voice manager
    /// treats this as idempotent, which also makes transport seeks deterministic.
    pub fn schedule<'a>(
        &'a mut self,
        scheduler: &ClipScheduler<'_>,
        track_id: TrackId,
        block_start: SampleTime,
        frames: usize,
    ) -> &'a [ScheduledEvent] {
        self.clear();
        let block_end = SampleTime::new(block_start.samples().saturating_add(frames as u64));

        for clip in scheduler.track_clips(track_id) {
            if clip.start() >= clip.end() {
                continue;
            }

            if clip.start() >= block_start && clip.start() < block_end {
                self.insert(ScheduledEvent::new(
                    offset_from(block_start, clip.start()),
                    VoiceEvent::StartVoice(clip),
                ));
            } else if clip.is_active_at(block_start) {
                self.insert(ScheduledEvent::new(0, VoiceEvent::StartVoice(clip)));
            }

            if clip.end() >= block_start && clip.end() < block_end {
                self.insert(ScheduledEvent::new(
                    offset_from(block_start, clip.end()),
                    VoiceEvent::StopVoice(clip.clip_id()),
                ));
            }
        }

        &self.events[..self.event_count]
    }

    fn clear(&mut self) {
        self.event_count = 0;
    }

    fn insert(&mut self, event: ScheduledEvent) {
        if self.event_count == N {
            return;
        }

        let mut index = self.event_count;
        while index > 0 {
            let previous = self.events[index - 1];
            if !is_before(event, previous) {
                break;
            }
            self.events[index] = previous;
            index -= 1;
        }
        self.events[index] = event;
        self.event_count += 1;
    }
}

fn offset_from(block_start: SampleTime, position: SampleTime) -> usize {
    position.samples().saturating_sub(block_start.samples()) as usize
}

fn is_before(left: ScheduledEvent, right: ScheduledEvent) -> bool {
    left.sample_offset < right.sample_offset
        || (left.sample_offset == right.sample_offset
            && event_priority(left.event) < event_priority(right.event))
}

const fn event_priority(event: VoiceEvent) -> u8 {
    match event {
        VoiceEvent::StopVoice(_) => 0,
        VoiceEvent::StartVoice(_) => 1,
    }
}

const EMPTY_EVENT: ScheduledEvent = ScheduledEvent::new(0, VoiceEvent::StopVoice(ClipId::new(0)));

#[cfg(test)]
mod tests {
    use super::{EventScheduler, VoiceEvent};
    use crate::{AudioClip, AudioSnapshot, ClipId, ClipScheduler, SampleTime, Timeline, TrackId};

    #[test]
    fn scheduler_preserves_and_orders_sample_offsets() {
        let mut timeline = Timeline::new();
        timeline.add_clip(AudioClip::new(
            ClipId::new(1),
            TrackId::new(1),
            SampleTime::new(100),
            SampleTime::new(200),
        ));
        let snapshot = AudioSnapshot::compile(&timeline).expect("valid timeline");
        let clips = ClipScheduler::new(&snapshot);
        let mut events = EventScheduler::<2>::new();

        let scheduled = events.schedule(&clips, TrackId::new(1), SampleTime::new(0), 512);

        assert_eq!(scheduled.len(), 2);
        assert_eq!(scheduled[0].sample_offset(), 100);
        assert!(matches!(scheduled[0].event(), VoiceEvent::StartVoice(_)));
        assert_eq!(scheduled[1].sample_offset(), 300);
        assert_eq!(scheduled[1].event(), VoiceEvent::StopVoice(ClipId::new(1)));
    }

    #[test]
    fn stop_events_precede_starts_at_the_same_sample() {
        let mut timeline = Timeline::new();
        timeline.add_clip(AudioClip::new(
            ClipId::new(1),
            TrackId::new(1),
            SampleTime::new(0),
            SampleTime::new(200),
        ));
        timeline.add_clip(AudioClip::new(
            ClipId::new(2),
            TrackId::new(1),
            SampleTime::new(200),
            SampleTime::new(200),
        ));
        let snapshot = AudioSnapshot::compile(&timeline).expect("valid timeline");
        let clips = ClipScheduler::new(&snapshot);
        let mut events = EventScheduler::<4>::new();

        let scheduled = events.schedule(&clips, TrackId::new(1), SampleTime::new(0), 512);

        assert_eq!(scheduled[1].sample_offset(), 200);
        assert_eq!(scheduled[1].event(), VoiceEvent::StopVoice(ClipId::new(1)));
        assert_eq!(scheduled[2].sample_offset(), 200);
        assert!(matches!(scheduled[2].event(), VoiceEvent::StartVoice(_)));
    }
}
