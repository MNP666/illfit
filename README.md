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

## Current status

Version `0.3.0` is complete.

The project already supports:

- parsing common three-column ASCII SAXS files
- fitting smooth `P(r)` distributions with a cubic B-spline basis
- weighted regularized least-squares fitting from `I(q)` data
- derived summaries such as `I(0)` and `Rg`
- local `Dmax` and low-`q` truncation sensitivity scans
- structured CSV/JSON outputs
- Python-based profiling against reference `P(r)` outputs

This is still an early scientific implementation, but it is now a real
end-to-end tool rather than just a scaffold.

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

## Quick start

Build and test the project:

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

Show the current CLI surface:

```bash
cargo run -- --help
```

Run a single fit on one of the example datasets:

```bash
cargo run -- fit \
  --data data/examples/SASDME2.dat \
  --dmax 120.0 \
  --basis-size 16 \
  --integration-intervals 1200 \
  --lambda 0.01 \
  --pr-sample-points 181 \
  --output-dir output/sasdme2_fit
```

Run a local `Dmax` scan:

```bash
cargo run -- scan-dmax \
  --data data/examples/SASDME2.dat \
  --center-dmax 120.0 \
  --half-width 10.0 \
  --point-count 5 \
  --basis-size 16 \
  --integration-intervals 1200 \
  --lambda 0.01 \
  --pr-sample-points 181 \
  --output-dir output/sasdme2_dmax_scan
```

Run a local low-`q` truncation scan:

```bash
cargo run -- scan-truncation \
  --data data/examples/SASDME2.dat \
  --dmax 120.0 \
  --baseline-drop-count 20 \
  --step-size 5 \
  --point-count 5 \
  --basis-size 16 \
  --integration-intervals 1200 \
  --lambda 0.01 \
  --pr-sample-points 181 \
  --output-dir output/sasdme2_truncation_scan
```

Run the profiling comparison workflow:

```bash
python3 profiling/compare_reference_pr.py
```

Edit [`profiling/config.toml`](/Users/air/Documents/illfit/profiling/config.toml)
to choose datasets and sweep parameters.

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
│   ├── PRD_0p2.md
│   ├── ISSUES_0p2.md
│   └── README.md
├── profiling/
│   ├── README.md
│   ├── compare_reference_pr.py
│   └── config.toml
└── src/
    ├── analysis/
    ├── basis/
    ├── data/
    ├── io/
    ├── regularization/
    ├── solver/
    ├── transform/
    ├── cli.rs
    ├── lib.rs
    └── main.rs
```

The exact layout will keep evolving, but the intent is already visible in the
current tree: numerical modeling, data ingestion, analysis, export, and CLI
concerns are separated so the code stays easier to review and extend.

## About `docs/`

The [`docs/`](/Users/air/Documents/illfit/docs/README.md) folder holds
iteration-level planning documents.

- `PRD_0p2.md` describes the goals, scope, and design intent for version `0.2`
- `ISSUES_0p2.md` breaks that iteration into concrete work items

This project uses versioned planning documents instead of one large permanent
specification. That keeps each iteration focused and makes it easier to refine
the design as the implementation matures.

## About `profiling/`

The [`profiling/`](/Users/air/Documents/illfit/profiling/README.md) folder
holds lightweight Python tooling for development-time comparison against
reference `P(r)` outputs.

- `compare_reference_pr.py` runs the Rust CLI on example datasets and compares
  generated `P(r)` curves against reference `.out` files
- `config.toml` controls which datasets and sweep parameters to use

This is intentionally separate from the Rust crate. Python is currently the
fastest way to automate subprocess-based comparisons and plotting while the
scientific core is still evolving.

## About `data/`

The [`data/`](/Users/air/Documents/illfit/data/README.md) folder is for example,
synthetic, and reference datasets used during development and validation.

- `examples/` is for small parser-focused files
- `synthetic/` is for generated test cases with known behavior
- `reference/` is for trusted real datasets and higher-level validation

## What the CLI writes

Single-fit runs write:

- `pr.csv`
- `fit.csv`
- `residuals.csv`
- `report.json`

`scan-dmax` runs also write:

- `dmax_scan.csv`
- `dmax_scan_report.json`

`scan-truncation` runs also write:

- `truncation_scan.csv`
- `truncation_scan_report.json`

## Near-term direction

Version `0.2` is now centered on:

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
- profiling-time comparison against reference `P(r)` outputs

The next iterations can build on this foundation with stronger scientific
validation, richer model families, and more advanced fitting strategies.

Likely `0.3` directions include:

- stronger scientific regression testing
- synthetic validation datasets with known expected behavior
- improved parameter-selection workflows
- richer model families and constraints

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
