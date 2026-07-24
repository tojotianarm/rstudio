use audio_engine::{AudioBuffer, AudioEngine};
use common::config::AppConfig;

#[test]
fn engine_processes_audio_buffer() {
    let config = AppConfig::default();

    let mut engine = AudioEngine::new(config);

    let mut buffer = AudioBuffer::new(128);

    engine.process(&mut buffer);

    assert!(buffer.as_slice().iter().any(|x| *x != 0.0));
}