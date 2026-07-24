pub struct AudioBuffer {
    samples: Vec<f32>,
}

impl AudioBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            samples: vec![0.0; size],
        }
    }

    pub fn clear(&mut self) {
        self.samples.fill(0.0);
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.samples
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.samples
    }
}