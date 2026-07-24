# Copilot Instructions

## Project

RSTUDIO is a Rust DAW project.

## General Rules

- Always read AGENTS.md before modifying code.
- Prefer small incremental changes.
- Explain the plan before implementing large features.
- Do not modify architecture without approval.
- Run cargo fmt and cargo check after changes.

## Rust Rules

- Use idiomatic Rust.
- Avoid unwrap() in production code.
- Avoid unnecessary allocations.
- Keep audio thread code real-time safe.

## Workflow

Before coding:
1. Analyze existing code.
2. Explain proposed changes.
3. Wait for confirmation for architectural changes.