use audio_engine::{AudioEngine, CpalOutputStream};
use common::config::AppConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::default();

    let engine = AudioEngine::new(config)?;
    let stream = CpalOutputStream::open_default(engine)?;
    let device = stream.device_info();
    stream.start()?;
    println!(
        "RSTUDIO audio output started: {} Hz, {} channels, {:?} buffer support.",
        device.sample_rate(),
        device.channels(),
        device.buffer_size()
    );
    std::thread::park();
    Ok(())
}
