use audio_engine::{
    AudioBuffer, AudioClip, AudioEngine, AudioFormat, AudioGraph, AudioGraphCommand, AudioMeter,
    AudioNode, AudioSnapshot, ClipId, GainNode, MasterBus, MixerNode, SampleTime, Timeline,
    TrackId,
};
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
    let mut graph =
        AudioGraph::with_oscillator(format, 64, 440.0).expect("graph buffers are valid");
    let mut output = AudioBuffer::new(64, format).expect("buffer dimensions are valid");

    graph.process(&mut output.block_mut(), SampleTime::new(0));

    assert!(output.as_slice().iter().any(|sample| *sample != 0.0));
    assert!(output.as_slice().chunks_exact(2).all(|frame| frame[0] == frame[1]));
}

#[test]
fn gain_node_preserves_unity_gain() {
    assert_eq!(process_gain(1.0), [0.5, -0.5]);
}

#[test]
fn gain_node_scales_samples() {
    assert_eq!(process_gain(0.5), [0.25, -0.25]);
}

#[test]
fn gain_node_silences_zero_gain() {
    assert_eq!(process_gain(0.0), [0.0, 0.0]);
}

#[test]
fn gain_node_sanitizes_invalid_input_and_gain() {
    let format = AudioFormat::new(48_000, 1).expect("valid mono format");
    let mut input = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    input.as_mut_slice().copy_from_slice(&[f32::NAN, f32::INFINITY]);
    let mut output = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    let mut gain = GainNode::new(1.0);
    gain.set_gain(f32::NAN);

    let mut output_block = output.block_mut();
    gain.process(&[input.block()], std::slice::from_mut(&mut output_block));

    assert_eq!(gain.gain(), 0.0);
    assert_eq!(output.as_slice(), &[0.0, 0.0]);
}

#[test]
fn mixer_node_sums_two_input_buses() {
    let format = AudioFormat::new(48_000, 1).expect("valid mono format");
    let mut input_a = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    input_a.as_mut_slice().copy_from_slice(&[0.2, 0.3]);
    let mut input_b = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    input_b.as_mut_slice().copy_from_slice(&[0.1, -0.1]);
    let mut output = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    let mut mixer = MixerNode::new(2);

    let inputs = [input_a.block(), input_b.block()];
    let mut output_block = output.block_mut();
    mixer.process(&inputs, std::slice::from_mut(&mut output_block));

    assert!((output.as_slice()[0] - 0.3).abs() < 1.0e-6);
    assert!((output.as_slice()[1] - 0.2).abs() < 1.0e-6);
}

#[test]
fn graph_routes_oscillator_through_master_bus() {
    let format = AudioFormat::new(48_000, 1).expect("valid mono format");
    let mut graph =
        AudioGraph::with_oscillator(format, 64, 440.0).expect("graph buffers are valid");
    let mut output = AudioBuffer::new(64, format).expect("buffer dimensions are valid");
    graph.apply_command(AudioGraphCommand::SetMasterGain(0.0));

    graph.process(&mut output.block_mut(), SampleTime::new(0));

    assert!(output.as_slice().iter().all(|sample| *sample == 0.0));
}

#[test]
fn master_bus_preserves_unity_gain() {
    assert_eq!(process_master_bus(1.0, [0.5, -0.5]), [0.5, -0.5]);
}

#[test]
fn master_bus_scales_signal() {
    assert_eq!(process_master_bus(0.5, [0.5, -0.5]), [0.25, -0.25]);
}

#[test]
fn master_bus_saturates_output() {
    assert_eq!(process_master_bus(1.0, [2.0, -2.0]), [1.0, -1.0]);
}

#[test]
fn audio_meter_measures_peak_and_rms_for_a_block() {
    let format = AudioFormat::new(48_000, 1).expect("valid mono format");
    let mut input = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    input.as_mut_slice().copy_from_slice(&[0.5, -0.5]);
    let mut meter = AudioMeter::default();

    meter.measure(input.block());

    assert_eq!(meter.peak(), 0.5);
    assert!((meter.rms() - 0.5).abs() < 1.0e-6);
    assert!(meter.peak().is_finite());
    assert!(meter.rms().is_finite());
}

#[test]
fn track_mixer_master_pipeline_outputs_valid_samples_within_preallocated_capacity() {
    let format = AudioFormat::new(48_000, 2).expect("valid stereo format");
    let mut graph =
        AudioGraph::with_oscillator(format, 64, 440.0).expect("graph buffers are valid");
    let mut output = AudioBuffer::new(64, format).expect("buffer dimensions are valid");

    graph.process(&mut output.block_mut(), SampleTime::new(0));

    assert_eq!(output.frames(), 64);
    assert_eq!(output.format(), format);
    assert!(output.as_slice().iter().all(|sample| sample.is_finite()));
    assert!(output.as_slice().iter().all(|sample| (-1.0..=1.0).contains(sample)));
    assert!(graph.meter().peak().is_finite());
    assert!(graph.meter().rms().is_finite());
}

#[test]
fn clip_player_silences_inactive_tracks_and_plays_active_tracks() {
    let format = AudioFormat::new(48_000, 1).expect("valid mono format");
    let mut graph =
        AudioGraph::with_oscillator(format, 64, 440.0).expect("graph buffers are valid");
    let mut output = AudioBuffer::new(64, format).expect("buffer dimensions are valid");

    graph.set_snapshot(AudioSnapshot::compile(&Timeline::new()).expect("empty timeline compiles"));
    graph.process(&mut output.block_mut(), SampleTime::new(0));
    assert!(output.as_slice().iter().all(|sample| *sample == 0.0));

    let mut timeline = Timeline::new();
    timeline.add_clip(AudioClip::new(
        ClipId::new(1),
        TrackId::new(1),
        SampleTime::new(0),
        SampleTime::new(1_000),
    ));
    graph.set_snapshot(AudioSnapshot::compile(&timeline).expect("valid timeline compiles"));
    graph.process(&mut output.block_mut(), SampleTime::new(0));

    assert!(output.as_slice().iter().any(|sample| *sample != 0.0));
    assert!(output.as_slice().iter().all(|sample| sample.is_finite()));
}

fn process_gain(gain_value: f32) -> [f32; 2] {
    let format = AudioFormat::new(48_000, 1).expect("valid mono format");
    let mut input = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    input.as_mut_slice().copy_from_slice(&[0.5, -0.5]);
    let mut output = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    let mut gain = GainNode::new(gain_value);

    let mut output_block = output.block_mut();
    gain.process(&[input.block()], std::slice::from_mut(&mut output_block));

    [output.as_slice()[0], output.as_slice()[1]]
}

fn process_master_bus(gain_value: f32, input_samples: [f32; 2]) -> [f32; 2] {
    let format = AudioFormat::new(48_000, 1).expect("valid mono format");
    let mut input = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    input.as_mut_slice().copy_from_slice(&input_samples);
    let mut output = AudioBuffer::new(2, format).expect("buffer dimensions are valid");
    let mut master_bus = MasterBus::new(gain_value);

    let mut output_block = output.block_mut();
    master_bus.process(&[input.block()], std::slice::from_mut(&mut output_block));

    [output.as_slice()[0], output.as_slice()[1]]
}
