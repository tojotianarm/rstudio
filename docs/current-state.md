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
audio callback -> DSP oscillator -> output device
```

`AudioEngine` owns processing state and is moved into the CPAL callback. CPAL-specific device and stream management stays in `audio-engine::stream`; `dsp` has no CPAL dependency.

Audio rendering now passes through a minimal `AudioGraph` with one `OscillatorNode`. CPAL's
negotiated sample rate and channel count configure the graph before stream creation. The callback
uses only preallocated graph buffers; an oversized callback is silenced and reported as an event
rather than resizing memory.

## Implemented Features

- Validated application audio configuration.
- Interleaved audio buffers with explicit frame count, channel count, and sample rate.
- Minimal real-time audio callback for `f32`, `i16`, and `u16` output devices.
- Bounded lock-free command and event queues between the application and audio callback.
- Typed stream creation and start errors.
- Linear audio graph and extensible multi-input/multi-output `AudioNode` contract.

## Not Implemented Yet

- Input streams and recording.
- Audio graph, tracks, mixer, transport timeline, or routing.
- Device selection, device-change recovery, and negotiated fixed buffer size.
- MIDI, UI, project persistence, plugins, effects, and automation.
- Real-time performance benchmarks and hardware integration tests.
