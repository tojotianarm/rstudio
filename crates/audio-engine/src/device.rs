use cpal::traits::{DeviceTrait, HostTrait};

pub fn default_output_device() -> Option<cpal::Device> {
    let host = cpal::default_host();

    host.default_output_device()
}