use crate::error::{RStudioError, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub sample_rate: u32,
    pub buffer_size: usize,
}

impl AppConfig {
    pub const MIN_SAMPLE_RATE: u32 = 8_000;
    pub const MAX_SAMPLE_RATE: u32 = 384_000;
    pub const MAX_BUFFER_SIZE: usize = 65_536;

    pub fn validate(&self) -> Result<()> {
        if !(Self::MIN_SAMPLE_RATE..=Self::MAX_SAMPLE_RATE).contains(&self.sample_rate) {
            return Err(RStudioError::InvalidConfiguration {
                field: "sample_rate",
                reason: "must be between 8_000 and 384_000 Hz",
            });
        }
        if self.buffer_size == 0 || self.buffer_size > Self::MAX_BUFFER_SIZE {
            return Err(RStudioError::InvalidConfiguration {
                field: "buffer_size",
                reason: "must be between 1 and 65_536 frames",
            });
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { sample_rate: 44_100, buffer_size: 512 }
    }
}
