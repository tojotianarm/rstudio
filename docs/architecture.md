# RSTUDIO Architecture

## Overview

RSTUDIO is a modern Digital Audio Workstation (DAW) written in Rust.

The project aims to create a modular, performant and extensible music production environment.

Main goals:

- Real-time audio processing
- Low latency
- Modular architecture
- Cross-platform support
- Future plugin ecosystem

## Workspace Structure

RSTUDIO uses a Rust Cargo workspace.

```text
rstudio/

├── app/
│   Main application entry point
│
├── crates/
│   │
│   ├── common/
│   │   Shared types, errors and utilities
│   │
│   ├── audio-engine/
│   │   Real-time audio engine
│   │
│   └── dsp/
│       Digital signal processing algorithms
│
└── docs/
    Project documentation
````

## Crate Responsibilities

### app

Responsible for:

* Application initialization
* User interface integration
* Connecting all project components

### common

Contains:

* Shared types
* Configuration
* Error handling
* Common utilities

### audio-engine

Responsible for:

* Audio device management
* Audio streams
* Audio buffers
* Real-time processing
* Transport system

The crate separates the hardware-independent `AudioEngine` from its CPAL stream adapter. The engine owns DSP processing state; the adapter owns device discovery, stream construction, sample format conversion, and the CPAL callback.

Application-to-audio commands and audio-to-application events use bounded lock-free queues. The callback only drains a bounded number of commands, renders samples, converts them to the device format, and attempts non-blocking event publication.

`AudioEngine` also owns a hardware-independent `Transport`. It tracks the sample position,
play/pause state, BPM, and the negotiated sample rate. The transport advances by the rendered
frame count only while playing. Audio time is expressed as absolute sample position; musical time
is derived from BPM through explicit beat-to-sample helpers. `Play`, `Pause`, `Seek`, and `SetBpm`
are consumed from the existing command queue, and `TransportUpdate` is published best-effort from
the callback. The transport does not yet alter graph processing or schedule clips.

`AudioEngine` delegates rendering to an `AudioGraph`. The initial graph is a fixed linear chain:
`OscillatorNode -> GainNode -> MixerNode -> MasterBus -> output`. The graph owns preallocated
intermediate buffers for every link before the final output, so execution in the callback neither
allocates nor resizes memory. `MasterBus` applies the final gain and is the future insertion point
for master effects such as a limiter, EQ, or compressor. `SetMasterGain` travels through the
existing bounded command queue and is consumed only by `MasterBus`.

The graph measures the signal after `MasterBus` on every rendered block. `AudioMeter` computes
peak and RMS without temporary buffers, then the CPAL adapter publishes a best-effort
`AudioEvent::MeterUpdate` through the existing bounded lock-free event queue. Events may be
dropped when that queue is full; the callback never blocks to publish metering data.

Its `Vec<Box<dyn AudioNode>>` is a deliberately limited, pre-stream abstraction: topology never
changes in the callback and it is not the final DAW graph model. The node contract receives input
and output bus slices (and declares their counts), so a future planner can introduce routing and
multiple buses without changing the processing interface.

### dsp

Responsible for:

* Signal generation
* Audio processing algorithms
* Effects
* Filters
* Synthesis modules

## Audio Flow

The expected audio pipeline:

```text
Application Commands
     |
     v
Audio Engine API
     |
     v
Transport
     |
     v
CPAL output callback
     |
     v
AudioGraph: oscillator -> gain -> mixer -> master bus -> meter
     |
     v
Audio Output
```

## Design Principles

### Real-time Safety

The audio thread must avoid:

* Dynamic memory allocation
* Blocking operations
* File I/O
* Network operations
* Long locks

### Modularity

Subsystems should remain independent:

* Audio engine
* DSP
* MIDI
* GUI
* Plugins
* Project management

### Extensibility

The architecture should support future features:

* VST3 plugins
* MIDI controllers
* Audio effects
* Automation
* Multiple instruments
