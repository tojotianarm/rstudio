use dsp::oscillator::Oscillator;

#[test]
fn oscillator_generates_samples() {
    let mut osc = Oscillator::new(440.0, 44100.0);

    let sample1 = osc.next_sample();
    let sample2 = osc.next_sample();

    assert_ne!(sample1, sample2);
}