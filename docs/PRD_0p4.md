# PRD 0.4

## Working Title

Regularization and weighting study framework for SAXS iFT.

## Purpose

Version 0.4 should build on the synthetic and noisy benchmark machinery from
`0.3` by adding a full experiment pipeline for testing how weighting choices
affect recovery quality and regularization selection.

This iteration should prioritize four things:

1. A configurable weighting framework for the inverse problem
2. A robust lambda-scan workflow with clear reporting
3. First automatic lambda-selection methods, specifically L-curve and GCV
4. A TOML-driven CLI pipeline that writes complete profiling outputs to a fresh
   folder under `profiling/output/`

## Product Vision

The tool should now help answer a more advanced scientific question:

"Given a family of weighting choices and a family of lambda values, which
combinations behave best on noiseless and noisy SAXS benchmarks?"

Version 0.4 is not mainly about adding another single fit command. It is about
turning the project into a reproducible experiment platform for optimization
strategy decisions.

When `0.4` is complete, a user should be able to define an experiment in one
TOML file, run one CLI command, and receive a full output bundle containing:

- fitted outputs
- lambda scan summaries
- selected lambda values
- L-curve data
- GCV data
- machine-readable summaries suitable for plotting and review

## Background

Version `0.2` established the base iFT pipeline:

- spline-based `P(r)` representation
- forward transform
- weighted regularized least squares
- `Dmax` and truncation scanning

Version `0.3` added:

- deterministic synthetic benchmark generation
- committed regression assets
- truth-vs-recovery metrics in `r` and `q`
- noisy observed benchmark variants
- benchmark CLI and profiling workflows

That means the project now has the infrastructure needed to test optimization
choices in a principled way.

The current solver still uses:

- raw `I(q)` as the fitting target
- weighting by `1 / sigma`
- a second-difference coefficient penalty
- a user-chosen `lambda`

This is a good baseline, but it leaves two major questions open:

- how should the data be weighted or rescaled during fitting?
- how should `lambda` be selected?

## Primary Goals

- add a configurable weighting or scaling framework for benchmarked fits
- compare weighting strategies on both noiseless and noisy benchmark suites
- add structured lambda scans for those weighting strategies
- implement L-curve and GCV as the first automatic lambda-selection methods
- expose experiment configuration through a TOML-driven CLI workflow
- write full experiment results to a fresh profiling output folder
- preserve clarity in both the Rust implementation and the experiment outputs

## Non-Goals

- positivity-constrained optimization in version 0.4
- replacing the baseline solver entirely before comparison data is gathered
- structure-factor modeling in version 0.4
- broad solver zoo or dashboard work
- GUI work
- migrating synthetic benchmark generation from Python into Rust

Clarification:

- it is acceptable if version `0.4` includes one optional alternative solve
  formulation later in the iteration
- however, weighting and lambda selection are the core scope, not general
  solver redesign

## Users

### Primary user

A scientific developer who wants to test optimization choices systematically
while continuing to use the project as a Rust-learning vehicle.

### Secondary users

- SAXS method developers evaluating regularization strategies
- future contributors adding constraints, new penalties, or new solvers

## Product Principles

### 1. Baselines must stay visible

Any new weighting or lambda-selection method should be comparable against the
current baseline, not replace it silently.

### 2. Experiment definitions should be reproducible

The main `0.4` workflow should be driven by TOML configuration rather than
ad hoc command lines or notebook state.

### 3. Benchmarks should drive decisions

New optimization methods should be judged on the synthetic and noisy benchmark
suites, not mainly by subjective inspection of one or two real curves.

### 4. Reports should support review

The output bundle should be useful for both machine-readable comparison and
human review.

### 5. Code should remain teachable

The implementation should favor explicitness over abstraction-heavy design.

## Scientific Scope

## Current baseline

Version 0.4 assumes the current baseline remains available:

- fit raw `I(q)`
- weight by `1 / sigma`
- regularize with a second-difference penalty on coefficients
- choose `lambda` manually

All new methods should be comparable to this baseline.

### Weighting or scaling strategies

Version 0.4 should introduce configurable weighting or scaling strategies for
the fit objective.

Initial candidate strategies:

- `none`
- `q`
- `q^2`
- general `q^alpha`

There are two conceptually different options:

1. transformed target fits
   - fit `q^alpha I(q)` and transform uncertainties consistently

2. residual reweighting
   - keep raw `I(q)` but apply `q`-dependent weighting to the residuals

Version 0.4 does not need to settle the scientific question in advance. It
should make these alternatives testable in a controlled way.

### Lambda scans

Version 0.4 should add a first-class lambda-scan workflow.

The experiment runner should be able to:

- define a lambda grid
- run fits across that grid
- preserve fit metrics across lambda
- compare behavior under different weighting strategies

This scan should work on:

- noiseless benchmark suites
- noisy observed benchmark suites

### Automatic lambda selection

Version 0.4 should implement two first automatic lambda-selection methods:

#### 1. L-curve

Use the tradeoff curve between:

- weighted data misfit
- smoothness penalty

and report the chosen corner or selected region.

#### 2. GCV

Use generalized cross-validation as an automated criterion for choosing
`lambda`.

This will likely require careful bookkeeping around the effective influence of
the regularized solve, but it is in scope for `0.4`.

### Outputs and readout

For each experiment run, the output bundle should contain:

- configuration snapshot
- per-strategy summary
- per-lambda summary table
- fitted `P(r)` and `I(q)` outputs for selected lambda values
- L-curve data
- GCV data
- selected lambda by method
- benchmark-level comparison summaries

Plots may be generated either directly or through plot-ready CSV/JSON outputs.

### Profiling output location

The main experiment runner should write to a new folder under:

- [`profiling/output/`](/Users/air/Documents/illfit/profiling/output)

The folder should be fresh per run, either by:

- timestamp
- experiment name
- or both

The goal is to avoid overwriting prior experiment results by default.

## Functional Requirements

### TOML-driven experiment workflow

Version 0.4 must provide a workflow where one TOML file defines:

- input benchmark suite
- weighting strategies
- lambda grid
- lambda-selection methods
- recovery configuration
- output naming

The user should then run one CLI command against that file.

Likely command shape:

- `profile-regularization`
- or a similarly explicit subcommand name

### Benchmark experiment execution

The runner must support:

- running the same benchmark suite across multiple weighting strategies
- running lambda scans for each strategy
- computing suite-level comparison metrics
- recording automatic lambda choices from L-curve and GCV

### Output bundle

Each run must produce a self-contained output folder with:

- config snapshot
- summary tables
- selector outputs
- selected-fit outputs
- plot-ready artifacts

## Non-Functional Requirements

- deterministic behavior for the same config and input assets
- explicit and documented weighting definitions
- explicit and documented lambda-selection outputs
- idiomatic Rust module boundaries
- comments around mathematically non-obvious steps
- strong test coverage for weighting logic and selector calculations

## Proposed User Experience

Version 0.4 should remain CLI-first.

The main new workflow should look like:

```bash
cargo run -- profile-regularization --config profiling/regularization_0p4.toml
```

The user should then inspect the generated folder under
[`profiling/output/`](/Users/air/Documents/illfit/profiling/output).

## Suggested Technical Structure

Likely new modules:

- `src/weighting`
  - weighting and scaling definitions
- `src/lambda`
  - lambda grids, L-curve, GCV
- `src/experiment`
  - TOML config parsing and experiment orchestration
- `src/benchmark`
  - integration with existing noiseless and noisy benchmark paths
- `src/io`
  - experiment output writers

Profiling-side support likely includes:

- plotting helpers for lambda scans
- plotting helpers for L-curves
- plotting helpers for GCV traces

## Success Criteria

Version 0.4 is successful if:

- weighting strategies can be compared reproducibly
- lambda scans work on benchmark suites end to end
- L-curve and GCV both produce usable outputs
- one TOML-driven command generates a full profiling readout
- the results are useful enough to guide future solver and regularization work

## Open Questions

1. Should `0.4` prioritize transformed targets, residual reweighting, or both?

2. Should `q^alpha` be fully generic in `0.4`, or should we start with a small
set like:

- `none`
- `q`
- `q^2`

3. Should selected lambda values be reported:

- per case
- per suite
- or both?

4. Should `0.4` include only the existing normal-equation solver, or leave room
for one comparison solve formulation later in the iteration?

## Recommendation

My recommendation is:

- include both transformed-target and residual-weighting machinery only if the
  implementation remains clear
- otherwise start with one weighting abstraction that is general enough to
  support both later
- keep L-curve and GCV as the core new scientific features
- center the whole iteration around the TOML-driven experiment runner
