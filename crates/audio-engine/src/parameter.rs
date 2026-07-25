/// Stable key for a real-time parameter slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterId(u16);

impl ParameterId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u16 {
        self.0
    }
}

pub type ParameterValue = f32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterDescriptor {
    pub id: ParameterId,
    pub minimum: ParameterValue,
    pub maximum: ParameterValue,
    pub default: ParameterValue,
    pub smoothing_samples: usize,
}

impl ParameterDescriptor {
    pub const fn new(
        id: ParameterId,
        minimum: f32,
        maximum: f32,
        default: f32,
        smoothing_samples: usize,
    ) -> Self {
        Self { id, minimum, maximum, default, smoothing_samples }
    }
}

#[derive(Clone, Copy)]
struct Parameter {
    descriptor: ParameterDescriptor,
    current: f32,
    target: f32,
    step: f32,
    remaining: usize,
}

impl Parameter {
    fn new(descriptor: ParameterDescriptor) -> Self {
        let value = clamp(descriptor.default, descriptor);
        Self { descriptor, current: value, target: value, step: 0.0, remaining: 0 }
    }

    fn set_target(&mut self, value: f32) {
        self.target = clamp(value, self.descriptor);
        self.remaining = self.descriptor.smoothing_samples;
        self.step = if self.remaining == 0 {
            0.0
        } else {
            (self.target - self.current) / self.remaining as f32
        };
        if self.remaining == 0 {
            self.current = self.target;
        }
    }

    fn next_value(&mut self) -> f32 {
        if self.remaining > 0 {
            self.current += self.step;
            self.remaining -= 1;
            if self.remaining == 0 {
                self.current = self.target;
            }
        }
        self.current
    }
}

/// Bounded parameter storage used only by the audio thread.
pub struct ParameterStore<const N: usize> {
    parameters: [Option<Parameter>; N],
}

impl<const N: usize> ParameterStore<N> {
    pub fn new(descriptors: &[ParameterDescriptor]) -> Self {
        let mut parameters = [None; N];
        for (slot, descriptor) in parameters.iter_mut().zip(descriptors.iter().copied()) {
            *slot = Some(Parameter::new(descriptor));
        }
        Self { parameters }
    }
    pub fn set_target(&mut self, id: ParameterId, value: f32) -> bool {
        if let Some(parameter) =
            self.parameters.iter_mut().flatten().find(|parameter| parameter.descriptor.id == id)
        {
            parameter.set_target(value);
            true
        } else {
            false
        }
    }
    pub fn next_value(&mut self, id: ParameterId) -> Option<f32> {
        self.parameters
            .iter_mut()
            .flatten()
            .find(|parameter| parameter.descriptor.id == id)
            .map(Parameter::next_value)
    }
    pub fn current_value(&self, id: ParameterId) -> Option<f32> {
        self.parameters
            .iter()
            .flatten()
            .find(|parameter| parameter.descriptor.id == id)
            .map(|parameter| parameter.current)
    }
}

fn clamp(value: f32, descriptor: ParameterDescriptor) -> f32 {
    if value.is_finite() {
        value.clamp(descriptor.minimum, descriptor.maximum)
    } else {
        descriptor.default
    }
}

pub const MASTER_GAIN_PARAMETER: ParameterId = ParameterId::new(0);
pub const TRACK_ONE_GAIN_PARAMETER: ParameterId = ParameterId::new(1);
pub const TRACK_TWO_GAIN_PARAMETER: ParameterId = ParameterId::new(2);
pub const SYNTH_FREQUENCY_PARAMETER: ParameterId = ParameterId::new(3);
pub const SYNTH_GAIN_PARAMETER: ParameterId = ParameterId::new(4);
pub const SYNTH_ATTACK_PARAMETER: ParameterId = ParameterId::new(5);
pub const SYNTH_DECAY_PARAMETER: ParameterId = ParameterId::new(6);
pub const SYNTH_SUSTAIN_PARAMETER: ParameterId = ParameterId::new(7);
pub const SYNTH_RELEASE_PARAMETER: ParameterId = ParameterId::new(8);
pub const PARAMETER_CAPACITY: usize = 9;
pub const ENGINE_PARAMETERS: [ParameterDescriptor; PARAMETER_CAPACITY] = [
    ParameterDescriptor::new(MASTER_GAIN_PARAMETER, 0.0, 2.0, 1.0, 64),
    ParameterDescriptor::new(TRACK_ONE_GAIN_PARAMETER, 0.0, 2.0, 1.0, 64),
    ParameterDescriptor::new(TRACK_TWO_GAIN_PARAMETER, 0.0, 2.0, 1.0, 64),
    ParameterDescriptor::new(SYNTH_FREQUENCY_PARAMETER, 0.0, 20_000.0, 440.0, 64),
    ParameterDescriptor::new(SYNTH_GAIN_PARAMETER, 0.0, 1.0, 1.0, 64),
    ParameterDescriptor::new(SYNTH_ATTACK_PARAMETER, 0.0, 10.0, 0.002, 64),
    ParameterDescriptor::new(SYNTH_DECAY_PARAMETER, 0.0, 10.0, 0.050, 64),
    ParameterDescriptor::new(SYNTH_SUSTAIN_PARAMETER, 0.0, 1.0, 0.8, 64),
    ParameterDescriptor::new(SYNTH_RELEASE_PARAMETER, 0.0, 10.0, 0.010, 64),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn store_clamps_and_smooths_parameter_changes() {
        let descriptor = ParameterDescriptor::new(ParameterId::new(42), 0.0, 1.0, 0.0, 4);
        let mut store = ParameterStore::<1>::new(&[descriptor]);
        assert!(store.set_target(ParameterId::new(42), 2.0));
        assert!((store.next_value(ParameterId::new(42)).unwrap_or_default() - 0.25).abs() < 1e-6);
        for _ in 0..3 {
            let _ = store.next_value(ParameterId::new(42));
        }
        assert_eq!(store.current_value(ParameterId::new(42)), Some(1.0));
    }
}
