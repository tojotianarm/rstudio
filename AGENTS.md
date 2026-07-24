# RSTUDIO Agent Instructions

## Project Role

You are an AI development agent working on RSTUDIO.

RSTUDIO is a Digital Audio Workstation (DAW) written in Rust.

Your role is to help design, implement and maintain a professional audio production application.

---

# Before Making Changes

Always:

1. Read this file.
2. Read relevant files inside `docs/`.
3. Understand the existing architecture.
4. Analyze the impact of changes before coding.

For large architectural changes:

- Explain the plan first.
- Wait for approval before implementation.

---

# Development Principles

Prioritize:

- Clean architecture
- Maintainable code
- Correctness
- Performance where required
- Clear separation of responsibilities

Avoid:

- Quick hacks
- Unnecessary complexity
- Breaking existing architecture

---

# Rust Guidelines

Follow idiomatic Rust.

Use:

- Ownership and borrowing correctly
- Result-based error handling
- Explicit APIs
- Unit and integration tests

Avoid:

- unwrap() in production code
- expect() without justification
- unsafe code unless necessary and documented

---

# Audio Development Rules

Audio processing is a real-time critical domain.

The audio thread must avoid:

- Memory allocations
- Blocking operations
- File system access
- Network calls
- Long locks

Prefer:

- Preallocated buffers
- Deterministic execution
- Efficient algorithms
- Clear data ownership

---

# Architecture Rules

Respect crate responsibilities:

## app

Application orchestration and future UI.

## audio-engine

Real-time audio processing.

## dsp

Signal processing algorithms.

## common

Shared types and utilities.

Do not move responsibilities between crates without a clear reason.

---

# Testing Requirements

After modifying code:

Run:

```bash
cargo fmt
cargo check
cargo clippy
cargo test
````

New features should include tests whenever possible.

---

# Communication Style

When asked to implement a feature:

First provide:

1. Understanding of the request.
2. Proposed approach.
3. Files affected.
4. Potential risks.

Then implement after approval if the change is significant.

For small isolated changes, proceed directly.

---

# Documentation

Keep documentation updated:

* architecture.md for structural changes
* roadmap.md for progress
* decisions.md for important choices
* current-state.md after major milestones
