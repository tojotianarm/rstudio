use audio_engine::AudioEngine;
use common::config::AppConfig;

#[test]
fn engine_can_start_and_stop() {
    let config = AppConfig::default();

    let mut engine = AudioEngine::new(config).expect("default configuration is valid");

    assert!(!engine.is_running());

    engine.start();

    assert!(engine.is_running());

    engine.stop();

    assert!(!engine.is_running());
}

#[test]
fn engine_rejects_invalid_configuration() {
    let config = AppConfig { sample_rate: 0, buffer_size: 512 };
    assert!(AudioEngine::new(config).is_err());
}
