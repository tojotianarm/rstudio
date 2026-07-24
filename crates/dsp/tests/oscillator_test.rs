use dsp::oscillator::Oscillator;

#[test]
fn oscillator_generates_samples() {
    let mut osc = Oscillator::new(440.0, 44100.0);

    let sample1 = osc.next_sample();
    let sample2 = osc.next_sample();

    assert_ne!(sample1, sample2);
}

#[test]
fn oscillator_reset_restores_its_initial_phase() {
    let mut oscillator = Oscillator::new(440.0, 44_100.0);
    let first_sample = oscillator.next_sample();
    oscillator.next_sample();
    oscillator.reset();

    assert_eq!(oscillator.next_sample(), first_sample);
}
