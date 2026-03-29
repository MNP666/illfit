# Issues 0.4

This document breaks the `0.4` iteration into concrete work items. The goal is
to build a reproducible experiment pipeline for studying weighting strategies
and lambda selection on noiseless and noisy benchmark suites.

Status note:

- `0.4` starts from a stable `0.3` benchmark and profiling foundation
- the main new work is weighting, lambda selection, and TOML-driven experiment
  orchestration

## Milestone 1: Experiment configuration model

### Issue 1.1: Define experiment config types

Status: not started

Create explicit Rust types for the `0.4` TOML experiment configuration.

Definition of done:

- config types are explicit and documented
- weighting, lambda grid, selector, and output options are represented
- validation errors are clear

### Issue 1.2: Add TOML parsing and validation

Status: not started

Parse experiment configuration from TOML and validate it before execution.

Definition of done:

- TOML config can be loaded through Rust
- invalid experiment configs fail clearly
- a config snapshot can be written to output

## Milestone 2: Weighting framework

### Issue 2.1: Define weighting strategy abstraction

Status: not started

Add an explicit abstraction for weighting or scaling strategies.

Definition of done:

- strategy definitions are explicit
- the baseline `none` strategy remains available
- the interface is clear enough to extend later

### Issue 2.2: Implement initial weighting strategies

Status: not started

Implement the first weighting or scaling strategies.

Recommended first set:

- `none`
- `q`
- `q^2`

Definition of done:

- each strategy is implemented and tested
- transformed uncertainties are handled consistently where needed

### Issue 2.3: Benchmark weighting behavior on noiseless and noisy suites

Status: not started

Make weighting strategies runnable across both benchmark types.

Definition of done:

- weighting strategies can be applied in noiseless benchmark experiments
- weighting strategies can be applied in noisy benchmark experiments

## Milestone 3: Lambda scan framework

### Issue 3.1: Define lambda grid model

Status: not started

Add an explicit representation for lambda grids in experiment configs and
execution.

Definition of done:

- lambda grids are explicit and documented
- grids can be serialized into output metadata

### Issue 3.2: Implement lambda scan execution

Status: not started

Run fits across a lambda grid for each selected weighting strategy.

Definition of done:

- lambda scan execution works end to end
- scan summaries preserve misfit and smoothness terms
- scan summaries preserve benchmark metrics

## Milestone 4: L-curve

### Issue 4.1: Export L-curve data

Status: not started

Compute and export the data needed for L-curve analysis.

Definition of done:

- each lambda scan records data misfit and penalty terms
- L-curve data is exported in plot-ready form

### Issue 4.2: Implement L-curve lambda selection

Status: not started

Select lambda from the L-curve.

Definition of done:

- one clear L-curve selection method is implemented
- selected lambda is recorded in outputs
- behavior is covered by tests

## Milestone 5: GCV

### Issue 5.1: Compute GCV scores across lambda

Status: not started

Add GCV evaluation to lambda scans.

Definition of done:

- GCV score is computed for each lambda
- results are exported in plot-ready form

### Issue 5.2: Implement GCV lambda selection

Status: not started

Choose lambda by minimizing the GCV score.

Definition of done:

- selected lambda is recorded in outputs
- behavior is covered by tests

## Milestone 6: Experiment reporting

### Issue 6.1: Write experiment-level summary tables

Status: not started

Write suite-level and strategy-level summaries for one experiment run.

Definition of done:

- summaries compare weighting strategies
- summaries compare selected lambda values by method
- outputs are easy to inspect in CSV and JSON form

### Issue 6.2: Write selected-fit outputs

Status: not started

Preserve detailed fit outputs for selected lambda values.

Definition of done:

- selected `P(r)` and `I(q)` outputs are written
- output selection is documented in the experiment report

## Milestone 7: CLI workflow

### Issue 7.1: Add TOML-driven experiment CLI command

Status: not started

Provide the main `0.4` experiment entry point.

Definition of done:

- one CLI command runs the experiment from a TOML file
- help text is clear
- config and output paths are explicit

### Issue 7.2: Create fresh output folders under profiling

Status: not started

Write each experiment run into a new folder under
[`profiling/output/`](/Users/air/Documents/illfit/profiling/output).

Definition of done:

- output folders are fresh per run
- prior results are not overwritten by default

## Milestone 8: Profiling and visualization

### Issue 8.1: Add L-curve plotting helper

Status: not started

Add a Python plotting script for L-curve outputs.

Definition of done:

- one script can plot L-curve outputs from a `0.4` run

### Issue 8.2: Add lambda-scan comparison plotting helper

Status: not started

Add a Python plotting script for comparing weighting strategies across lambda.

Definition of done:

- one script can plot strategy comparisons from a `0.4` run

### Issue 8.3: Document the full 0.4 experiment workflow

Status: not started

Document how to define a run in TOML and inspect the outputs.

Definition of done:

- README or profiling docs explain the workflow
- example config is included

## Milestone 9: Regression and test coverage

### Issue 9.1: Add tests for weighting logic

Status: not started

Protect the mathematical behavior of weighting and scaling code.

Definition of done:

- unit tests cover baseline and initial weighting strategies

### Issue 9.2: Add tests for lambda selectors

Status: not started

Protect L-curve and GCV calculations against accidental regressions.

Definition of done:

- selector logic is covered by unit or integration tests

### Issue 9.3: Add end-to-end experiment test

Status: not started

Run one small TOML-driven experiment in tests.

Definition of done:

- one end-to-end experiment path is covered
- output artifacts are verified

## Stretch ideas for 0.4

These are intentionally outside the core committed scope:

- alternative linear solve formulations for comparison
- positivity-aware optimization
- robust losses for noisy observed data
- per-case adaptive lambda reporting beyond the first selectors
