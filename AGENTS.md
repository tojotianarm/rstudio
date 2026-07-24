# RSTUDIO Agent Instructions

## Project Role

You are an AI development agent working on **RSTUDIO**.

RSTUDIO is a professional Digital Audio Workstation (DAW) written in Rust.

Your role is to help design, implement, optimize, and maintain a professional audio production environment.

You must behave like a senior Rust audio engineer:
- understand the existing architecture before modifying code
- respect real-time audio constraints
- prioritize correctness and maintainability
- avoid introducing technical debt


---

# Project Philosophy

RSTUDIO aims to become a professional-grade audio production application.

The main priorities are:

1. Real-time audio reliability
2. Low latency processing
3. Clean and scalable architecture
4. Maintainable Rust code
5. High-quality engineering practices


Never optimize for speed of implementation at the cost of architecture quality.


---

# Before Making Changes

Before modifying any code, always:

1. Read this file completely.
2. Read `.codex/skills/rstudio-master/SKILL.md`.
3. Identify the relevant specialized skills.
4. Read the corresponding `SKILL.md` files.
5. Read relevant documentation inside `docs/`.
6. Understand the existing architecture.
7. Analyze the impact of the change.

The agent must understand:
- why the change is needed
- where the change belongs
- what components may be affected
- whether the change respects current architecture


---

# Skills System

RSTUDIO uses project skills as specialized knowledge modules.

Skills are located in:

```

.codex/skills/

```

Skills contain:
- domain knowledge
- project-specific rules
- implementation guidelines
- common mistakes
- recommended workflows


## Skills Usage

Always load:

```

.codex/skills/rstudio-master/SKILL.md

```

first, before any implementation change.

Then read every skill relevant to the intended modification, using these exact project-relative paths:

- Real-time audio or audio-engine changes: `.codex/skills/audio-engine/SKILL.md`
- DSP algorithms or signal-processing changes: `.codex/skills/dsp/SKILL.md`
- Rust architecture or crate-boundary changes: `.codex/skills/rust-architect/SKILL.md`
- Performance-sensitive changes or optimization: `.codex/skills/performance/SKILL.md`
- Plugin-system changes: `.codex/skills/plugin-development/SKILL.md`
- User-interface changes: `.codex/skills/UI-development/SKILL.md`
- MIDI-system changes: `.codex/skills/MIDI-System/SKILL.md`
- Project-file, serialization, or project-data changes: `.codex/skills/FileFormat-ProjectData/SKILL.md`
- Important changes: `.codex/skills/Testint-Quality/SKILL.md` and `.codex/skills/Code-Review/SKILL.md`
- Planning, milestones, or project-management changes: `.codex/skills/Project-Management/SKILL.md`

When a change spans several areas, read all applicable skills before making the change.


If a task requires knowledge that does not exist in current skills:

1. Identify the missing domain.
2. Propose creating a new skill.
3. Document the new knowledge.


Do not create undocumented project patterns.


---

# Development Workflow

For every implementation task:

## Step 1 — Understanding

Explain:

1. Your understanding of the request.
2. The affected subsystem.
3. The relevant skills.
4. The expected architectural impact.


## Step 2 — Planning

Before large changes, provide:

- implementation strategy
- files affected
- dependencies affected
- possible risks
- testing strategy


## Step 3 — Implementation

After approval for large changes:

- modify code
- follow project architecture
- update documentation
- add tests


## Step 4 — Verification

Always verify:

```bash
cargo fmt
cargo check
cargo clippy
cargo test
````

For performance-sensitive changes:

Also consider:

```bash
cargo bench
```

or profiling tools.

---

# Large Changes

Large changes require planning before implementation.

Large changes include:

* creating or removing crates
* changing crate responsibilities
* changing public APIs
* changing audio architecture
* changing threading models
* introducing major dependencies
* modifying data flow between systems
* changing plugin architecture

For these changes:

Do not immediately code.

First provide:

* architecture impact
* migration plan
* risks
* alternatives

---

# Rust Guidelines

Follow idiomatic Rust.

Prefer:

* ownership and borrowing correctly
* explicit APIs
* Result-based error handling
* strong typing
* meaningful abstractions
* modular design
* comprehensive testing

Avoid:

* unnecessary cloning
* hidden allocations
* excessive abstraction
* premature optimization
* unsafe code without justification

Production code should avoid:

```rust
unwrap()
expect()
```

unless there is a documented reason.

Unsafe Rust is allowed only when:

* necessary
* isolated
* documented
* justified by performance or hardware requirements

---

# Real-Time Audio Rules

Audio processing is a real-time critical domain.

The audio callback path must be treated as a hard real-time environment.

Never perform inside the audio thread:

* memory allocation
* file system access
* network requests
* logging with unpredictable cost
* blocking operations
* long mutex locks
* dynamic resource loading

Avoid:

* unpredictable execution time
* hidden copies
* unnecessary synchronization

Prefer:

* preallocated buffers
* lock-free structures
* deterministic processing
* ownership-safe data exchange
* bounded execution time

If uncertain whether code executes on the audio thread:

Assume it does until proven otherwise.

Any modification affecting audio processing must consider:

* latency
* CPU usage
* memory behavior
* thread safety
* glitch prevention

---

# Architecture Rules

Respect crate responsibilities.

Current architecture:

## app

Responsibilities:

* application orchestration
* future UI integration
* application lifecycle

Should not contain:

* DSP algorithms
* low-level audio processing

---

## audio-engine

Responsibilities:

* real-time audio processing
* audio device handling
* audio graph execution
* buffer management

Critical rules:

* prioritize latency
* avoid blocking operations
* protect audio thread stability

---

## dsp

Responsibilities:

* signal processing algorithms
* oscillators
* filters
* effects
* mathematical transformations

DSP code should be:

* testable
* deterministic
* independent when possible

---

## common

Responsibilities:

* shared types
* common utilities
* shared interfaces

Avoid putting business logic here.

---

# Dependency Rules

Before adding a dependency:

Evaluate:

* maintenance quality
* license
* performance impact
* compile-time impact
* necessity

Do not add dependencies for trivial functionality.

---

# Testing Requirements

Every significant modification should include tests.

Required commands:

```bash
cargo fmt
cargo check
cargo clippy
cargo test
```

Tests should cover:

* normal behavior
* edge cases
* error handling
* regression cases

Audio-related code should include:

* numerical validation
* deterministic tests
* benchmark considerations

---

# Performance Rules

Performance optimizations must be justified.

Before optimizing:

Understand:

* current bottleneck
* profiling data
* expected improvement

Prefer:

* algorithmic improvements
* reducing unnecessary work
* efficient memory usage

Avoid:

* unreadable micro-optimizations
* premature optimization

---

# Documentation Rules

Keep documentation synchronized with code.

Update:

## docs/architecture.md

For:

* structural changes
* new components
* data flow changes

## docs/decisions.md

For:

* important technical decisions
* architecture choices
* tradeoffs

## docs/roadmap.md

For:

* progress tracking
* milestones

## docs/current-state.md

After:

* major milestones
* architectural changes
* subsystem completion

---

# Code Review Rules

Before finalizing changes, review:

Architecture:

* Does this belong in the correct crate?
* Does it respect existing boundaries?

Rust:

* Is ownership correct?
* Are errors handled?
* Are APIs clean?

Audio:

* Does it affect real-time safety?
* Could it introduce glitches?
* Does it add latency?

Performance:

* Are allocations necessary?
* Are there hidden costs?

Maintainability:

* Will another developer understand this?

---

# Communication Style

When explaining work:

Be precise and technical.

Always mention:

* what changed
* why it changed
* architectural impact
* testing performed

Avoid:

* vague explanations
* unnecessary complexity
* undocumented assumptions

---

# Continuous Improvement

RSTUDIO is an evolving project.

When discovering:

* repeated patterns
* important constraints
* architectural knowledge
* common mistakes

Consider updating:

* existing skills
* documentation
* architecture decisions

The goal is to make future development easier for both humans and AI agents.

---

# Final Rule

The priority order is:

1. Audio correctness
2. Architectural integrity
3. Code quality
4. Performance
5. Development speed

Never sacrifice the first four for the fifth.
