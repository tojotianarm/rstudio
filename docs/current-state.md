# RSTUDIO Current State

## Overview

RSTUDIO now provides an audio-engine MVP: the application can initialize the default CPAL output
device, schedule synth and preloaded WAV clips, and render them through the same real-time graph.

## Current Architecture

```text
app
 |
 v
AudioEngine API (hardware-independent)
 |
 v
CPAL output stream adapter
 |
 v
audio callback -> Transport -> Snapshot scheduler -> AudioGraph -> output device
```

`AudioEngine` owns processing state and is moved into the CPAL callback. CPAL-specific device and stream management stays in `audio-engine::stream`; `dsp` has no CPAL dependency.

Audio rendering now passes through two fixed `Track` channels, then `MixerNode` and `MasterBus`.
Each track has an eight-slot, preallocated `VoiceManager`; every active clip is assigned an
audio-thread-owned `SimpleSynth` instrument, allowing overlapping clips to render simultaneously.
`SimpleSynth` combines the oscillator DSP primitive with a deterministic ADSR envelope. Voice
slots are activated and released in place, without callback-time allocation, locks, or dynamic
track lookup. A slot remains occupied during ADSR release after a clip ends and is recycled only
after the envelope reaches idle. Each track also has channel gain, mute state, and a preallocated
output buffer.
`EventScheduler` produces a fixed, ordered list of voice start/stop events for each callback
block. The voice pool consumes each event before its exact output frame, so clips can start and
end sample-accurately inside a CPAL block rather than only at a block boundary.
The initial fixed-capacity `ParameterStore` supplies smoothed frequency, gain and ADSR targets to
`SimpleSynth` through the generic `SetParameter` command, with no blocking synchronization.
Each track and the master path now own an in-place fixed-capacity `EffectRack`; empty slots cost
only a bounded branch. `GainEffect` is the initial parameter-driven validation effect.

The audio-file foundation is available outside the callback: `WavLoader` decodes WAV into
`PcmAudioBuffer`, and `SampleRegistryBuilder` constructs an immutable fixed-capacity
`SampleRegistry` before it is attached to `AudioEngine`. Timeline clips and snapshots can carry
`ClipSource::AudioFile(SampleId)`. `SamplePlayer` performs preloaded PCM playback and simple
linear sample-rate conversion without allocation. The registry is transferred into `AudioGraph`
before CPAL stream creation and is immutable in the callback. A voice now selects synth or PCM
from `ClipSource`; sample voices resolve `SampleId` in the registry, render preloaded frames, and
recycle their fixed slot at end of buffer. Audio-file clips therefore follow the same track,
mixer, effects, master bus, meter, and CPAL path as synth voices.
Track names are configuration-only; the callback uses only a fixed array and `TrackId` comparisons.
CPAL's negotiated sample rate and channel count configure the graph before stream creation.
`AudioMeter` measures peak and RMS after the master bus; CPAL publishes the values as
non-blocking, best-effort events. The callback uses no allocations, locks, or system access.

`Transport` is owned by `AudioEngine`, not by the application or CPAL. It advances its sample
position only for rendered blocks while playing, and provides sample/second and beat/sample
conversion using the negotiated sample rate and BPM. Transport commands and best-effort state
updates use the existing bounded lock-free queues. Its current position drives immutable-snapshot
clip scheduling; automation and MIDI remain future work.

`Timeline` and `AudioClip` provide the non-real-time, sample-accurate project model for placing
events on tracks. `SnapshotCompiler` converts that editable data into an immutable fixed-capacity
`AudioSnapshot`, transferred through a bounded lock-free queue. The callback consults only the
current snapshot via `ClipScheduler`; it never traverses the editable timeline. Active clips start
either a synth or a preloaded PCM sample source for their track, while inactive tracks render
silence. Audio files are decoded only before stream construction.

## Implemented Features

- Validated application audio configuration.
- Interleaved audio buffers with explicit frame count, channel count, and sample rate.
- Minimal real-time audio callback for `f32`, `i16`, and `u16` output devices.
- Bounded lock-free command and event queues between the application and audio callback.
- Typed stream creation and start errors.
- Two fixed tracks routed as `track -> mixer -> master bus -> output`, with preallocated buffers.
- Targeted track gain, mute, and solo-state commands through the bounded command queue.
- Master gain control through the bounded real-time command queue, plus saturated node output.
- Per-block peak/RMS metering emitted through the bounded event queue.
- Sample-accurate transport state with play, pause, stop, seek, BPM, and time conversions.
- Sample-accurate clips and non-real-time timeline queries, ready for a future sequencer.
- Immutable fixed-capacity snapshots, lock-free snapshot publication, and clip scheduling.
- Fixed-capacity polyphonic voice pools per track, including overlapping clip rendering.
- Sample-accurate start/stop scheduling inside a callback block through fixed event tables.
- `Instrument` abstraction with a first polyphonic `SimpleSynth` and allocation-free ADSR.
- Fixed real-time parameter store with descriptors, clamping, targets and deterministic smoothing.
- WAV loading, immutable preloaded sample registry, audio-file clips, and `SampleVoice` PCM playback.
- Runnable `mini_daw` example: WAV loading, synth/sample mixing, transport start, and meter events.
- Application-side `Project` and `EngineController` APIs: editable tracks/clips and WAV loading
  stay outside the callback; transport, track controls, snapshots, and meter events cross the
  bounded engine queues only.
- Extensible multi-input/multi-output `AudioNode` contract; `MixerNode` supports a fixed number
  of input buses.

## Not Implemented Yet

- Input streams and recording.
- Dynamic graph routing, voice stealing, and scalable snapshot capacities.
- Runtime voice-capacity changes, voice stealing, additional instrument families, and dynamic graph routing.
- More than two tracks; the current UI-facing `Project` reflects the fixed two-track MVP graph.
- Device selection, device-change recovery, and negotiated fixed buffer size.
- MIDI, UI, project persistence, plugins, effects, and automation.
- Real-time performance benchmarks and hardware integration tests.

## Running the audio-file demonstration

With an available default output device and a WAV file, run:

```bash
cargo run -p audio-engine --example mini_daw -- path/to/file.wav
```

The example preloads the WAV into `SampleRegistry`, schedules it on track 1, adds a synth clip on
track 2, starts transport, and prints best-effort peak/RMS events. File decoding happens before
the stream is created; the callback only reads immutable PCM data.
