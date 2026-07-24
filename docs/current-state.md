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
audio callback -> AudioGraph (oscillator -> gain -> mixer) -> output device
```

`AudioEngine` owns processing state and is moved into the CPAL callback. CPAL-specific device and stream management stays in `audio-engine::stream`; `dsp` has no CPAL dependency.

Audio rendering now passes through a minimal `AudioGraph` with `OscillatorNode`, `GainNode`, and
`MixerNode`. CPAL's negotiated sample rate and channel count configure the graph and preallocate
its intermediate buffers before stream creation. The callback uses no allocations, locks, or system
access; an oversized callback is silenced and reported as an event rather than resizing memory.

## Implemented Features

- Validated application audio configuration.
- Interleaved audio buffers with explicit frame count, channel count, and sample rate.
- Minimal real-time audio callback for `f32`, `i16`, and `u16` output devices.
- Bounded lock-free command and event queues between the application and audio callback.
- Typed stream creation and start errors.
- Linear `oscillator -> gain -> mixer -> output` graph with preallocated intermediate buffers.
- Gain control through the bounded real-time command queue, plus saturated gain and mix output.
- Extensible multi-input/multi-output `AudioNode` contract; `MixerNode` supports a fixed number
  of input buses.

## Not Implemented Yet

- Input streams and recording.
- Audio graph, tracks, mixer, transport timeline, or routing.
- Device selection, device-change recovery, and negotiated fixed buffer size.
- MIDI, UI, project persistence, plugins, effects, and automation.
- Real-time performance benchmarks and hardware integration tests.
