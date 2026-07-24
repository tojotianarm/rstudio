use audio_engine::{AudioBuffer, AudioEngine, AudioFormat, AudioGraph};
use common::config::AppConfig;

#[test]
fn engine_processes_an_interleaved_audio_block() {
    let mut engine =
        AudioEngine::new(AppConfig::default()).expect("default configuration is valid");
    let format = AudioFormat::new(44_100, 2).expect("valid stereo format");
    engine.configure_output(format, 128).expect("output configuration is valid");
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

#[test]
fn negotiated_output_format_reconfigures_the_graph() {
    let mut engine =
        AudioEngine::new(AppConfig::default()).expect("default configuration is valid");
    let negotiated_format = AudioFormat::new(48_000, 2).expect("valid stereo format");
    engine.configure_output(negotiated_format, 256).expect("output configuration is valid");
    let mut output = AudioBuffer::new(256, negotiated_format).expect("buffer dimensions are valid");

    engine.start();
    engine.process(&mut output.block_mut());

    assert_eq!(engine.output_format(), negotiated_format);
    assert!(output.as_slice().iter().any(|sample| *sample != 0.0));
}

#[test]
fn linear_graph_renders_an_oscillator_to_its_output_bus() {
    let format = AudioFormat::new(48_000, 2).expect("valid stereo format");
    let mut graph = AudioGraph::with_oscillator(format, 440.0);
    let mut output = AudioBuffer::new(64, format).expect("buffer dimensions are valid");

    graph.process(&mut output.block_mut());

    assert!(output.as_slice().iter().any(|sample| *sample != 0.0));
    assert!(output.as_slice().chunks_exact(2).all(|frame| frame[0] == frame[1]));
}
