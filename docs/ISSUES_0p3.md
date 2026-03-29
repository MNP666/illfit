# Issues 0.3

This document breaks the `0.3` iteration into concrete work items. The goal is
to turn the synthetic benchmark idea into an explicit, testable implementation
without losing scientific clarity or Rust readability.

## Milestone 1: Benchmark data model

### Issue 1.1: Define synthetic benchmark case types

Create explicit Rust types for synthetic truth cases, recovered benchmark
results, and benchmark suite summaries.

Definition of done:

- truth-case structs are explicit and documented
- recovery-result structs are explicit and documented
- suite-summary structs are explicit and documented

### Issue 1.2: Add benchmark file I/O types

Define the serialized output model for per-case truth data, per-case recovery
data, and suite summary artifacts.

Definition of done:

- output structs match planned CSV/JSON artifacts
- file naming conventions are documented in code comments or docs

## Milestone 2: Deterministic synthetic `P(r)` asset generation

### Issue 2.1: Create a structured Python benchmark-export tool

Build a small Python tool that exports deterministic synthetic benchmark assets
for later use by the Rust codebase.

Definition of done:

- export tool is deterministic
- accepted cases are written with metadata
- output layout is stable and reviewable

### Issue 2.2: Implement initial clamped-spline truth families

Use clamped spline truth generation with controlled seeds or parameter grids to
export realistic `P(r)` benchmark cases.

Definition of done:

- generated truth cases satisfy endpoint expectations by construction
- generation settings are recorded
- cases are suitable for later review and selection

### Issue 2.3: Label and organize benchmark shape families

Make generated cases interpretable by assigning family labels or construction
labels rather than producing an opaque list of cases.

Definition of done:

- cases carry family or pattern labels
- labels are suitable for later reporting and plotting

## Milestone 3: Physical plausibility screening

### Issue 3.1: Implement basic `P(r)` validity screening

Reject synthetic cases whose sampled `P(r)` behavior is obviously unphysical.

Definition of done:

- non-negativity checks are implemented with documented tolerance
- endpoint checks for `P(0)` and `P(Dmax)` are implemented with documented
  tolerance
- support assumptions are explicit
- rejected cases preserve a reason for rejection

### Issue 3.2: Implement basic `I(q)` validity screening

Reject synthetic cases whose forward-generated SAXS curve is obviously invalid
over the sampled `q` grid.

Definition of done:

- non-negativity checks are implemented with documented tolerance
- rejected cases preserve a reason for rejection

## Milestone 4: Benchmark generation outputs

### Issue 4.1: Export accepted synthetic truth cases

Write accepted benchmark cases to disk in a plotting-friendly and
machine-readable format.

Definition of done:

- true `P(r)` is written
- true synthetic `I(q)` is written
- case metadata is written

### Issue 4.2: Export rejected-case summaries

Write a summary of screened-out candidate cases so generation decisions are
inspectable.

Definition of done:

- rejected cases are counted
- rejection reasons are recorded
- output is suitable for later review

## Milestone 5: Recovery benchmarking

### Issue 5.1: Run the existing fit pipeline against synthetic cases

Reuse the current fit machinery to recover `P(r)` from accepted synthetic SAXS
curves.

Definition of done:

- synthetic curves can be fed through the normal recovery pipeline
- recovered results are preserved per case
- fit configuration is recorded per case

### Issue 5.2: Preserve truth and recovery together

Ensure each recovery result includes enough information to compare synthetic
truth and recovered outputs without reconstructing context elsewhere.

Definition of done:

- truth and recovered data are linked by case id
- both `r`-space and `q`-space outputs are available for comparison

## Milestone 6: Truth-vs-recovery metrics

### Issue 6.1: Implement `r`-space comparison metrics

Add metrics comparing true and recovered `P(r)` curves.

Definition of done:

- RMSE is reported
- normalized RMSE is reported
- correlation is reported
- integrated absolute error is reported
- `Rg` and `I(0)` errors are reported

### Issue 6.2: Implement `q`-space comparison metrics

Add metrics comparing true synthetic `I(q)` and recovered back-calculated
`I(q)`.

Definition of done:

- RMSE is reported
- normalized RMSE is reported
- residual curves are preserved

## Milestone 7: Benchmark suite reporting

### Issue 7.1: Write per-case recovery artifacts

Export truth, recovery, and comparison outputs for each benchmark case.

Definition of done:

- per-case files contain truth and recovery outputs
- per-case files contain comparison summaries
- outputs are usable from Python plotting tools

### Issue 7.2: Write suite-level benchmark summaries

Aggregate case-level metrics into suite-level outputs for profiling and review.

Definition of done:

- per-case summary table is written
- suite summary JSON is written
- outputs are easy to inspect and compare across runs

## Milestone 8: CLI and workflow integration

### Issue 8.1: Implement Rust-side benchmark asset loading workflow

Provide a Rust-side workflow for loading exported synthetic benchmark assets.

Definition of done:

- benchmark asset paths are exposed on the CLI
- accepted exported assets can be loaded without ad hoc conversion
- help text is clear

### Issue 8.2: Implement benchmark recovery CLI command

Provide a command-line workflow for running recovery and comparison across a
benchmark suite.

Definition of done:

- recovery parameters are exposed on the CLI
- output layout is consistent
- help text is clear

### Issue 8.3: Update profiling workflow to consume benchmark outputs

Extend the Python-side profiling tooling so it can plot and summarize synthetic
benchmark results, not only `.out`-based reference comparisons.

Definition of done:

- profiling tooling can read benchmark outputs
- benchmark plots or summaries can be produced without ad hoc file handling

## Milestone 9: Regression testing foundation

### Issue 9.1: Add deterministic regression benchmark cases

Promote a small number of accepted synthetic benchmark cases into standing
regression assets.

Definition of done:

- a compact deterministic benchmark subset is selected
- benchmark identities and expected behavior are documented

### Issue 9.2: Add regression tests based on benchmark recovery behavior

Use the benchmark subset to protect important scientific behavior over time.

Definition of done:

- tests cover at least one truth-recovery path end to end
- tolerances are documented
- failures are interpretable

## Milestone 10: Noisy-observation robustness subset

Scope note:

- this milestone is intentionally late in the iteration
- it should only be started after the deterministic benchmark machinery is in
  place
- the goal is not broad stochastic exploration, but a small fixed subset of
  cases that helps us study a realistic failure mode where observed noisy SAXS
  curves contain negative values

### Issue 10.1: Add fixed Gaussian-noise variants for selected benchmark cases

Create a small number of noisy observed curves from selected deterministic truth
cases using predefined Gaussian noise levels.

Definition of done:

- a small benchmark subset is selected
- noisy observed curves are generated at fixed levels
- noiseless truth and noisy observations are both preserved

### Issue 10.2: Record negative-intensity statistics for noisy cases

Track how often noisy observed curves become partially negative.

Definition of done:

- per-case negative-value count or fraction is reported
- noise level is recorded with each noisy observed case

### Issue 10.3: Compare recovery degradation across noisy variants

Measure how recovery quality changes as noise increases for the selected noisy
benchmark subset.

Definition of done:

- `r`-space and `q`-space comparison metrics are preserved for noisy cases
- per-case or per-noise-level summaries make degradation easy to inspect
- outputs are suitable for plotting and profiling

## Stretch ideas for 0.3

These are intentionally not part of the core committed scope:

- multiple solver comparison in one benchmark command
- richer shape-family catalogs
- benchmark ranking or dashboards

## Exit criteria for the 0.3 iteration

The iteration is complete when:

- deterministic synthetic benchmark generation works end to end
- accepted cases are screened for basic plausibility
- recovery benchmarking works end to end
- truth-vs-recovery metrics are available in both `r` and `q` space
- benchmark outputs are useful for plotting and regression tests
- if the noisy-observation subset is taken on, it works end to end for a small
  fixed subset of cases
- the code and docs remain clear enough to support Rust learning during review
