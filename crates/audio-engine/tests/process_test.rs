use audio_engine::{AudioBuffer, AudioEngine, AudioFormat};
use common::config::AppConfig;

#[test]
fn engine_processes_an_interleaved_audio_block() {
    let mut engine =
        AudioEngine::new(AppConfig::default()).expect("default configuration is valid");
    let format = AudioFormat::new(44_100, 2).expect("valid stereo format");
    let mut buffer = AudioBuffer::new(128, format).expect("buffer dimensions are valid");
    engine.start();
    engine.process(&mut buffer.block_mut());
    assert_eq!(buffer.frames(), 128);
    assert_eq!(buffer.format(), format);
    assert!(buffer.as_slice().iter().any(|sample| *sample != 0.0));
    assert!(buffer.as_slice().chunks_exact(2).all(|frame| frame[0] == frame[1]));
}

#[test]
fn audio_buffer_rejects_invalid_format_and_overflow() {
    assert!(AudioFormat::new(0, 2).is_err());

    let format = AudioFormat::new(44_100, 2).expect("valid stereo format");
    assert!(AudioBuffer::new(usize::MAX, format).is_err());
}
