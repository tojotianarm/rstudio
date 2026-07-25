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

`Timeline` is a separate non-real-time project model. It stores sample-accurate `AudioClip`
metadata associated with `TrackId`; editing it may allocate or move its `Vec` and therefore never
happens in the callback. `SnapshotCompiler` converts it into an immutable fixed-capacity
`AudioSnapshot`: each `ClipPlayback` contains precomputed start and end sample positions.

The application publishes snapshots through a bounded lock-free queue. The callback consumes only
fixed-size snapshot values and retains its current one when no update is available. `ClipScheduler`
then determines active clips at the current transport position. This first version bounds playback
to two tracks and sixteen clips per track, avoiding allocation, reference-count destruction, or
dynamic timeline traversal in the audio thread.

`AudioEngine` delegates rendering to an `AudioGraph` with two fixed `Track` channels. Each track
owns channel state (gain, mute, solo flag) and a fixed `VoiceManager<8>`. A voice owns a concrete
`SimpleSynth` through the `Instrument` contract, and is assigned to one active `ClipPlayback`;
overlapping clips therefore render through distinct voice slots. `SimpleSynth` encapsulates an
oscillator and a DSP-owned ADSR envelope. The scheduler is queried only against the immutable
snapshot, then the manager reconciles and mixes its bounded voice pool into the track's
preallocated output buffer. Inactive slots produce silence. The graph passes both track buffers to
`MixerNode(2)`, then routes the mix through `MasterBus`.
`TrackId` is a stable numeric key for targeted queue commands; track names are configuration data
and are never read in the callback. `SetTrackGain`, `SetTrackMute`, and `SetTrackSolo` scan only
the fixed track array, without maps, locks, or allocation.

Voice capacity is selected when each track is built, rather than resized while rendering. At this
stage, excess simultaneous clips are left unassigned once the pool is full; voice stealing and
runtime polyphony reconfiguration require a future non-real-time graph/snapshot rebuild.

For every track block, `EventScheduler` scans only its bounded immutable snapshot data and writes
ordered `StartVoice` and `StopVoice` entries to fixed storage. Each entry has a frame offset in
the current callback block; stops sort before starts at the same offset. `VoiceManager` applies
those events immediately before rendering the matching frame, which gives sample-accurate clip
boundaries without traversing the mutable timeline, allocating, or locking in the callback.

`StartVoice` calls `Instrument::note_on`, beginning the ADSR attack phase. `StopVoice` calls
`Instrument::note_off`; it does not immediately return the slot to the pool. The voice remains
owned by the audio thread through `Release` and is released only after the envelope reaches
`Idle`, avoiding an audible discontinuity. The initial concrete instrument is deliberately stored
without dynamic dispatch in the voice pool; the trait defines the extension boundary for future
samplers and synths without introducing callback-time allocation.

`ParameterStore<9>` is the bounded parameter engine for the initial graph. It stores stable
descriptors, bounded current and target values, and deterministic smoothing state. Application
commands update targets through `SetParameter`; `SimpleSynth` reads frequency, gain and ADSR
parameters from this store while rendering. The same fixed-store boundary prepares automation,
MIDI controllers, effects and plugin parameters without locks or callback-time allocation.

Each track processes its `EffectRack<4>` after its instrument voices and before the mixer. A second
`EffectRack<4>` processes the mixed signal before `MasterBus`. Racks store concrete fixed slots,
run effects in insertion order, and process buffers in place; `AudioEffect` is the extension point
for future EQ, delay, dynamics and plugin adapters.

Audio-file playback is prepared outside the real-time path. `WavLoader` decodes WAV data into a
`PcmAudioBuffer`, while `SampleRegistryBuilder` places that PCM data into a fixed `SampleRegistry`.
The registry is attached to `AudioEngine` before the engine is moved into the CPAL callback and is
then read-only. `AudioClip` and `ClipPlayback` can identify `ClipSource::AudioFile(SampleId)`;
the snapshot therefore carries only a copyable sample identifier, never a file handle or mutable
project object. `SamplePlayer` already provides allocation-free frame reading with linear sample
rate conversion. `AudioGraph` owns the immutable registry before stream construction; each fixed
voice selects either `SimpleSynth` or a PCM cursor from `ClipSource`. A sample voice resolves its
`SampleId` only in that registry and advances its cursor in place, so no file access, allocation,
lock, or PCM copy occurs in the callback. End-of-buffer recycles the fixed voice slot.

`MasterBus` applies final gain and is the future insertion point for limiter, EQ, or compression.
All track, mix, and master buffers are allocated before the stream starts, so callback execution
neither allocates nor resizes memory.

The graph measures the signal after `MasterBus` on every rendered block. `AudioMeter` computes
peak and RMS without temporary buffers, then the CPAL adapter publishes a best-effort
`AudioEvent::MeterUpdate` through the existing bounded lock-free event queue. Events may be
dropped when that queue is full; the callback never blocks to publish metering data.

The topology is deliberately fixed in this version; a future graph planner can extend routing and
track counts only through preallocated playback data and a safe snapshot publication protocol.

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
Timeline -> SnapshotCompiler -> immutable AudioSnapshot
     |
     v
CPAL output callback
     |
     v
AudioGraph: ClipScheduler -> EventScheduler -> VoiceManager -> Instrument/ADSR -> mixer -> master bus -> meter
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
