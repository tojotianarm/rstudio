use audio_engine::AudioEngine;
use common::config::AppConfig;

#[test]
fn engine_can_start_and_stop() {
    let config = AppConfig::default();

    let mut engine = AudioEngine::new(config);

    assert!(!engine.is_running());

    engine.start();

    assert!(engine.is_running());

    engine.stop();

    assert!(!engine.is_running());
}