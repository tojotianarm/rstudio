use crate::{AudioBlockMut, AudioFormat, AudioGraphCommand, AudioNode, OscillatorNode};

/// Minimal linear audio graph.
///
/// `Vec<Box<dyn AudioNode>>` is an intentional first-stage abstraction, not the final DAW graph
/// architecture. Its allocation and topology are fixed before the stream starts; processing only
/// iterates existing nodes. A future graph planner can use node input/output counts to allocate
/// and route multiple buses without changing the node contract.
pub struct AudioGraph {
    format: AudioFormat,
    nodes: Vec<Box<dyn AudioNode>>,
}

impl AudioGraph {
    pub fn with_oscillator(format: AudioFormat, frequency: f32) -> Self {
        let nodes: Vec<Box<dyn AudioNode>> = vec![Box::new(OscillatorNode::new(frequency, format))];
        Self { format, nodes }
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }

    pub fn prepare(&mut self, format: AudioFormat) {
        self.format = format;
        for node in &mut self.nodes {
            node.prepare(format);
        }
    }

    pub fn process(&mut self, output: &mut AudioBlockMut<'_>) {
        if output.format() != self.format {
            output.clear();
            return;
        }

        output.clear();
        for node in &mut self.nodes {
            node.process(&[], std::slice::from_mut(&mut *output));
        }
    }

    pub fn reset(&mut self) {
        for node in &mut self.nodes {
            node.reset();
        }
    }

    pub fn apply_command(&mut self, command: AudioGraphCommand) {
        for node in &mut self.nodes {
            node.apply_command(command);
        }
    }
}
