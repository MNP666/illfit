# PRD 0.2

## Working Title

Rust tool for indirect Fourier transformation of SAXS data with smooth `P(r)`
representations and explicit `Dmax` stability analysis.

## Purpose

Version 0.2 defines the first serious planning target for the project. The aim
is to build a scientifically transparent and idiomatic Rust command-line tool
for indirect Fourier transformation (iFT) of SAXS data.

This iteration should prioritize three things:

1. A mathematically clear iFT pipeline based on smooth basis functions
2. Product support for local `Dmax` stability analysis
3. A codebase that is easy to learn from and review as a Rust project

## Product Vision

The tool should help a SAXS user recover a smooth, interpretable `P(r)` from
measured scattering data without inheriting design constraints from GNOM or
ATSAS. Compatibility with those tools is explicitly out of scope.

The tool should also help the user reason about the effect of subjective
`Dmax` choice by making local sensitivity analysis a core workflow rather than a
late add-on.

## Background

Indirect Fourier transformation for SAXS is an inverse problem: infer a
real-space pair-distance distribution `P(r)` from reciprocal-space intensity
data `I(q)`. In practice, this requires:

- a representation for `P(r)`
- a forward transform from `P(r)` to predicted `I(q)`
- a fitting objective, ideally weighted by experimental uncertainty
- regularization to stabilize the inversion
- diagnostics to understand fit quality and physical plausibility

For this project, we assume that physically meaningful `P(r)` curves are smooth.
That assumption should be reflected directly in the representation and the API.

## Primary Goals

- Fit smooth `P(r)` functions from 1D SAXS data using a basis-function approach
- Make the scientific choices explicit and inspectable
- Support local scanning around a chosen `Dmax` to estimate solution stability
- Produce reproducible, machine-readable outputs
- Use idiomatic, principled Rust with comments that support learning and review

## Non-Goals

- GNOM compatibility
- ATSAS compatibility
- Reproducing legacy file formats or legacy solver behavior
- SAXS detector data reduction or raw image processing
- GUI support in version 0.2
- A fully automatic black-box pipeline that hides scientific choices

## Users

### Primary user

A scientific developer learning Rust by building a real numerical tool.

### Secondary users

- SAXS researchers comfortable with CLI workflows
- Future contributors experimenting with bases, regularizers, and constraints

## Product Principles

### 1. Scientific clarity over legacy familiarity

The tool should prefer explicit mathematical structure over compatibility with
existing SAXS packages.

### 2. Smoothness is built into the model

`P(r)` should be represented using smooth basis functions rather than raw
histogram bins.

### 3. User judgment is supported, not hidden

`Dmax` sensitivity should be visible and measurable.

### 4. Code should teach

The implementation should be understandable enough that a motivated reviewer can
learn Rust and the numerical design by reading it.

## Scientific Scope

### Data model

The initial data model should support:

- `q`
- `I(q)`
- optional `sigma(q)`

Input validation should check:

- monotonically increasing `q`
- finite numeric values
- strictly positive uncertainties when provided

### Representation of `P(r)`

Version 0.2 should use a smooth basis-function representation. The preferred
default is cubic B-splines.

Why cubic B-splines:

- they encode smoothness naturally
- they avoid jagged bin artifacts
- they support principled smoothness penalties
- they keep the representation local and extensible

The architecture should make it possible to add other smooth bases later, such
as Gaussian expansions, without reworking the full application.

### Fitting model

The tool should:

- represent `P(r)` as a linear combination of basis functions
- compute predicted `I(q)` from the current basis coefficients
- fit coefficients using weighted regularized least squares
- apply support on `0 <= r <= Dmax`

### Regularization

Version 0.2 should include explicit smoothness regularization. A second-
derivative or curvature penalty is the most natural starting point.

The chosen penalty must be:

- documented in the code and docs
- configurable by the user
- reported in output metadata

### `Dmax` stability analysis

This is a core feature of version 0.2.

The tool should allow the user to:

- choose a central `Dmax`
- define a local scan range around that value
- fit solutions across that neighborhood
- compare how the resulting `P(r)`, `Rg`, `I(0)`, and fit metrics vary

This feature exists to estimate how sensitive the solution is to reasonable
subjective choices, not to claim a single authoritative `Dmax`.

## Functional Requirements

### Single-fit workflow

The tool must provide a CLI workflow for fitting one solution from one SAXS
curve and one chosen `Dmax`.

Required inputs:

- data file path
- `Dmax`

Configurable inputs:

- basis size or knot count
- regularization strength
- output directory
- `r` evaluation grid density

Required outputs:

- sampled `P(r)`
- predicted `I(q)`
- residuals
- summary metrics such as `Rg`, `I(0)`, and a goodness-of-fit statistic
- machine-readable run report

### `Dmax` scan workflow

The tool must provide a CLI workflow for running a local scan around a chosen
`Dmax`.

Required scan inputs:

- central `Dmax`
- local scan width or explicit range
- number of scan points

Required scan outputs:

- per-fit summary table across scanned `Dmax` values
- comparative metrics across the scan
- machine-readable report

## Non-Functional Requirements

- Idiomatic Rust structure and naming
- Clear module boundaries
- Comments for non-obvious mathematical or Rust-specific logic
- Deterministic outputs for identical inputs and settings
- Helpful error messages
- Testable design
- Reproducible output artifacts suitable for later comparison

## Proposed User Experience

Version 0.2 should be CLI-first.

Likely subcommands:

- `fit`
- `scan-dmax`

The tool should produce an output directory containing structured tabular data
and a summary report.

Suggested outputs:

- `pr.csv`
- `fit.csv`
- `residuals.csv`
- `report.json`
- `dmax_scan.csv` for scan mode

## Technical Design Goals

The codebase should be organized into small, focused modules with strong data
types and limited hidden coupling.

Suggested top-level structure:

- `src/data`
- `src/basis`
- `src/transform`
- `src/regularization`
- `src/solver`
- `src/analysis`
- `src/io`
- `src/cli`

Design guidelines:

- prefer explicit structs over vague maps or tuples
- use small traits only where they improve extension without obscuring flow
- keep numerical code readable before trying to make it overly generic
- include comments explaining the mathematical role of each major component

## Deferred Decisions

The following should remain open unless implementation work strongly forces a
decision:

- whether positivity constraints belong in version 0.2
- whether automatic selection of regularization strength belongs in version 0.2
- whether scan mode should also support regularization sweeps

Current recommendation:

- positivity is deferred
- regularization selection remains user-driven in 0.2
- local `Dmax` scanning is included in 0.2

## Success Criteria

Version 0.2 is successful if:

- it can fit smooth `P(r)` solutions from representative SAXS datasets
- it reports stable, interpretable outputs
- it makes nearby `Dmax` sensitivity easy to inspect
- the code is modular and documented enough to support Rust learning
- adding a second basis type later does not require major redesign

## Risks

- over-scoping the first implementation with too many solver features
- conflating scientific exploration with product complexity
- adding abstraction too early and making the code harder to learn from
- under-specifying outputs, making later comparisons difficult

## Recommended Boundaries for 0.2

Keep version 0.2 focused on:

- one smooth basis implementation
- one well-documented regularized solver path
- strong output reporting
- local `Dmax` stability analysis

Defer for later versions:

- positivity-constrained solvers
- automatic hyperparameter search
- richer visualization layers
- additional basis families
