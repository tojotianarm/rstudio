# RSTUDIO Coding Style

## General Principles

The codebase must prioritize:

- Readability
- Maintainability
- Correctness
- Performance when necessary

Prefer simple and explicit solutions.

Avoid premature optimization.

---

# Rust Guidelines

## Error Handling

Use explicit error handling with:

- Result<T, E>
- Option<T>
- Proper error propagation

Avoid in production code:

- unwrap()
- expect()
- panic!()

unless there is a documented reason.

---

# Code Organization

Prefer:

- Small modules
- Clear responsibilities
- Minimal coupling
- Reusable components
- Well-defined public APIs

Avoid:

- Large files
- Duplicate logic
- Circular dependencies
- Hidden side effects

---

# Naming Conventions

Follow standard Rust conventions.

## Types

Use PascalCase.

Examples:

```

AudioEngine
AudioBuffer
ProjectConfig

```

## Functions and Variables

Use snake_case.

Examples:

```

process_audio()
sample_rate
buffer_size

```

## Constants

Use SCREAMING_SNAKE_CASE.

Examples:

```

MAX_BUFFER_SIZE
DEFAULT_SAMPLE_RATE

````

---

# Audio Thread Rules

The audio thread is a real-time critical component.

Avoid:

- Memory allocations
- File operations
- Network requests
- Blocking operations
- Heavy computations

Prefer:

- Preallocated buffers
- Deterministic execution
- Predictable processing time
- Lock-free communication when possible

---

# Dependencies

Before adding a dependency:

1. Verify that it solves a real problem.
2. Check its maintenance status.
3. Prefer stable and well-supported libraries.
4. Avoid unnecessary dependencies.

---

# Testing Requirements

New features should include tests.

Before considering a change complete, run:

```bash
cargo fmt
cargo check
cargo clippy
cargo test
````

---

# Git Workflow

Follow these rules:

* Make small commits.
* Use clear commit messages.
* Keep one feature per commit.
* Avoid mixing unrelated changes.
