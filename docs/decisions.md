# RSTUDIO Architecture Decisions

This document records important technical decisions made during the development of RSTUDIO.

The goal is to preserve the reasoning behind architectural choices.

---

# ADR-001: Programming Language

Date: 2026-07-24

## Decision

Use Rust as the main programming language.

## Reason

Rust provides:

- High performance
- Memory safety
- Low-level control
- Good support for real-time applications

Rust is suitable for:

- Audio engines
- DSP processing
- System programming


---

# ADR-002: Project Structure

Date: 2026-07-24

## Decision

Use a Cargo workspace with multiple crates.

## Structure

```

app
|
+-- audio-engine
|
+-- dsp
|
+-- common

```

## Reason

A modular architecture allows:

- Independent development
- Easier testing
- Better separation of responsibilities
- Future expansion


---

# ADR-003: Real-Time Audio Design

Date: 2026-07-24

## Decision

The audio processing thread must remain real-time safe.

## Rules

The audio thread should avoid:

- Allocations
- Blocking operations
- File access
- Network operations

## Reason

Audio requires predictable execution to prevent glitches and latency issues.


---

# ADR-004: Documentation Driven Development

Date: 2026-07-24

## Decision

Maintain project documentation for AI agents and developers.

## Reason

The project will use AI coding agents.

Documentation provides:

- Architectural context
- Project memory
- Consistent decisions
- Faster development

---

# ADR-005: Preallocated Linear Audio Graph

Date: 2026-07-24

## Decision

Use a fixed `OscillatorNode -> GainNode -> MixerNode -> output` chain for the first composable
audio graph. Allocate its intermediate buffers during graph preparation, before the CPAL stream
starts.

## Reason

This provides real node-to-node routing without allocations, locks, or topology changes in the
audio callback. `AudioNode` already accepts input and output bus slices, preserving a migration
path to a future routed multi-bus graph without adding a graph planner prematurely.
