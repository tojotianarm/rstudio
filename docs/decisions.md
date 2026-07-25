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

---

# ADR-008: Fixed Two-Track Graph

Date: 2026-07-24

## Decision

Use two fixed, preallocated `Track` channels as the first DAW-level graph topology. Route both
track outputs through a two-input mixer and the existing master bus.

## Reason

Tracks establish independent channel state and targeted controls without introducing dynamic
routing, callback-time allocation, maps, or synchronization. A future graph planner can replace
this bounded topology when track creation and routing become application-level operations.

---

# ADR-009: Non-Real-Time Timeline Model

Date: 2026-07-24

## Decision

Keep editable clip and timeline data outside the audio callback. Represent positions and lengths
as integer sample counts, and expose active clips through a non-allocating iterator.

## Reason

Timeline edits can grow or compact a `Vec`, which is forbidden in the callback. This creates a
clear separation between project editing and future playback scheduling, where a preallocated
immutable snapshot will be required for the audio thread.

---

# ADR-010: Fixed-Capacity Audio Snapshots

Date: 2026-07-24

## Decision

Compile editable timeline data into fixed-size, copyable audio snapshots and transfer them through
a bounded lock-free queue. The audio callback consumes only the latest immutable snapshot.

## Reason

Heap-backed snapshots, reference-counted swaps, and mutable timelines can allocate or release
memory in the audio callback. Fixed-capacity values avoid that risk while providing deterministic
clip scheduling. The initial limits are two tracks and sixteen clips per track; future growth must
preserve the same real-time publication guarantees.

---

# ADR-011: Fixed-Capacity Per-Track Voice Pools

Date: 2026-07-24

## Decision

Assign active `ClipPlayback` entries to preallocated `VoiceManager<N>` slots owned by each fixed
track. Each `Voice` owns its oscillator DSP state and is activated or released in place by the
audio callback. The initial graph uses eight slots per track.

## Reason

Overlapping clips need independent source state, but dynamically creating, destroying, or
resizing voices in the callback could allocate or run unpredictable destructors. A fixed pool
makes all pool scans and rendering bounded. The audio callback reads only `AudioSnapshot` data,
uses no locks, and mixes active voices directly into the existing track buffer. Capacity changes,
voice stealing, and sample-accurate intra-block scheduling remain explicit future work rather than
hidden real-time behavior.

---

# ADR-012: Fixed Intra-Block Voice Event Scheduling

Date: 2026-07-24

## Decision

For each rendered track block, convert bounded snapshot clip ranges into a fixed-size, offset
ordered `EventScheduler` list of `StartVoice` and `StopVoice` events. Apply each event immediately
before rendering its target frame.

## Reason

Checking only whether a clip is active at the beginning of a callback makes its audible boundaries
depend on the CPAL block size. A bounded event table keeps scheduling deterministic while producing
sample-accurate starts and stops. It avoids callback-time allocation and does not give the audio
thread access to the editable timeline. Stop events precede start events at equal offsets so slot
reuse has a deterministic order.

---

# ADR-013: Concrete Instrument Voices with ADSR Release

Date: 2026-07-25

## Decision

Define an `Instrument` real-time contract in `audio-engine`, and make each fixed voice own a
concrete `SimpleSynth`. `SimpleSynth` combines the DSP oscillator with an allocation-free ADSR
envelope. `StopVoice` invokes `note_off`; the slot remains allocated until the envelope is idle.

## Reason

Separating a voice's scheduling identity from the instrument's DSP state prepares the engine for
samplers and richer synthesis without coupling clips to an oscillator. Storing the concrete first
instrument directly avoids trait-object allocation and dispatch in the hot path. Delaying slot
reuse through release prevents clicks from abrupt signal truncation while preserving fixed,
deterministic voice-pool bounds.

---

# ADR-014: Fixed Real-Time Parameter Store

Date: 2026-07-25

## Decision

Use a fixed-capacity `ParameterStore` with stable numeric identifiers, descriptors, bounded
targets and sample-count smoothing. `SetParameter` is consumed through the existing audio command
queue; the first integration supplies all `SimpleSynth` controls.

## Reason

The store centralizes mutable real-time control without maps, locks or allocation. It creates one
parameter protocol reusable by automation, MIDI CC, effects and future plugin bridges while
keeping the initial graph bounded and deterministic.
