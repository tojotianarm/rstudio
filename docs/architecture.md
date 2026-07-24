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
Audio Input
     |
     v
Audio Callback
     |
     v
Audio Engine
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
