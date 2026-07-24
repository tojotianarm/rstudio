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

`AudioEngine` now delegates rendering to an `AudioGraph`. The first graph is linear and owns a
single oscillator node. Its `Vec<Box<dyn AudioNode>>` is a deliberately limited, pre-stream
abstraction: it never changes in the callback and is not the final topology model. The node
contract receives input and output bus slices (and declares their counts), so a future graph
planner can introduce multiple buses without changing the processing interface.

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
CPAL Output Callback
     |
     v
DSP Processing
     |
     v
Mixer
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
