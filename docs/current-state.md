# RSTUDIO Current State

## Overview

RSTUDIO now provides a minimal real-time output foundation. The application can initialize the default CPAL output device, create an output stream, and render an oscillator through its callback.

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
audio callback -> Transport -> AudioGraph (tracks -> mixer -> master bus -> meter) -> output device
```

`AudioEngine` owns processing state and is moved into the CPAL callback. CPAL-specific device and stream management stays in `audio-engine::stream`; `dsp` has no CPAL dependency.

Audio rendering now passes through two fixed `Track` channels, then `MixerNode` and `MasterBus`.
Each track has its own oscillator source, channel gain, mute state, and preallocated output buffer.
Track names are configuration-only; the callback uses only a fixed array and `TrackId` comparisons.
CPAL's negotiated sample rate and channel count configure the graph before stream creation.
`AudioMeter` measures peak and RMS after the master bus; CPAL publishes the values as
non-blocking, best-effort events. The callback uses no allocations, locks, or system access.

`Transport` is owned by `AudioEngine`, not by the application or CPAL. It advances its sample
position only for rendered blocks while playing, and provides sample/second and beat/sample
conversion using the negotiated sample rate and BPM. Transport commands and best-effort state
updates use the existing bounded lock-free queues. It does not yet drive clips, automation, MIDI,
or graph scheduling.

`Timeline` and `AudioClip` now provide the non-real-time, sample-accurate project model for
placing future events on tracks. A clip records its ID, track ID, start position, and length;
timeline queries resolve active clips for a sample position. This model is intentionally outside
the callback because editable `Vec` storage can allocate. It does not yet trigger sources or read
audio files.

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
- Extensible multi-input/multi-output `AudioNode` contract; `MixerNode` supports a fixed number
  of input buses.

## Not Implemented Yet

- Input streams and recording.
- Dynamic graph routing, scheduled clip playback, and a timeline-to-audio snapshot mechanism.
- Device selection, device-change recovery, and negotiated fixed buffer size.
- MIDI, UI, project persistence, plugins, effects, and automation.
- Real-time performance benchmarks and hardware integration tests.
