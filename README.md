# ARCHIVED MARCH 30TH 2026
I did not plan the implementations well enough as vertical slices and hence build (and asked ChatGPT) to build a lot of features that are a nightmare to test and develop properly. Moreover, I moved too fast with the spline setup, and they underpin the whole pipeline.

I think it is possible to remedy it by going back to the start and re-introducing the splines as clamped and then working forward, but it is simply easier to start all over with a new planning stage. That will give me the experience in planning these things out with vertical slices and the appropriate design of interfaces.

The code is still left here, in case I want to re-examing it in connection with the new version: unFourier

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
- deterministic synthetic benchmark suites
- committed regression benchmark assets
- noiseless and noisy benchmark recovery workflows
- structured CSV/JSON outputs
- Python-based profiling and plotting for both reference data and benchmark data

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
│   ├── regression/
│   ├── reference/
│   └── synthetic/
├── docs/
│   ├── DEVELOPMENT.md
│   ├── PRD_0p3.md
│   ├── PRD_0p4.md
│   ├── PRD_0p2.md
│   ├── ISSUES_0p3.md
│   ├── ISSUES_0p4.md
│   ├── ISSUES_0p2.md
│   ├── TECHNICAL_0p4.md
│   └── README.md
├── profiling/
│   ├── README.md
│   ├── benchmark_export.toml
│   ├── compare_reference_pr.py
│   ├── config.toml
│   ├── export_synthetic_benchmarks.py
│   ├── export_noisy_benchmark_signal_scaled.py
│   ├── export_noisy_benchmark_variants.py
│   ├── noisy_benchmark.toml
│   ├── noisy_benchmark_signal_scaled.toml
│   ├── plot_benchmark_case_fits.py
│   ├── plot_benchmark_recovery.py
│   ├── plot_noisy_benchmark_case_fits.py
│   ├── plot_noisy_benchmark_recovery.py
│   └── plot_synthetic_suite.py
└── src/
    ├── analysis/
    ├── benchmark/
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
- `PRD_0p3.md` and `ISSUES_0p3.md` capture the synthetic benchmark and
  regression-testing iteration
- `TECHNICAL_0p4.md`, `PRD_0p4.md`, and `ISSUES_0p4.md` frame the planned
  weighting and lambda-selection work

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
fastest way to automate subprocess-based comparisons, experiment exports, and
plotting while the scientific core is still evolving.

## About `data/`

The [`data/`](/Users/air/Documents/illfit/data/README.md) folder is for example,
synthetic, and reference datasets used during development and validation.

- `examples/` is for small parser-focused files
- `synthetic/` is for generated exploratory benchmark and noisy-suite assets
- `regression/` is for committed benchmark assets used in standing regression
  tests
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

`benchmark-recover` runs write:

- per-case truth, recovery, and comparison files
- `benchmark_suite_summary.csv`
- `benchmark_suite_report.json`

`benchmark-recover-noisy` runs write:

- per-noise-level and per-case recovery bundles
- noisy-case metadata
- `benchmark_suite_summary.csv`
- `benchmark_suite_report.json`

## Near-term direction

Version `0.3` is now centered on:

- a CLI-first workflow
- smooth basis-function representations for `P(r)`
- regularized fitting of SAXS curves
- local `Dmax` scanning to assess solution stability
- local low-`q` truncation scanning to assess preprocessing sensitivity
- deterministic synthetic benchmark generation and recovery
- noisy observed benchmark analysis
- lightweight Python profiling against both reference and benchmark outputs

## Development notes

The `0.3` scientific core is now implemented end to end:

- SAXS data parsing and validation
- cubic B-spline `P(r)` basis representation
- forward transform to predicted `I(q)`
- weighted regularized least-squares fitting
- derived summaries such as `I(0)` and `Rg`
- structured output writing
- CLI commands for single fits, `Dmax` scans, and truncation scans
- benchmark asset loading, recovery, and comparison
- regression testing against committed benchmark suites
- profiling-time comparison and plotting for both reference and benchmark data

The next iterations can build on this foundation with stronger scientific
validation, richer model families, and more advanced fitting strategies.

Likely `0.4` directions include:

- weighting or scaling strategy experiments
- lambda scans and automatic lambda selection
- L-curve and GCV analysis
- TOML-driven experiment workflows and profiling output bundles

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
