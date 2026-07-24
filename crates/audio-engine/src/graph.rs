use common::error::Result;

use crate::{
    AudioBlockMut, AudioBuffer, AudioFormat, AudioGraphCommand, AudioNode, GainNode, MixerNode,
    OscillatorNode,
};

/// Minimal linear audio graph.
///
/// `Vec<Box<dyn AudioNode>>` is an intentional first-stage abstraction, not the final DAW graph
/// architecture. Its allocation and topology are fixed before the stream starts; processing only
/// iterates existing nodes. A future graph planner can use node input/output counts to allocate
/// and route multiple buses without changing the node contract.
pub struct AudioGraph {
    format: AudioFormat,
    nodes: Vec<Box<dyn AudioNode>>,
    intermediate_buffers: Vec<AudioBuffer>,
    maximum_block_frames: usize,
}

impl AudioGraph {
    /// Creates the initial linear processing chain:
    /// `OscillatorNode -> GainNode -> MixerNode -> output`.
    ///
    /// Intermediate buffers are allocated here, outside the audio callback.
    pub fn with_oscillator(
        format: AudioFormat,
        maximum_block_frames: usize,
        frequency: f32,
    ) -> Result<Self> {
        let nodes: Vec<Box<dyn AudioNode>> = vec![
            Box::new(OscillatorNode::new(frequency, format)),
            Box::new(GainNode::new(1.0)),
            Box::new(MixerNode::new(1)),
        ];
        let mut graph =
            Self { format, nodes, intermediate_buffers: Vec::new(), maximum_block_frames: 0 };
        graph.prepare(format, maximum_block_frames)?;
        Ok(graph)
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }

    /// Allocates fixed intermediate buffers before the audio stream begins.
    pub fn prepare(&mut self, format: AudioFormat, maximum_block_frames: usize) -> Result<()> {
        let mut intermediate_buffers = Vec::with_capacity(self.nodes.len().saturating_sub(1));
        for _ in 0..self.nodes.len().saturating_sub(1) {
            intermediate_buffers.push(AudioBuffer::new(maximum_block_frames, format)?);
        }
        self.format = format;
        for node in &mut self.nodes {
            node.prepare(format);
        }
        self.intermediate_buffers = intermediate_buffers;
        self.maximum_block_frames = maximum_block_frames;
        Ok(())
    }

    pub fn process(&mut self, output: &mut AudioBlockMut<'_>) {
        if output.format() != self.format || output.frames() > self.maximum_block_frames {
            output.clear();
            return;
        }

        output.clear();
        let last_node_index = self.nodes.len().saturating_sub(1);
        if self.nodes.is_empty() || self.intermediate_buffers.len() != last_node_index {
            return;
        }

        let frames = output.frames();
        for node_index in 0..self.nodes.len() {
            let node = &mut self.nodes[node_index];
            if node_index == 0 && node_index == last_node_index {
                node.process(&[], std::slice::from_mut(&mut *output));
            } else if node_index == 0 {
                let Some(mut node_output) =
                    self.intermediate_buffers[0].block_mut_for_frames(frames)
                else {
                    output.clear();
                    return;
                };
                node.process(&[], std::slice::from_mut(&mut node_output));
            } else if node_index == last_node_index {
                let Some(node_input) =
                    self.intermediate_buffers[node_index - 1].block_for_frames(frames)
                else {
                    output.clear();
                    return;
                };
                node.process(std::slice::from_ref(&node_input), std::slice::from_mut(&mut *output));
            } else {
                let (completed, remaining) = self.intermediate_buffers.split_at_mut(node_index);
                let Some(node_input) = completed[node_index - 1].block_for_frames(frames) else {
                    output.clear();
                    return;
                };
                let Some(mut node_output) = remaining[0].block_mut_for_frames(frames) else {
                    output.clear();
                    return;
                };
                node.process(
                    std::slice::from_ref(&node_input),
                    std::slice::from_mut(&mut node_output),
                );
            }
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
