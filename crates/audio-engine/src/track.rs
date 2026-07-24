use crate::{AudioBlockMut, AudioFormat, AudioGraphCommand, AudioNode, OscillatorNode};

/// Stable numeric identifier for a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrackId(u32);

impl TrackId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

/// A fixed audio channel with configuration data and real-time render state.
///
/// `name` is application-facing configuration and is never accessed by `process`.
pub struct Track {
    configuration: TrackConfiguration,
    realtime: TrackRealtimeState,
}

/// Application-facing data that is not read during rendering.
struct TrackConfiguration {
    id: TrackId,
    name: String,
}

/// Audio-thread-owned state used by the bounded render path.
struct TrackRealtimeState {
    gain: f32,
    muted: bool,
    solo: bool,
    source: OscillatorNode,
}

impl Track {
    pub fn new(id: TrackId, name: String, frequency: f32, format: AudioFormat) -> Self {
        Self {
            configuration: TrackConfiguration { id, name },
            realtime: TrackRealtimeState {
                gain: 1.0,
                muted: false,
                solo: false,
                source: OscillatorNode::new(frequency, format),
            },
        }
    }

    pub fn id(&self) -> TrackId {
        self.configuration.id
    }

    pub fn name(&self) -> &str {
        &self.configuration.name
    }

    pub fn gain(&self) -> f32 {
        self.realtime.gain
    }

    pub fn is_muted(&self) -> bool {
        self.realtime.muted
    }

    pub fn is_solo(&self) -> bool {
        self.realtime.solo
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.realtime.gain = if gain.is_finite() { gain.max(0.0) } else { 0.0 };
    }

    pub fn set_mute(&mut self, muted: bool) {
        self.realtime.muted = muted;
    }

    pub fn set_solo(&mut self, solo: bool) {
        self.realtime.solo = solo;
    }

    pub fn prepare(&mut self, format: AudioFormat) {
        self.realtime.source.prepare(format);
    }

    /// Renders the source and applies the track channel state without allocating.
    pub fn process(&mut self, output: &mut AudioBlockMut<'_>) {
        self.realtime.source.process(&[], std::slice::from_mut(&mut *output));
        self.apply_channel_state(output);
    }

    fn apply_channel_state(&self, output: &mut AudioBlockMut<'_>) {
        if self.realtime.muted {
            output.clear();
            return;
        }

        for sample in output.as_mut_slice() {
            *sample = if sample.is_finite() {
                (*sample * self.realtime.gain).clamp(-1.0, 1.0)
            } else {
                0.0
            };
        }
    }

    pub fn reset(&mut self) {
        self.realtime.source.reset();
    }

    pub fn apply_command(&mut self, command: AudioGraphCommand) {
        self.realtime.source.apply_command(command);
    }
}

#[cfg(test)]
mod tests {
    use super::{Track, TrackId};
    use crate::{AudioBuffer, AudioFormat};

    #[test]
    fn track_starts_with_neutral_channel_state() {
        let format = AudioFormat::new(48_000, 1).expect("valid mono format");
        let track = Track::new(TrackId::new(7), String::from("Synth"), 440.0, format);

        assert_eq!(track.id(), TrackId::new(7));
        assert_eq!(track.name(), "Synth");
        assert_eq!(track.gain(), 1.0);
        assert!(!track.is_muted());
        assert!(!track.is_solo());
    }

    #[test]
    fn track_channel_gain_scales_samples() {
        let format = AudioFormat::new(48_000, 1).expect("valid mono format");
        let mut track = Track::new(TrackId::new(1), String::from("Track"), 440.0, format);
        track.set_gain(0.5);
        let mut output = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
        output.as_mut_slice().copy_from_slice(&[0.5, -0.5]);

        track.apply_channel_state(&mut output.block_mut());

        assert_eq!(output.as_slice(), &[0.25, -0.25]);
    }

    #[test]
    fn muted_track_outputs_silence() {
        let format = AudioFormat::new(48_000, 1).expect("valid mono format");
        let mut track = Track::new(TrackId::new(1), String::from("Track"), 440.0, format);
        track.set_mute(true);
        let mut output = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
        output.as_mut_slice().copy_from_slice(&[0.5, -0.5]);

        track.apply_channel_state(&mut output.block_mut());

        assert_eq!(output.as_slice(), &[0.0, 0.0]);
    }
}
