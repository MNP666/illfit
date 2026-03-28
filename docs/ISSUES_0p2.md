# Issues 0.2

This document breaks the `0.2` iteration into concrete work items. The goal is
to preserve momentum without losing the scientific or educational objectives of
the project.

## Milestone 1: Project skeleton and conventions

### Issue 1.1: Initialize Rust crate and workspace conventions

Status: complete

Create the initial Rust project layout and establish conventions for modules,
formatting, linting, and tests.

Definition of done:

- Rust project initializes successfully
- basic crate structure exists
- formatting and linting commands are documented
- test command is documented

### Issue 1.2: Write high-level architecture notes in code

Add crate-level or module-level comments that explain the role of the main
components so the codebase remains reviewable while it grows.

Definition of done:

- main modules have top-level comments
- design intent is discoverable from source

## Milestone 2: Data ingestion and validation

### Issue 2.1: Implement SAXS data parser

Read text or CSV SAXS curves with columns for `q`, `I(q)`, and optional
`sigma(q)`.

Definition of done:

- parser reads expected formats
- optional uncertainties are supported
- parsing errors are actionable

### Issue 2.2: Add validated SAXS curve type

Represent parsed data with a validated domain type instead of passing raw
vectors throughout the code.

Definition of done:

- monotonic `q` is enforced
- invalid numeric values are rejected
- uncertainty handling is explicit

## Milestone 3: Basis and transform

### Issue 3.1: Implement cubic B-spline basis representation for `P(r)`

Create the core basis representation and evaluation machinery for smooth
`P(r)` modeling on `0 <= r <= Dmax`.

Definition of done:

- basis construction is parameterized by `Dmax` and basis resolution
- basis functions can be evaluated on an `r` grid
- implementation is documented with comments around non-obvious math

### Issue 3.2: Implement forward transform to predicted `I(q)`

Compute predicted scattering intensities from basis coefficients.

Definition of done:

- forward transform is tested on simple cases
- numerical assumptions are documented
- interface integrates with validated data types

## Milestone 4: Solver and regularization

### Issue 4.1: Implement weighted regularized least-squares solver

Solve for basis coefficients using measured uncertainties when available and a
documented smoothness penalty.

Definition of done:

- objective function is clearly defined
- regularization strength is configurable
- solver returns coefficients and fit diagnostics

### Issue 4.2: Implement smoothness penalty construction

Construct the penalty operator used to favor smooth `P(r)` solutions.

Definition of done:

- chosen penalty is documented
- penalty integrates cleanly with the solver
- tests cover shape and basic behavior

## Milestone 5: Derived quantities and reporting

### Issue 5.1: Compute derived summary metrics

Add derived outputs such as `Rg`, `I(0)`, and fit-quality statistics.

Definition of done:

- summary metrics are reported from fit results
- tests cover basic correctness on controlled examples

### Issue 5.2: Write result export layer

Export fit results as tabular files and a machine-readable summary report.

Definition of done:

- `pr.csv` is written
- `fit.csv` is written
- `residuals.csv` is written
- `report.json` is written

## Milestone 6: `Dmax` stability analysis

Scope note:

- the original `0.2` plan for Milestone 6 focused only on local `Dmax`
  sensitivity
- the truncation-scan issues below were added on 2026-03-28 after implementation
  work on earlier milestones had already started
- this is intentional scope growth to capture another important user judgment:
  how many low-`q` points to exclude before fitting

### Issue 6.1: Implement local `Dmax` scan workflow

Run the same fitting machinery across a neighborhood around a chosen `Dmax`.

Definition of done:

- scan parameters are user-configurable
- scan reuses the same underlying fit pipeline
- per-scan results are collected consistently

### Issue 6.2: Add scan summary metrics

Summarize how sensitive the solution is to nearby `Dmax` choices.

Definition of done:

- scan output includes per-`Dmax` fit statistics
- variation in `Rg` and `I(0)` is reported
- output is suitable for later plotting or comparison

### Issue 6.3: Implement local low-`q` truncation scan workflow

Status: added after work started

Run the same fitting machinery across a neighborhood around a user-chosen low-`q`
truncation point, expressed initially as "drop the first N points".

This should be a local scan around a user-selected starting value rather than a
global search. The expectation is that the starting truncation is usually
reasonable, while nearby values may reveal how sensitive the inferred solution
is to that choice.

Definition of done:

- user can specify a baseline dropped-point count
- user can scan nearby truncations in fixed steps, with step size `5` points as
  the initial intended workflow
- each scan entry records both dropped-point count and resulting minimum
  retained `q`
- scan reuses the same underlying fit pipeline as standard fits and `Dmax`
  scans

### Issue 6.4: Handle degraded nearby truncation fits and summarize truncation stability

Status: added after work started

When scanning nearby truncation choices, adding more low-`q` data can
reasonably produce poor fits or poor `P(r)` behavior. The tool should detect,
record, and report these cases instead of assuming all nearby scan points are
valid or equally interpretable.

Definition of done:

- truncation scan output reports per-scan fit statistics such as `Rg`, `I(0)`,
  and fit quality metrics for successful fits
- scan output records when a nearby truncation produces a failed fit or a fit
  flagged as poor or suspicious
- degraded nearby results are preserved in the summary rather than silently
  discarded
- output is suitable for later plotting or comparison, including scans with a
  mix of acceptable and poor nearby fits

## Milestone 7: CLI and ergonomics

### Issue 7.1: Implement `fit` CLI command

Provide a command-line entry point for a single fit.

Definition of done:

- user can specify data path and `Dmax`
- outputs are written to a chosen directory
- help text is clear

### Issue 7.2: Implement `scan-dmax` CLI command

Provide a command-line entry point for local `Dmax` scanning.

Definition of done:

- scan parameters are exposed on the CLI
- output layout is consistent with fit mode
- help text is clear

### Issue 7.3: Implement `scan-truncation` CLI command

Status: added after work started

Provide a command-line entry point for local low-`q` truncation scanning.

Definition of done:

- baseline dropped-point count is exposed on the CLI
- nearby truncation scan parameters are exposed on the CLI
- output layout is consistent with other scan modes
- degraded or failed nearby scan entries are preserved in exported output
- help text is clear

## Milestone 8: Testing and developer quality

### Issue 8.1: Add unit tests for core numerical pieces

Status: complete

Current note:

- parser and validation tests are implemented
- basis, transform, regularization, solver, analysis, scan, export, and CLI
  flows all have dedicated tests in the current `0.2` implementation

Cover parser behavior, basis evaluation, transform logic, regularization
construction, and derived metrics.

Definition of done:

- tests cover main numerical modules
- failures are easy to interpret

### Issue 8.2: Add small reference or synthetic datasets for validation

Include compact datasets that make it easy to validate behavior without needing
large external dependencies.

Definition of done:

- at least one synthetic dataset exists
- expected behavior is documented

### Issue 8.3: Document developer workflow

Status: complete

Explain how to build, test, lint, and review the project so the repository stays
friendly to learning and contribution.

Definition of done:

- developer workflow is documented
- commands for formatting, linting, and testing are listed

## Stretch ideas for 0.2

These are intentionally not part of the core committed scope:

- positivity constraints for `P(r)`
- automatic regularization selection
- additional basis families
- built-in plotting

## Exit criteria for the 0.2 iteration

The iteration is complete when:

- the core fit path works end-to-end
- `Dmax` scan mode works end-to-end
- outputs are reproducible and structured
- tests cover the major scientific building blocks
- the code and docs are clear enough to support Rust learning during review

## Closure status

Iteration status: closed as complete on 2026-03-29

Finished in `0.2`:

- `1.1` Initialize Rust crate and workspace conventions
- `1.2` Write high-level architecture notes in code
- `2.1` Implement SAXS data parser
- `2.2` Add validated SAXS curve type
- `3.1` Implement cubic B-spline basis representation for `P(r)`
- `3.2` Implement forward transform to predicted `I(q)`
- `4.1` Implement weighted regularized least-squares solver
- `4.2` Implement smoothness penalty construction
- `5.1` Compute derived summary metrics
- `5.2` Write result export layer
- `6.1` Implement local `Dmax` scan workflow
- `6.2` Add scan summary metrics
- `6.3` Implement local low-`q` truncation scan workflow
- `6.4` Handle degraded nearby truncation fits and summarize truncation stability
- `7.1` Implement `fit` CLI command
- `7.2` Implement `scan-dmax` CLI command
- `7.3` Implement `scan-truncation` CLI command
- `8.1` Add unit tests for core numerical pieces
- `8.3` Document developer workflow

Deferred beyond `0.2`:

- `8.2` Add small reference or synthetic datasets for validation
  Reason: the repository now contains compact real reference datasets, but it
  does not yet contain a dedicated synthetic validation dataset with documented
  expected behavior.
- positivity constraints for `P(r)`
- automatic regularization selection
- additional basis families
- built-in plotting

Testing note for `8.1`:

- `8.1` is reasonable to close for `0.2`. The codebase now has targeted tests
  for the parser, validated data model, basis evaluation, forward transform,
  regularization, solver behavior, derived metrics, scan workflows, export
  writing, and CLI parsing.
- What remains for later is not basic unit-test coverage, but stronger
  scientific regression coverage: synthetic truth-recovery cases, more
  reference-dataset comparisons, and parameter-sensitivity regression checks.
