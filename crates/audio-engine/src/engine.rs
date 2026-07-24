use common::config::AppConfig;

pub struct AudioEngine {
    config: AppConfig,
    running: bool,
}

impl AudioEngine {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            running: false,
        }
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }
}