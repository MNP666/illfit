# illfit

`illfit` is a Rust implementation of indirect Fourier transformation (iFT) for
small-angle X-ray scattering (SAXS) data.

The project starts simple on purpose. The initial goal is to build a clear,
scientifically transparent foundation for fitting smooth `P(r)` distributions
from SAXS data, while leaving room for more advanced approaches over time.

A central project goal is also to learn Rust well while building something
useful. That means the code should favor idiomatic, principled design, clear
module boundaries, and comments around mathematical or language-level details
that are not immediately obvious during review.

## Project goals

- Build a clean Rust implementation of iFT for SAXS
- Start with a simple, well-scoped solver path before adding more advanced
  approaches
- Treat smooth `P(r)` representations as a first-class modeling choice
- Support reproducible analysis and explicit diagnostics
- Keep the architecture extensible so new basis functions, regularizers, and
  analysis workflows can be added later

## Non-goals

- Compatibility with GNOM or ATSAS
- Reproducing legacy SAXS tool behavior for its own sake
- Hiding scientific choices behind a fully automatic black box

## Philosophy

This repository is both a scientific software project and a Rust learning
project.

That has a few practical consequences:

- numerical choices should be documented
- abstractions should be introduced only when they improve clarity
- code should be organized so it is easy to review and extend
- comments should help explain the "why" behind important implementation
  decisions

## Project layout

Current repository structure:

```text
.
├── Cargo.toml
├── LICENSE
├── README.md
├── data/
│   ├── README.md
│   ├── examples/
│   ├── reference/
│   └── synthetic/
├── docs/
│   ├── DEVELOPMENT.md
│   ├── README.md
│   ├── PRD_0p2.md
│   └── ISSUES_0p2.md
└── src/
    ├── data/
    ├── lib.rs
    └── main.rs
```

Expected layout as the implementation grows:

```text
.
├── data/
│   ├── examples/
│   ├── reference/
│   └── synthetic/
├── docs/
│   ├── PRD_0p2.md
│   ├── ISSUES_0p2.md
│   ├── PRD_0p3.md
│   └── ISSUES_0p3.md
└── src/
    ├── analysis/
    ├── basis/
    ├── cli/
    ├── data/
    ├── io/
    ├── regularization/
    ├── solver/
    └── transform/
```

The exact source layout may evolve, but the intent is to keep numerical,
domain, and interface concerns separated as the codebase grows.

## About `docs/`

The [`docs/`](/Users/air/Documents/illfit/docs/README.md) folder holds
iteration-level planning documents.

- `PRD_0p2.md` describes the goals, scope, and design intent for version `0.2`
- `ISSUES_0p2.md` breaks that iteration into concrete work items

This project uses versioned planning documents instead of one large permanent
specification. That keeps each iteration focused and makes it easier to refine
the design as the implementation matures.

## About `data/`

The [`data/`](/Users/air/Documents/illfit/data/README.md) folder is for example,
synthetic, and reference datasets used during development and validation.

- `examples/` is for small parser-focused files
- `synthetic/` is for generated test cases with known behavior
- `reference/` is for trusted real datasets and higher-level validation

## Near-term direction

Version `0.2` is centered on:

- a CLI-first workflow
- smooth basis-function representations for `P(r)`
- regularized fitting of SAXS curves
- local `Dmax` scanning to assess solution stability
- local low-`q` truncation scanning to assess preprocessing sensitivity
- lightweight Python profiling against reference `P(r)` outputs

## Development notes

The `0.2` scientific core is now implemented end to end:

- SAXS data parsing and validation
- cubic B-spline `P(r)` basis representation
- forward transform to predicted `I(q)`
- weighted regularized least-squares fitting
- derived summaries such as `I(0)` and `Rg`
- structured output writing
- CLI commands for single fits, `Dmax` scans, and truncation scans

The next iterations can build on this foundation with stronger scientific
validation, richer model families, and more advanced fitting strategies.

## Developer workflow

The main development commands are:

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
cargo run
```

The more detailed workflow and project conventions live in
[`docs/DEVELOPMENT.md`](/Users/air/Documents/illfit/docs/DEVELOPMENT.md).
