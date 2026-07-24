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

---

# ADR-006: Master Bus and Best-Effort Metering

Date: 2026-07-24

## Decision

Terminate the initial graph with a `MasterBus`, and measure peak/RMS after that bus for each
rendered block. Publish the measurements through the existing bounded lock-free event queue.

## Reason

The master bus creates a dedicated location for final gain and future master effects. Measuring
after it reports the signal actually sent to the output. Meter events are intentionally best-effort:
the callback drops an event when the queue is full rather than allocating or blocking.

---

# ADR-007: Audio-Thread-Owned Transport

Date: 2026-07-24

## Decision

Keep transport state inside `AudioEngine` and advance it from completed render blocks only while
playing. Exchange transport commands and snapshots through the existing bounded lock-free queues.

## Reason

The audio thread is the authoritative source for sample position. Keeping the state there avoids
locks and clock drift between the application and hardware callback while preserving a future path
to timeline, MIDI, clip, and automation scheduling.
