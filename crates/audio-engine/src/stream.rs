use crate::{AudioBuffer, AudioEngine};

pub struct AudioProcessor {
    engine: AudioEngine,
}

impl AudioProcessor {
    pub fn new(engine: AudioEngine) -> Self {
        Self { engine }
    }

    pub fn process_block(&mut self, buffer: &mut AudioBuffer) {
        self.engine.process(buffer);
    }
}
