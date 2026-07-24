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
    SetMasterGain(f32),
}

pub struct OscillatorNode {
    oscillator: Oscillator,
}

impl OscillatorNode {
    pub fn new(frequency: f32, format: AudioFormat) -> Self {
        Self { oscillator: Oscillator::new(frequency, format.sample_rate() as f32) }
    }

    /// Adds one generated sample to every channel of an interleaved frame.
    ///
    /// This is used by the fixed voice pool. It deliberately does not allocate and keeps the
    /// oscillator state local to the audio thread.
    pub(crate) fn render_frame_add(&mut self, frame: &mut [f32]) {
        let sample = self.oscillator.next_sample();
        let sample = if sample.is_finite() { sample } else { 0.0 };
        for channel in frame {
            *channel = (*channel + sample).clamp(-1.0, 1.0);
        }
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
                *channel = sample;
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
            AudioGraphCommand::SetOscillatorFrequency(_) | AudioGraphCommand::SetMasterGain(_) => {}
        }
    }
}

/// Applies a finite, non-negative gain to one input bus.
pub struct GainNode {
    gain: f32,
}

impl GainNode {
    pub fn new(gain: f32) -> Self {
        let mut node = Self { gain: 1.0 };
        node.set_gain(gain);
        node
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = if gain.is_finite() { gain.max(0.0) } else { 0.0 };
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }
}

impl AudioNode for GainNode {
    fn input_count(&self) -> usize {
        1
    }

    fn output_count(&self) -> usize {
        1
    }

    fn prepare(&mut self, _format: AudioFormat) {}

    fn process(&mut self, inputs: &[AudioBlock<'_>], outputs: &mut [AudioBlockMut<'_>]) {
        let output_count = outputs.len();
        let Some(input) = inputs.first() else {
            clear_first_output(outputs);
            return;
        };
        let Some(output) = outputs.first_mut() else {
            return;
        };
        if inputs.len() != self.input_count()
            || output_count != self.output_count()
            || input.format() != output.format()
            || input.as_slice().len() != output.as_slice().len()
        {
            output.clear();
            return;
        }

        for (destination, source) in output.as_mut_slice().iter_mut().zip(input.as_slice()) {
            *destination =
                if source.is_finite() { (*source * self.gain).clamp(-1.0, 1.0) } else { 0.0 };
        }
    }

    fn reset(&mut self) {}
}

/// Mixes a fixed number of input buses into one saturated output bus.
pub struct MixerNode {
    input_count: usize,
}

impl MixerNode {
    pub fn new(input_count: usize) -> Self {
        Self { input_count }
    }
}

impl AudioNode for MixerNode {
    fn input_count(&self) -> usize {
        self.input_count
    }

    fn output_count(&self) -> usize {
        1
    }

    fn prepare(&mut self, _format: AudioFormat) {}

    fn process(&mut self, inputs: &[AudioBlock<'_>], outputs: &mut [AudioBlockMut<'_>]) {
        let output_count = outputs.len();
        let Some(output) = outputs.first_mut() else {
            return;
        };
        if inputs.len() != self.input_count() || output_count != self.output_count() {
            output.clear();
            return;
        }
        if inputs.iter().any(|input| {
            input.format() != output.format() || input.as_slice().len() != output.as_slice().len()
        }) {
            output.clear();
            return;
        }

        for (sample_index, destination) in output.as_mut_slice().iter_mut().enumerate() {
            let mut mixed = 0.0;
            for input in inputs {
                let sample = input.as_slice()[sample_index];
                if sample.is_finite() {
                    mixed = (mixed + sample).clamp(-1.0, 1.0);
                }
            }
            *destination = mixed;
        }
    }

    fn reset(&mut self) {}
}

/// Final output bus of the graph.
///
/// Effects such as a limiter or master EQ will be inserted here in a future graph revision.
pub struct MasterBus {
    gain: f32,
}

impl MasterBus {
    pub fn new(gain: f32) -> Self {
        let mut bus = Self { gain: 1.0 };
        bus.set_gain(gain);
        bus
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = if gain.is_finite() { gain.max(0.0) } else { 0.0 };
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }
}

impl AudioNode for MasterBus {
    fn input_count(&self) -> usize {
        1
    }

    fn output_count(&self) -> usize {
        1
    }

    fn prepare(&mut self, _format: AudioFormat) {}

    fn process(&mut self, inputs: &[AudioBlock<'_>], outputs: &mut [AudioBlockMut<'_>]) {
        let output_count = outputs.len();
        let Some(input) = inputs.first() else {
            clear_first_output(outputs);
            return;
        };
        let Some(output) = outputs.first_mut() else {
            return;
        };
        if inputs.len() != self.input_count()
            || output_count != self.output_count()
            || input.format() != output.format()
            || input.as_slice().len() != output.as_slice().len()
        {
            output.clear();
            return;
        }

        for (destination, source) in output.as_mut_slice().iter_mut().zip(input.as_slice()) {
            *destination =
                if source.is_finite() { (*source * self.gain).clamp(-1.0, 1.0) } else { 0.0 };
        }
    }

    fn reset(&mut self) {}

    fn apply_command(&mut self, command: AudioGraphCommand) {
        if let AudioGraphCommand::SetMasterGain(gain) = command {
            self.set_gain(gain);
        }
    }
}

fn clear_first_output(outputs: &mut [AudioBlockMut<'_>]) {
    if let Some(output) = outputs.first_mut() {
        output.clear();
    }
}
