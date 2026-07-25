use crate::{AudioBlockMut, AudioFormat, PARAMETER_CAPACITY, ParameterId, ParameterStore};

pub trait AudioEffect: Send {
    fn prepare(&mut self, format: AudioFormat);
    fn process(
        &mut self,
        block: &mut AudioBlockMut<'_>,
        parameters: &mut ParameterStore<PARAMETER_CAPACITY>,
    );
    fn reset(&mut self);
}

pub struct GainEffect {
    parameter: ParameterId,
}
impl GainEffect {
    pub const fn new(parameter: ParameterId) -> Self {
        Self { parameter }
    }
}
impl AudioEffect for GainEffect {
    fn prepare(&mut self, _: AudioFormat) {}
    fn process(
        &mut self,
        block: &mut AudioBlockMut<'_>,
        parameters: &mut ParameterStore<PARAMETER_CAPACITY>,
    ) {
        for sample in block.as_mut_slice() {
            let gain = parameters.next_value(self.parameter).unwrap_or(1.0);
            *sample = if sample.is_finite() { (*sample * gain).clamp(-1.0, 1.0) } else { 0.0 };
        }
    }
    fn reset(&mut self) {}
}

pub enum EffectSlot {
    Empty,
    Gain(GainEffect),
}
pub struct EffectRack<const N: usize> {
    slots: [EffectSlot; N],
}
impl<const N: usize> EffectRack<N> {
    pub fn empty() -> Self {
        Self { slots: std::array::from_fn(|_| EffectSlot::Empty) }
    }
    pub fn set_gain(&mut self, index: usize, effect: GainEffect) -> bool {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = EffectSlot::Gain(effect);
            true
        } else {
            false
        }
    }
    pub fn prepare(&mut self, format: AudioFormat) {
        for slot in &mut self.slots {
            if let EffectSlot::Gain(effect) = slot {
                effect.prepare(format);
            }
        }
    }
    pub fn process(
        &mut self,
        block: &mut AudioBlockMut<'_>,
        parameters: &mut ParameterStore<PARAMETER_CAPACITY>,
    ) {
        for slot in &mut self.slots {
            if let EffectSlot::Gain(effect) = slot {
                effect.process(block, parameters);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioBuffer, AudioFormat, ParameterDescriptor};
    #[test]
    fn gain_rack_processes_in_order() {
        let format = AudioFormat::new(48_000, 1).expect("format");
        let id = ParameterId::new(99);
        let mut store = ParameterStore::<PARAMETER_CAPACITY>::new(&[ParameterDescriptor::new(
            id, 0.0, 2.0, 0.5, 0,
        )]);
        let mut rack = EffectRack::<2>::empty();
        assert!(rack.set_gain(0, GainEffect::new(id)));
        let mut buffer = AudioBuffer::new(1, format).expect("buffer");
        buffer.as_mut_slice()[0] = 1.0;
        rack.process(&mut buffer.block_mut(), &mut store);
        assert_eq!(buffer.as_slice()[0], 0.5);
    }
}
