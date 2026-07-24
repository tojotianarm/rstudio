use crate::{AudioBlock, AudioBlockMut, AudioFormat};
use dsp::oscillator::Oscillator;

/// Real-time processing unit owned by an [`AudioGraph`](crate::AudioGraph).
///
/// The current graph is linear and provides one output bus, but the explicit input/output counts
/// reserve the interface for a future graph planner with multiple buses. Nodes are created and
/// prepared outside the audio callback; `process` must not allocate or block.
pub trait AudioNode: Send {
    fn input_count(&self) -> usize;
    fn output_count(&self) -> usize;
    fn prepare(&mut self, format: AudioFormat);
    /// Processes zero or more input buses into zero or more output buses.
    ///
    /// The first graph only supplies one output bus, but using a bus slice here keeps the node
    /// boundary compatible with future multi-input and multi-output routing without allocating
    /// during processing.
    fn process(&mut self, inputs: &[AudioBlock<'_>], outputs: &mut [AudioBlockMut<'_>]);
    fn reset(&mut self);
    fn apply_command(&mut self, _command: AudioGraphCommand) {}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioGraphCommand {
    SetOscillatorFrequency(f32),
}

pub struct OscillatorNode {
    oscillator: Oscillator,
}

impl OscillatorNode {
    pub fn new(frequency: f32, format: AudioFormat) -> Self {
        Self { oscillator: Oscillator::new(frequency, format.sample_rate() as f32) }
    }
}

impl AudioNode for OscillatorNode {
    fn input_count(&self) -> usize {
        0
    }

    fn output_count(&self) -> usize {
        1
    }

    fn prepare(&mut self, format: AudioFormat) {
        self.oscillator.set_sample_rate(format.sample_rate() as f32);
        self.reset();
    }

    fn process(&mut self, inputs: &[AudioBlock<'_>], outputs: &mut [AudioBlockMut<'_>]) {
        debug_assert!(inputs.is_empty());
        debug_assert_eq!(outputs.len(), self.output_count());
        let Some(output) = outputs.first_mut() else {
            return;
        };
        for frame in output.frames_mut() {
            let sample = self.oscillator.next_sample();
            for channel in frame {
                *channel += sample;
            }
        }
    }

    fn reset(&mut self) {
        self.oscillator.reset();
    }

    fn apply_command(&mut self, command: AudioGraphCommand) {
        match command {
            AudioGraphCommand::SetOscillatorFrequency(frequency)
                if frequency.is_finite() && frequency >= 0.0 =>
            {
                self.oscillator.set_frequency(frequency);
            }
            AudioGraphCommand::SetOscillatorFrequency(_) => {}
        }
    }
}
