/// Immutable transport data that can be sent from the audio thread without synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportState {
    position_samples: u64,
    playing: bool,
}

impl TransportState {
    pub fn position_samples(self) -> u64 {
        self.position_samples
    }

    pub fn is_playing(self) -> bool {
        self.playing
    }
}

/// Sample-accurate musical clock owned exclusively by the audio engine.
pub struct Transport {
    playing: bool,
    bpm: f64,
    position_samples: u64,
    sample_rate: u32,
}

impl Transport {
    pub fn new(sample_rate: u32) -> Self {
        Self { playing: false, bpm: 120.0, position_samples: 0, sample_rate: sample_rate.max(1) }
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.position_samples = 0;
    }

    pub fn seek(&mut self, position_samples: u64) {
        self.position_samples = position_samples;
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        if bpm.is_finite() && bpm > 0.0 {
            self.bpm = bpm;
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate.max(1);
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn position_samples(&self) -> u64 {
        self.position_samples
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn samples_to_seconds(&self, samples: u64) -> f64 {
        samples as f64 / f64::from(self.sample_rate)
    }

    pub fn beats_to_samples(&self, beats: f64) -> u64 {
        if !beats.is_finite() || beats <= 0.0 {
            return 0;
        }

        let samples = beats * 60.0 / self.bpm * f64::from(self.sample_rate);
        if !samples.is_finite() || samples >= u64::MAX as f64 {
            u64::MAX
        } else {
            samples.round() as u64
        }
    }

    pub fn state(&self) -> TransportState {
        TransportState { position_samples: self.position_samples, playing: self.playing }
    }

    pub(crate) fn advance(&mut self, frames: usize) {
        if self.playing {
            self.position_samples = self.position_samples.saturating_add(frames as u64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Transport;

    #[test]
    fn transport_starts_stopped_at_zero() {
        let transport = Transport::new(48_000);

        assert!(!transport.is_playing());
        assert_eq!(transport.position_samples(), 0);
    }

    #[test]
    fn transport_play_pause_stop_and_seek_update_state() {
        let mut transport = Transport::new(48_000);
        transport.play();
        assert!(transport.is_playing());

        transport.pause();
        assert!(!transport.is_playing());

        transport.seek(48_000);
        assert_eq!(transport.position_samples(), 48_000);

        transport.stop();
        assert!(!transport.is_playing());
        assert_eq!(transport.position_samples(), 0);
    }

    #[test]
    fn transport_advances_only_while_playing() {
        let mut transport = Transport::new(48_000);
        transport.advance(256);
        assert_eq!(transport.position_samples(), 0);

        transport.play();
        transport.advance(256);
        assert_eq!(transport.position_samples(), 256);
    }

    #[test]
    fn transport_converts_samples_and_beats() {
        let transport = Transport::new(48_000);

        assert_eq!(transport.samples_to_seconds(48_000), 1.0);
        assert_eq!(transport.beats_to_samples(1.0), 24_000);
    }
}
