#[derive(Debug, Clone)]
pub struct AppConfig {
    pub sample_rate: u32,
    pub buffer_size: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            buffer_size: 512,
        }
    }
}