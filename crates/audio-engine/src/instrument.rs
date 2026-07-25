use dsp::{
    envelope::{Adsr, AdsrParameters},
    oscillator::Oscillator,
};

use crate::{
    AudioFormat, AudioGraphCommand, PARAMETER_CAPACITY, ParameterStore, SYNTH_ATTACK_PARAMETER,
    SYNTH_DECAY_PARAMETER, SYNTH_FREQUENCY_PARAMETER, SYNTH_GAIN_PARAMETER,
    SYNTH_RELEASE_PARAMETER, SYNTH_SUSTAIN_PARAMETER,
};

/// Sample-accurate note parameters passed from a voice to its instrument.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteEvent {
    frequency: f32,
}

impl NoteEvent {
    pub fn new(frequency: f32) -> Self {
        Self { frequency: sanitize_frequency(frequency) }
    }

    pub const fn frequency(self) -> f32 {
        self.frequency
    }
}

/// Real-time instrument contract implemented by the fixed voice pool.
///
/// Implementations own all DSP state and must not allocate or block from these methods.
pub trait Instrument: Send {
    fn prepare(&mut self, format: AudioFormat);
    fn note_on(&mut self, note: NoteEvent);
    fn note_off(&mut self);
    fn render_frame(
        &mut self,
        frame: &mut [f32],
        parameters: &mut ParameterStore<PARAMETER_CAPACITY>,
    );
    fn is_active(&self) -> bool;
    fn reset(&mut self);
    fn apply_command(&mut self, _command: AudioGraphCommand) {}
}

/// Initial polyphonic synth: a sine oscillator shaped by an ADSR envelope.
pub struct SimpleSynth {
    oscillator: Oscillator,
    envelope: Adsr,
    default_frequency: f32,
}

impl SimpleSynth {
    pub fn new(frequency: f32, format: AudioFormat) -> Self {
        let frequency = sanitize_frequency(frequency);
        Self {
            oscillator: Oscillator::new(frequency, format.sample_rate() as f32),
            envelope: Adsr::new(format.sample_rate() as f32, AdsrParameters::default()),
            default_frequency: frequency,
        }
    }
}

impl Instrument for SimpleSynth {
    fn prepare(&mut self, format: AudioFormat) {
        self.oscillator.set_sample_rate(format.sample_rate() as f32);
        self.envelope.set_sample_rate(format.sample_rate() as f32);
        self.reset();
    }

    fn note_on(&mut self, note: NoteEvent) {
        self.oscillator.set_frequency(note.frequency());
        self.oscillator.reset();
        self.envelope.note_on();
    }

    fn note_off(&mut self) {
        self.envelope.note_off();
    }

    fn render_frame(
        &mut self,
        frame: &mut [f32],
        parameters: &mut ParameterStore<PARAMETER_CAPACITY>,
    ) {
        if !self.is_active() {
            return;
        }
        self.oscillator.set_frequency(
            parameters.next_value(SYNTH_FREQUENCY_PARAMETER).unwrap_or(self.default_frequency),
        );
        self.envelope.set_parameters(AdsrParameters {
            attack_seconds: parameters.next_value(SYNTH_ATTACK_PARAMETER).unwrap_or(0.002),
            decay_seconds: parameters.next_value(SYNTH_DECAY_PARAMETER).unwrap_or(0.050),
            sustain_level: parameters.next_value(SYNTH_SUSTAIN_PARAMETER).unwrap_or(0.8),
            release_seconds: parameters.next_value(SYNTH_RELEASE_PARAMETER).unwrap_or(0.010),
        });
        let amplitude = self.envelope.next_amplitude();
        let sample = self.oscillator.next_sample()
            * amplitude
            * parameters.next_value(SYNTH_GAIN_PARAMETER).unwrap_or(1.0);
        let sample = if sample.is_finite() { sample } else { 0.0 };
        for channel in frame {
            *channel = (*channel + sample).clamp(-1.0, 1.0);
        }
    }

    fn is_active(&self) -> bool {
        self.envelope.is_active()
    }

    fn reset(&mut self) {
        self.oscillator.reset();
        self.envelope.reset();
    }

    fn apply_command(&mut self, command: AudioGraphCommand) {
        if let AudioGraphCommand::SetOscillatorFrequency(frequency) = command {
            self.default_frequency = sanitize_frequency(frequency);
        }
    }
}

impl SimpleSynth {
    pub fn default_note(&self) -> NoteEvent {
        NoteEvent::new(self.default_frequency)
    }
}

fn sanitize_frequency(frequency: f32) -> f32 {
    if frequency.is_finite() { frequency.max(0.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::{Instrument, NoteEvent, SimpleSynth};
    use crate::{AudioFormat, ENGINE_PARAMETERS, PARAMETER_CAPACITY, ParameterStore};

    #[test]
    fn simple_synth_produces_signal_and_finishes_its_release() {
        let format = AudioFormat::new(1_000, 1).expect("valid format");
        let mut synth = SimpleSynth::new(100.0, format);
        let mut frame = [0.0];
        let mut parameters = ParameterStore::<PARAMETER_CAPACITY>::new(&ENGINE_PARAMETERS);

        synth.note_on(NoteEvent::new(100.0));
        for _ in 0..10 {
            synth.render_frame(&mut frame, &mut parameters);
        }
        assert!(frame[0].is_finite());
        assert!(synth.is_active());

        synth.note_off();
        for _ in 0..16 {
            synth.render_frame(&mut frame, &mut parameters);
        }
        assert!(!synth.is_active());
    }
}
