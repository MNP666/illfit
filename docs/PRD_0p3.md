# PRD 0.3

## Working Title

Deterministic synthetic benchmark framework for SAXS iFT validation and
regression testing.

## Purpose

Version 0.3 should build on the `0.2` fitting pipeline by adding a controlled,
reproducible way to generate synthetic benchmark assets from known smooth
`P(r)` functions and evaluate how well the inversion machinery recovers them.

This iteration should prioritize four things:

1. Deterministic synthetic benchmark asset generation from known `P(r)` truth
2. Comparison of truth and recovery in both `r` space and `q` space
3. Reusable benchmark artifacts for profiling, plotting, and regression tests
4. A code structure that remains explicit, idiomatic, and easy to review

## Product Vision

The tool should not only fit experimental SAXS data, but also help us
understand the strengths and weaknesses of the inversion pipeline itself.

Version 0.3 introduces a benchmark framework where we start from a known,
scope-appropriate `P(r)`, generate synthetic `I(q)`, run the existing iFT
pipeline, and measure how well the recovered solution matches the truth.

This gives the project a principled validation path:

- we can test solver behavior on controlled shape families
- we can profile where regularization or fitting choices struggle
- we can add stronger regression tests that protect scientific behavior over
  time
- we can probe a limited but important real-world failure mode where noisy
  observed intensities contain negative values

## Background

Version `0.2` established an end-to-end iFT workflow:

- parse ASCII SAXS data
- represent `P(r)` with a cubic B-spline basis
- forward-transform to predicted `I(q)`
- solve a weighted regularized least-squares problem
- report derived metrics and export results
- scan local `Dmax` and low-`q` truncation choices

That is enough to fit real data and compare against reference outputs, but it
does not yet provide a controlled truth-recovery framework. Real datasets are
valuable, but they rarely give us direct access to the true underlying `P(r)`.

Synthetic benchmarks solve that problem by giving us known `P(r)` truth while
still exercising the same forward and inverse machinery.

## Primary Goals

- Generate deterministic synthetic SAXS benchmark assets from known smooth
  `P(r)` functions
- Use a lightweight Python-side generation workflow while keeping Rust focused
  on parsing, recovery, comparison, and regression use
- Systematically vary controlled `P(r)` families in a finite, reviewable way
- Filter generated cases to retain only benchmarks that satisfy the explicit
  `0.3` acceptance rules
- Recover `P(r)` from synthetic `I(q)` using the existing fit pipeline
- Compute truth-vs-recovery metrics in both `r` space and `q` space
- Export benchmark artifacts in a form that is easy to plot and compare
- Use benchmark cases as the foundation for stronger regression testing
- Add a small late-iteration robustness subset where Gaussian noise is applied
  to selected benchmark curves, allowing controlled negative observed
  intensities

## Non-Goals

- Random or Monte Carlo benchmark generation in version 0.3
- Full instrument simulation
- Structure-factor modeling in version 0.3
- Automatic discovery of all possible physically meaningful `P(r)` shapes
- Replacing real-data validation with synthetic validation
- GUI work

Clarification:

- a small fixed noisy-observation subset is in scope if it is built on top of
  deterministic truth cases
- broad stochastic exploration is not in scope

## Users

### Primary user

A scientific developer who wants to understand how solver and regularization
choices behave on controlled cases while continuing to learn Rust.

### Secondary users

- SAXS method developers comparing inversion behavior across shape families
- Future contributors adding new basis types, constraints, or solvers

## Product Principles

### 1. Truth should be preserved explicitly

Synthetic benchmarks are only useful if the true `P(r)` and generated `I(q)`
remain available throughout the workflow.

### 2. Deterministic beats random in the first benchmark system

Benchmark generation should be finite, reviewable, and reproducible. The same
settings should always produce the same case set.

### 3. Both spaces matter

Goodness of fit should be measurable in both:

- `r` space, where we compare recovered `P(r)` to truth
- `q` space, where we compare forward-generated and recovered scattering curves

### 4. Benchmark machinery should strengthen the rest of the project

The synthetic framework should serve profiling, scientific validation, and
regression testing, not live as an isolated side tool.

### 5. Code should remain teachable

The new machinery should favor clear data structures and comments over clever
abstractions.

## Scientific Scope

### Benchmark case generation

Version 0.3 should add a deterministic benchmark generation workflow that
starts from smooth `P(r)` definitions and produces synthetic SAXS benchmark
assets.

Each benchmark case should include:

- case identifier
- generation metadata
- `Dmax`
- `r` grid
- true `P(r)`
- `q` grid
- true synthetic `I(q)`

The first implementation should be deterministic and finite.

The benchmark generation step does not need to live in Rust. A small, explicit
Python export tool is acceptable and currently preferred if it produces stable,
reviewable benchmark assets that Rust can later parse and consume.

### Synthetic `P(r)` families

The first benchmark families do not need to be generated from the same basis
family used by the Rust recovery model.

Recommended approach:

- define one or more smooth truth families, such as clamped splines or tapered
  Gaussian-like constructions
- ensure the chosen family respects the endpoint behavior expected of `P(r)`
- generate a bounded set of smooth `P(r)` shapes by systematic variation
- label cases by shape family or construction rule

The goal is not to exhaust all possible shapes. The goal is to produce a useful
set of distinct benchmark cases with interpretable structure.

Suggested early shape families:

- clamped spline families with `P(0) = P(Dmax) = 0` and zero first derivative
  at both ends
- broad unimodal shapes
- narrower unimodal shapes
- low-`r` skewed shapes
- high-`r` skewed shapes
- shoulder-like or asymmetric shapes
- selected multi-feature shapes, if they remain physically plausible

It is acceptable, and likely preferable, for many benchmark truth cases to be
"out-of-family" with respect to the Rust recovery basis. That makes the
benchmark suite more useful for evaluating regularization behavior and model
mismatch.

### Physical plausibility filtering

Generated benchmark cases should be filtered before they are accepted into the
benchmark suite.

Version 0.3 should require at minimum:

- non-negative `P(r)` on the sampled `r` grid, up to a documented numerical
  tolerance
- `P(0) ~= 0` and `P(Dmax) ~= 0` on the sampled grid, up to a documented
  numerical tolerance
- support on `0 <= r <= Dmax`
- non-negative `I(q)` on the sampled `q` grid, up to a documented numerical
  tolerance

This is a deliberate scope boundary for version `0.3`, not a claim that all
scientifically meaningful `P(r)` functions are non-negative.

For the first synthetic benchmark system, restricting the suite to non-negative
`P(r)` cases keeps generation, screening, and interpretation simpler.

Signed `P(r)` cases are scientifically relevant in some contrast situations and
should remain a future extension rather than part of the initial benchmark
framework.

These checks do not guarantee that a case is experimentally realistic, but they
do screen out obviously invalid examples under the chosen `0.3` scope.

### Recovery benchmarking

The benchmark system should run accepted synthetic curves through the existing
fit pipeline.

For each benchmark case, it should record:

- true `P(r)`
- generated `I(q)`
- recovered `P(r)`
- back-calculated `I(q)` from the recovered solution
- derived truth and recovered metrics such as `Rg` and `I(0)`

The core benchmark path should use noiseless synthetic curves that satisfy the
chosen `0.3` acceptance rules. A smaller late-iteration extension may also
include observed variants where deterministic truth curves are perturbed by
Gaussian noise, potentially producing some negative observed intensities.

### Comparison metrics

Version 0.3 should explicitly measure truth-vs-recovery performance in both
spaces.

Recommended `r`-space metrics:

- RMSE
- normalized RMSE
- correlation
- integrated absolute error
- `Rg` error
- `I(0)` error

Recommended `q`-space metrics:

- RMSE
- normalized RMSE
- residual curves
- weighted goodness-of-fit when uncertainties are available later

For the noisy-observation subset, it is also useful to record:

- the count or fraction of negative observed intensity values
- how recovery metrics degrade as noise level increases

The metric design should remain simple and explicit in 0.3.

## Functional Requirements

### Synthetic benchmark generation workflow

The project must provide a workflow that:

- defines one or more deterministic benchmark families
- generates a finite set of candidate `P(r)` cases
- filters out physically unacceptable cases
- exports accepted cases with truth metadata

Required outputs per accepted case:

- sampled true `P(r)`
- generated synthetic `I(q)`
- case metadata

Recommended implementation boundary:

- Python handles benchmark asset generation and export
- Rust handles asset parsing, recovery, comparison, and regression use

### Benchmark recovery workflow

The project must provide a workflow that:

- takes accepted benchmark cases
- runs the existing fit machinery on the synthetic `I(q)`
- stores recovered outputs alongside truth
- computes comparison metrics in `r` and `q`

Required outputs per recovered case:

- recovered `P(r)`
- recovered fit in `q` space
- `r`-space comparison data
- `q`-space comparison data
- summary metrics

### Benchmark summary workflow

The project must provide a workflow that:

- aggregates case-level metrics across a benchmark suite
- writes plotting-friendly tabular outputs
- writes machine-readable summary reports

Required suite-level outputs:

- per-case summary table
- aggregate summary report
- enough structured output for Python-based plotting and profiling

### Noisy-observation robustness workflow

Late in version 0.3, the project may add a limited robustness workflow that:

- selects a small subset of accepted deterministic benchmark cases
- applies Gaussian noise at a small number of fixed levels
- preserves both noiseless truth and noisy observed curves
- records how recovery quality changes as negative observed intensities appear

This workflow should remain a focused extension, not the center of the version.

## Non-Functional Requirements

- Deterministic generation and recovery outputs for fixed settings
- Strongly typed, reviewable data structures
- Clear documentation for generation rules and acceptance filters
- Output formats that support plotting and regression checks
- Tests that are easy to interpret when behavior changes
- Idiomatic Rust structure and comments around non-obvious logic

## Proposed User Experience

Version 0.3 can remain CLI-first and may also expose some workflows through
profiling scripts.

Likely new CLI workflows:

- `generate-benchmarks`
- `benchmark-recovery`

Suggested outputs:

- per-case truth files
- per-case recovery files
- suite summary CSV
- suite summary JSON

The Python profiling layer can then consume those outputs for plotting and
interactive comparison.

## Technical Design Goals

The benchmark machinery should reuse as much of the existing modeling and
analysis stack as possible once benchmark assets have been generated.

Suggested new modules or submodules:

- `src/benchmark`
  - benchmark case definitions
  - benchmark asset parsers
  - recovery orchestration
  - comparison metrics
  - suite summaries

Suggested supporting tooling outside the Rust crate:

- `profiling/` benchmark export scripts that generate deterministic truth assets
  and metadata for later use by Rust

Design guidelines:

- keep truth and recovered data clearly separated in the data model
- keep benchmark generation scripts explicit and reviewable
- reuse existing Rust forward-transform and fit machinery rather than
  duplicating it for recovery
- make exported artifacts easy to inspect by hand

## Deferred Decisions

The following should remain outside core `0.3` scope unless implementation work
forces a change:

- randomness in benchmark generation during core benchmark export
- simulated noise models beyond very simple fixed noisy-observation variants
- structure-factor or interparticle-effect simulation
- positivity-constrained inversion as part of the benchmark work
- large benchmark catalogs generated by combinatorial explosion

Current recommendation:

- keep benchmark generation deterministic
- keep physical screening simple and explicit
- favor a small, interpretable benchmark suite over a huge one
- if noisy cases are added, keep them limited to a small fixed subset of truth
  cases with a few predefined noise levels

## Success Criteria

Version 0.3 is successful if:

- the project can export a reproducible suite of synthetic SAXS benchmark assets
- accepted benchmark cases are filtered for basic physical plausibility
- accepted benchmark cases satisfy the explicit `0.3` acceptance rules
- the existing fit pipeline can be run against those cases automatically
- truth-vs-recovery metrics are available in both `r` and `q` space
- benchmark outputs are useful for plotting, profiling, and regression testing
- the implementation stays modular and readable
- if the noisy-observation subset is included, it reveals how recovery behaves
  as selected cases acquire increasingly negative observed intensities

## Risks

- generating too many cases without enough interpretability
- encoding generation rules that are hard to reason about later
- overcomplicating physical screening in the first benchmark version
- coupling the benchmark system too tightly to one solver configuration
- spending too much scope on plotting instead of benchmark data products

## Recommended Boundaries for 0.3

Keep version 0.3 focused on:

- deterministic synthetic benchmark asset generation
- finite controlled truth families
- simple physical plausibility checks
- simple acceptance rules centered on non-negative `P(r)` for this iteration
- truth-vs-recovery reporting in `r` and `q` space
- benchmark outputs that support profiling and regression tests
- a small late-iteration noisy-observation subset only after the deterministic
  benchmark path is in place

Defer for later versions:

- randomized benchmark generation
- richer experimental perturbation models
- broad solver comparison frameworks
- structure-factor modeling
