use std::{env, thread, time::Duration};

use audio_engine::{
    AudioClip, AudioCommand, AudioEngine, ClipId, ClipSource, CpalOutputStream, SampleId,
    SampleRegistryBuilder, SampleTime, Timeline, TrackId, WavLoader,
};
use common::config::AppConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = env::args().nth(1) else {
        return Err("usage: cargo run -p audio-engine --example mini_daw -- <file.wav>".into());
    };

    let mut registry = SampleRegistryBuilder::new();
    registry.insert(SampleId::new(0), WavLoader::load(path)?)?;
    let engine = AudioEngine::new(AppConfig::default())?.with_sample_registry(registry.build());
    let stream = CpalOutputStream::open_default(engine)?;

    let mut timeline = Timeline::new();
    timeline.add_clip(
        AudioClip::new(
            ClipId::new(1),
            TrackId::new(1),
            SampleTime::new(0),
            SampleTime::new(u64::MAX),
        )
        .with_source(ClipSource::AudioFile(SampleId::new(0))),
    );
    timeline.add_clip(AudioClip::new(
        ClipId::new(2),
        TrackId::new(2),
        SampleTime::new(0),
        SampleTime::new(u64::MAX),
    ));
    stream.snapshots().try_send(audio_engine::AudioSnapshot::compile(&timeline)?)?;
    stream.start()?;
    stream.commands().try_send(AudioCommand::Play)?;

    loop {
        while let Some(event) = stream.events().try_recv() {
            if let audio_engine::AudioEvent::MeterUpdate { peak, rms } = event {
                println!("peak={peak:.3}, rms={rms:.3}");
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}
