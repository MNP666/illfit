# Development

This document captures the initial developer workflow and coding conventions for
`illfit`.

The project is intentionally small right now, but it is easier to keep a Rust
codebase healthy if expectations are explicit from the beginning.

## Core commands

Use these commands during normal development:

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
cargo run
```

What they are for:

- `cargo fmt`: apply standard Rust formatting
- `cargo clippy --all-targets --all-features`: lint library, binary, and tests
- `cargo test`: run unit and integration tests
- `cargo run`: run the current CLI entry point

## Current test scope

At the moment, tests focus on the data layer:

- parsing supported ASCII SAXS formats
- rejecting invalid curves
- validating real example files in `data/examples/`

As more numerical modules are added, test coverage should expand with them.

## Conventions

### Rust design

- prefer explicit domain types over raw tuples or loosely structured vectors
- keep module responsibilities narrow
- introduce traits only when they clearly help extension or testing
- document non-obvious numerical logic and important design tradeoffs

### Safety and style

- keep the code `unsafe`-free unless there is a strong reason otherwise
- prefer readable, idiomatic code over premature abstraction
- write comments to explain why a piece of code exists or what mathematical role
  it plays, not to restate obvious syntax

### Data and validation

- parsers may be permissive about text formatting
- validated domain types should be strict about scientific correctness
- example and synthetic datasets should stay small and easy to inspect

## Review checklist

Before considering a change complete, the default expectation is:

1. `cargo fmt` passes
2. `cargo clippy --all-targets --all-features` passes
3. `cargo test` passes
4. code comments explain non-obvious logic
5. docs are updated if user-facing structure or workflow changed
