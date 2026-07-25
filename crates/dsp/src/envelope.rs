/// Stages of a deterministic ADSR amplitude envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdsrStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Time-domain parameters for an ADSR envelope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrParameters {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain_level: f32,
    pub release_seconds: f32,
}

impl Default for AdsrParameters {
    fn default() -> Self {
        Self {
            attack_seconds: 0.002,
            decay_seconds: 0.050,
            sustain_level: 0.8,
            release_seconds: 0.010,
        }
    }
}

/// Allocation-free ADSR envelope advanced one sample at a time.
pub struct Adsr {
    parameters: AdsrParameters,
    sample_rate: f32,
    stage: AdsrStage,
    level: f32,
    release_step: f32,
}

impl Adsr {
    pub fn new(sample_rate: f32, parameters: AdsrParameters) -> Self {
        Self {
            parameters: sanitize_parameters(parameters),
            sample_rate: sanitize_sample_rate(sample_rate),
            stage: AdsrStage::Idle,
            level: 0.0,
            release_step: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sanitize_sample_rate(sample_rate);
    }

    pub fn set_parameters(&mut self, parameters: AdsrParameters) {
        self.parameters = sanitize_parameters(parameters);
    }

    pub fn note_on(&mut self) {
        self.stage = AdsrStage::Attack;
        self.level = 0.0;
        self.release_step = 0.0;
    }

    pub fn note_off(&mut self) {
        if self.stage == AdsrStage::Idle || self.stage == AdsrStage::Release {
            return;
        }
        let release_samples = samples_for(self.parameters.release_seconds, self.sample_rate);
        self.release_step =
            if release_samples == 0 { self.level } else { self.level / release_samples as f32 };
        self.stage = AdsrStage::Release;
    }

    pub fn next_amplitude(&mut self) -> f32 {
        match self.stage {
            AdsrStage::Idle => self.level = 0.0,
            AdsrStage::Attack => self.advance_attack(),
            AdsrStage::Decay => self.advance_decay(),
            AdsrStage::Sustain => self.level = self.parameters.sustain_level,
            AdsrStage::Release => self.advance_release(),
        }
        self.level = if self.level.is_finite() { self.level.clamp(0.0, 1.0) } else { 0.0 };
        self.level
    }

    pub const fn stage(&self) -> AdsrStage {
        self.stage
    }

    pub fn is_active(&self) -> bool {
        self.stage != AdsrStage::Idle
    }

    pub fn reset(&mut self) {
        self.stage = AdsrStage::Idle;
        self.level = 0.0;
        self.release_step = 0.0;
    }

    fn advance_attack(&mut self) {
        let samples = samples_for(self.parameters.attack_seconds, self.sample_rate);
        if samples == 0 {
            self.level = 1.0;
            self.stage = AdsrStage::Decay;
            return;
        }
        self.level += 1.0 / samples as f32;
        if self.level >= 1.0 {
            self.level = 1.0;
            self.stage = AdsrStage::Decay;
        }
    }

    fn advance_decay(&mut self) {
        let target = self.parameters.sustain_level;
        let samples = samples_for(self.parameters.decay_seconds, self.sample_rate);
        if samples == 0 {
            self.level = target;
            self.stage = AdsrStage::Sustain;
            return;
        }
        self.level -= (1.0 - target) / samples as f32;
        if self.level <= target {
            self.level = target;
            self.stage = AdsrStage::Sustain;
        }
    }

    fn advance_release(&mut self) {
        self.level -= self.release_step;
        if self.level <= 0.0 || !self.level.is_finite() {
            self.level = 0.0;
            self.stage = AdsrStage::Idle;
        }
    }
}

fn sanitize_parameters(parameters: AdsrParameters) -> AdsrParameters {
    AdsrParameters {
        attack_seconds: sanitize_seconds(parameters.attack_seconds),
        decay_seconds: sanitize_seconds(parameters.decay_seconds),
        sustain_level: if parameters.sustain_level.is_finite() {
            parameters.sustain_level.clamp(0.0, 1.0)
        } else {
            0.0
        },
        release_seconds: sanitize_seconds(parameters.release_seconds),
    }
}

fn sanitize_seconds(seconds: f32) -> f32 {
    if seconds.is_finite() { seconds.max(0.0) } else { 0.0 }
}

fn sanitize_sample_rate(sample_rate: f32) -> f32 {
    if sample_rate.is_finite() && sample_rate > 0.0 { sample_rate } else { 1.0 }
}

fn samples_for(seconds: f32, sample_rate: f32) -> usize {
    (seconds * sample_rate).round() as usize
}

#[cfg(test)]
mod tests {
    use super::{Adsr, AdsrParameters, AdsrStage};

    fn envelope() -> Adsr {
        Adsr::new(
            100.0,
            AdsrParameters {
                attack_seconds: 0.02,
                decay_seconds: 0.02,
                sustain_level: 0.5,
                release_seconds: 0.02,
            },
        )
    }

    #[test]
    fn adsr_transitions_from_attack_to_decay_and_sustain() {
        let mut adsr = envelope();
        adsr.note_on();

        assert_eq!(adsr.stage(), AdsrStage::Attack);
        let first = adsr.next_amplitude();
        assert!((0.0..=1.0).contains(&first));
        adsr.next_amplitude();
        assert_eq!(adsr.stage(), AdsrStage::Decay);
        adsr.next_amplitude();
        adsr.next_amplitude();
        assert_eq!(adsr.stage(), AdsrStage::Sustain);
    }

    #[test]
    fn adsr_release_reaches_idle_with_finite_bounded_values() {
        let mut adsr = envelope();
        adsr.note_on();
        for _ in 0..4 {
            let _ = adsr.next_amplitude();
        }
        adsr.note_off();
        assert_eq!(adsr.stage(), AdsrStage::Release);

        for _ in 0..4 {
            let value = adsr.next_amplitude();
            assert!(value.is_finite());
            assert!((0.0..=1.0).contains(&value));
        }
        assert_eq!(adsr.stage(), AdsrStage::Idle);
        assert!(!adsr.is_active());
    }
}
