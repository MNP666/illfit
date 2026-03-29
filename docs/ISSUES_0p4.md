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

Status: complete

Create explicit Rust types for the `0.4` TOML experiment configuration.

Current note:

- [`src/experiment/config.rs`](/Users/air/Documents/illfit/src/experiment/config.rs)
  now defines typed config structs for suite selection, recovery settings,
  weighting strategies, lambda grids, selector methods, and output settings

Definition of done:

- config types are explicit and documented
- weighting, lambda grid, selector, and output options are represented
- validation errors are clear

### Issue 1.2: Add TOML parsing and validation

Status: complete

Parse experiment configuration from TOML and validate it before execution.

Current note:

- TOML parsing and validation now work through
  [`parse_experiment_config(...)`](/Users/air/Documents/illfit/src/experiment/config.rs)
  and
  [`parse_experiment_config_str(...)`](/Users/air/Documents/illfit/src/experiment/config.rs)
- an example config now exists at
  [`profiling/regularization_0p4.toml`](/Users/air/Documents/illfit/profiling/regularization_0p4.toml)
- experiment output bundles now also write a full config snapshot as
  [`experiment_config.toml`](/Users/air/Documents/illfit/src/io/results.rs)

Definition of done:

- TOML config can be loaded through Rust
- invalid experiment configs fail clearly
- a config snapshot can be written to output

## Milestone 2: Weighting framework

### Issue 2.1: Define weighting strategy abstraction

Status: complete

Add an explicit abstraction for weighting or scaling strategies.

Current note:

- [`src/weighting/mod.rs`](/Users/air/Documents/illfit/src/weighting/mod.rs)
  now defines the first weighting abstraction and parsing path

Definition of done:

- strategy definitions are explicit
- the baseline `none` strategy remains available
- the interface is clear enough to extend later

### Issue 2.2: Implement initial weighting strategies

Status: complete

Implement the first weighting or scaling strategies.

Recommended first set:

- `none`
- `q`
- `q^2`

Definition of done:

- each strategy is implemented and tested
- transformed uncertainties are handled consistently where needed

Current note:

- the first shipped set is now implemented:
  - `none`
  - `q`
  - `q2`
  - generic `q^alpha`
- transformed observations currently rescale both intensity and sigma
  consistently through the weighting layer

### Issue 2.3: Benchmark weighting behavior on noiseless and noisy suites

Status: complete

Make weighting strategies runnable across both benchmark types.

Current note:

- [`src/experiment/regularization.rs`](/Users/air/Documents/illfit/src/experiment/regularization.rs)
  now runs weighting strategies across both noiseless benchmark suites and
  noisy observed benchmark suites
- coverage exists for both paths in experiment tests

Definition of done:

- weighting strategies can be applied in noiseless benchmark experiments
- weighting strategies can be applied in noisy benchmark experiments

## Milestone 3: Lambda scan framework

### Issue 3.1: Define lambda grid model

Status: complete

Add an explicit representation for lambda grids in experiment configs and
execution.

Current note:

- lambda grids are now represented through
  [`LambdaGridConfig`](/Users/air/Documents/illfit/src/experiment/config.rs)
  and parsed from TOML experiment configs

Definition of done:

- lambda grids are explicit and documented
- grids can be serialized into output metadata

### Issue 3.2: Implement lambda scan execution

Status: complete

Run fits across a lambda grid for each selected weighting strategy.

Current note:

- the experiment runner in
  [`src/experiment/regularization.rs`](/Users/air/Documents/illfit/src/experiment/regularization.rs)
  now executes full weighting-by-lambda scans across benchmark suites
- per-case and per-strategy summaries are already written through
  [`src/io/results.rs`](/Users/air/Documents/illfit/src/io/results.rs)
- selector-facing quantities are now preserved too:
  - mean data misfit
  - mean regularization penalty
  - mean effective degrees of freedom
  - mean GCV score
  - L-curve curvature estimates

Definition of done:

- lambda scan execution works end to end
- scan summaries preserve misfit and smoothness terms
- scan summaries preserve benchmark metrics

## Milestone 4: L-curve

### Issue 4.1: Export L-curve data

Status: complete

Current note:

- plot-ready L-curve data now writes through
  [`src/io/results.rs`](/Users/air/Documents/illfit/src/io/results.rs) as
  `l_curve.csv`
- each lambda point preserves mean data misfit, mean regularization penalty,
  curvature, and a selection flag

Compute and export the data needed for L-curve analysis.

Definition of done:

- each lambda scan records data misfit and penalty terms
- L-curve data is exported in plot-ready form

### Issue 4.2: Implement L-curve lambda selection

Status: complete

Current note:

- a first L-curve selector now runs in
  [`src/experiment/regularization.rs`](/Users/air/Documents/illfit/src/experiment/regularization.rs)
  by choosing the maximum discrete curvature on the log-misfit/log-penalty
  curve
- selected lambdas are exported to `selected_lambdas.csv` and summarized in
  `experiment_report.json`

Select lambda from the L-curve.

Definition of done:

- one clear L-curve selection method is implemented
- selected lambda is recorded in outputs
- behavior is covered by tests

## Milestone 5: GCV

### Issue 5.1: Compute GCV scores across lambda

Status: complete

Current note:

- per-case and mean GCV scores are now computed in
  [`src/experiment/regularization.rs`](/Users/air/Documents/illfit/src/experiment/regularization.rs)
- plot-ready GCV data now writes as `gcv.csv`

Add GCV evaluation to lambda scans.

Definition of done:

- GCV score is computed for each lambda
- results are exported in plot-ready form

### Issue 5.2: Implement GCV lambda selection

Status: complete

Current note:

- the first GCV selector now chooses the lambda with the minimum mean GCV score
  for each weighting strategy
- selected lambdas are exported alongside the L-curve picks in
  `selected_lambdas.csv`

Choose lambda by minimizing the GCV score.

Definition of done:

- selected lambda is recorded in outputs
- behavior is covered by tests

## Milestone 6: Experiment reporting

### Issue 6.1: Write experiment-level summary tables

Status: complete

Write suite-level and strategy-level summaries for one experiment run.

Current note:

- the experiment bundle now writes:
  - `case_results.csv`
  - `strategy_summary.csv`
  - `l_curve.csv`
  - `gcv.csv`
  - `selected_lambdas.csv`
  - `experiment_config.toml`
  - `experiment_report.json`
- selected lambda values by method are now included in the CSV and JSON outputs

Definition of done:

- summaries compare weighting strategies
- summaries compare selected lambda values by method
- outputs are easy to inspect in CSV and JSON form

### Issue 6.2: Write selected-fit outputs

Status: complete

Current note:

- selector-chosen detailed outputs now write through
  [`src/io/results.rs`](/Users/air/Documents/illfit/src/io/results.rs) under a
  dedicated `selected/` subtree
- each selected weighting/method/lambda combination gets per-case truth,
  recovery, and comparison artifacts plus a `selected_case_summary.csv`

Preserve detailed fit outputs for selected lambda values.

Definition of done:

- selected `P(r)` and `I(q)` outputs are written
- output selection is documented in the experiment report

## Milestone 7: CLI workflow

### Issue 7.1: Add TOML-driven experiment CLI command

Status: complete

Current note:

- the new CLI entry point is now
  `profile-regularization --config <path>` in
  [`src/cli.rs`](/Users/air/Documents/illfit/src/cli.rs)
- it loads the TOML config, runs the experiment, writes the output bundle, and
  prints the run directory

Provide the main `0.4` experiment entry point.

Definition of done:

- one CLI command runs the experiment from a TOML file
- help text is clear
- config and output paths are explicit

### Issue 7.2: Create fresh output folders under profiling

Status: complete

Current note:

- experiment runs now create fresh folders under the configured output root
  using `run_name` plus a high-resolution timestamp
- the default configured root remains under
  [`profiling/output/`](/Users/air/Documents/illfit/profiling/output)

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

Status: in progress

Document how to define a run in TOML and inspect the outputs.

Current note:

- a first example config now exists at
  [`profiling/regularization_0p4.toml`](/Users/air/Documents/illfit/profiling/regularization_0p4.toml)
- the CLI command and selector outputs now exist, so the remaining work is to
  document the end-to-end usage and plotting flow in the profiling docs or
  README

Definition of done:

- README or profiling docs explain the workflow
- example config is included

## Milestone 9: Regression and test coverage

### Issue 9.1: Add tests for weighting logic

Status: complete

Protect the mathematical behavior of weighting and scaling code.

Current note:

- unit tests in [`src/weighting/mod.rs`](/Users/air/Documents/illfit/src/weighting/mod.rs)
  now cover strategy parsing and consistent observation transformation

Definition of done:

- unit tests cover baseline and initial weighting strategies

### Issue 9.2: Add tests for lambda selectors

Status: in progress

Current note:

- experiment tests now cover:
  - selector result generation
  - finite GCV scores
  - non-trivial weighting changes in summary metrics
- a more focused selector-only unit test would still be useful later

Protect L-curve and GCV calculations against accidental regressions.

Definition of done:

- selector logic is covered by unit or integration tests

### Issue 9.3: Add end-to-end experiment test

Status: complete

Run one small TOML-driven experiment in tests.

Current note:

- experiment parsing, execution, and artifact writing are now covered in tests
  under [`src/experiment/regularization.rs`](/Users/air/Documents/illfit/src/experiment/regularization.rs)
  and [`src/io/results.rs`](/Users/air/Documents/illfit/src/io/results.rs)

Definition of done:

- one end-to-end experiment path is covered
- output artifacts are verified

## Stretch ideas for 0.4

These are intentionally outside the core committed scope:

- alternative linear solve formulations for comparison
- positivity-aware optimization
- robust losses for noisy observed data
- per-case adaptive lambda reporting beyond the first selectors
