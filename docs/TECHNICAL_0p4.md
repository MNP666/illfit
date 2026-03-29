# Technical Note for 0.4

## Purpose

This note summarizes:

- the mathematical formulation currently implemented in `illfit`
- what "regularization weight selection" means in the current codebase
- candidate `0.4` directions that are worth testing with the synthetic
  benchmark and noisy-benchmark machinery introduced in `0.3`

This is not yet a PRD. It is a technical framing document meant to help decide
what should go into `PRD_0p4.md`.

## Current model

### Real-space representation

`illfit` currently represents `P(r)` in a cubic B-spline basis on
`0 <= r <= Dmax`:

`P(r) = sum_j c_j B_j(r)`

where:

- `B_j(r)` are cubic B-spline basis functions
- `c_j` are the fitted coefficients

The basis is currently:

- cubic
- open and clamped
- defined on a fixed `Dmax`

### Forward transform

The current forward model is:

`I(q) = integral_0^Dmax P(r) * sinc(q r) dr`

with:

`sinc(x) = sin(x) / x`, and `sinc(0) = 1`

Substituting the spline expansion gives:

`I(q) = sum_j c_j integral_0^Dmax B_j(r) * sinc(q r) dr`

This is written in matrix form as:

`y_hat = A c`

where:

- `A` is the forward-transform design matrix
- `A_ij = integral_0^Dmax B_j(r) * sinc(q_i r) dr`

### Numerical quadrature

The current code approximates the forward integral with a composite midpoint
rule:

`A_ij ~= sum_k B_j(r_k) * sinc(q_i r_k) * Delta r`

with:

- `r_k = (k + 1/2) * Delta r`
- `Delta r = Dmax / N_intervals`

This is simple, explicit, and easy to inspect, which was a good fit for
`0.2` and `0.3`.

## Current inverse problem

### Objective

The current solver minimizes:

`||W (A c - y)||^2 + lambda ||L c||^2`

where:

- `y` is the observed SAXS intensity vector
- `A c` is the predicted intensity vector
- `W` is a diagonal weight matrix
- `W_ii = 1 / sigma_i`
- `L` is the regularization operator
- `lambda >= 0` is the regularization strength

### Current regularization operator

The current regularization is a second-difference penalty on neighboring spline
coefficients:

`(L c)_i = c_i - 2 c_(i+1) + c_(i+2)`

So the penalty term is:

`||L c||^2 = sum_i (c_i - 2 c_(i+1) + c_(i+2))^2`

This is a discrete curvature penalty. It prefers coefficient sequences that are
smooth and discourages oscillatory `P(r)` solutions.

### Normal equations

The solver forms:

`(A^T W^T W A + lambda L^T L) c = A^T W^T W y`

and solves that system with a Cholesky-based solver for symmetric
positive-definite matrices.

### Current interpretation

So, in plain terms, the present method is:

- fit raw `I(q)`
- weight by experimental uncertainty through `1 / sigma`
- smooth the coefficient sequence through a second-difference penalty
- solve the regularized normal equations directly

## How regularization weight is selected today

### Current status

There is currently no automatic selection of `lambda`.

The regularization strength is:

- supplied manually by the user
- passed through the CLI or profiling configuration
- used directly in the solver

In other words:

- `lambda` is a user-chosen hyperparameter
- the code currently does not compute an "optimal" value
- the current synthetic benchmark machinery is now available to test how
  sensitive recovery is to that choice

### Practical consequence

At present, regularization selection is external to the solver:

- the user chooses `lambda`
- synthetic benchmarks and noisy benchmarks can now be used to compare those
  choices
- there is no built-in L-curve, discrepancy principle, cross-validation, or
  evidence optimization yet

This is an important `0.4` opportunity.

## What is currently being fit

The present objective is applied to the measured intensity itself:

`y_i = I(q_i)`

That means:

- low-`q` points can dominate because intensities are often much larger there
- high-`q` structure can become relatively underemphasized
- noisy tails can still be difficult, especially once negative intensities
  appear

This is exactly why transformed or rescaled fitting targets are a plausible
`0.4` direction.

## Candidate 0.4 direction A: scaled data fits

### Motivation

A common idea is to fit a rescaled observable such as:

- `q I(q)`
- `q^2 I(q)`
- more generally `s(q) I(q)` for some scaling function `s(q)`

The motivation is to reduce the dynamic range across the curve so that the fit
is not dominated almost entirely by the lowest-`q` region.

### Important implementation point

If the target is transformed, the uncertainty model must be transformed too.

If:

`y'_i = s(q_i) y_i`

then the uncertainty should become:

`sigma'_i = |s(q_i)| sigma_i`

and the weighted residual should be formed consistently against `y'` and
`sigma'`.

Otherwise the optimization target becomes internally inconsistent.

### Equivalent formulation

A scaled-data objective can be written as:

`||W' (S A c - S y)||^2 + lambda ||L c||^2`

where:

- `S` is a diagonal scaling matrix with entries `s(q_i)`
- `W'` is built from transformed uncertainties

In practice, if `sigma'_i = |s(q_i)| sigma_i`, some scalings may partially
cancel in the weighted objective. That is not a bug; it means we should be
clear about what we are actually trying to emphasize:

- visual flattening for plotting is one thing
- changing the optimization emphasis is another

This is worth testing carefully rather than assuming `q^2 I(q)` automatically
improves the fit in the desired way.

### Variants worth testing

1. Direct target scaling

- fit `I(q)`
- fit `q I(q)`
- fit `q^2 I(q)`

2. Generalized power-law scaling

- fit `q^alpha I(q)` for selected fixed `alpha`
- for example `alpha in {0.5, 1.0, 1.5, 2.0}`

3. Weight-only rebalancing

Instead of changing the target, keep `y = I(q)` but modify the effective
weights, for example:

- `w_i = 1 / sigma_i`
- `w_i = q_i^alpha / sigma_i`

This may be easier to interpret than changing both the data and the model.

### Why this is attractive for 0.4

This is a good first `0.4` direction because:

- it is easy to implement
- it is easy to benchmark with the new synthetic suites
- it can be compared directly on noiseless and noisy cases
- it may improve the balance between low-`q` and high-`q` behavior

## Candidate 0.4 direction B: automatic lambda selection

### Motivation

The new benchmark system gives us a principled way to compare regularization
selection strategies. Since `lambda` is manual today, this is probably one of
the highest-value technical extensions.

### Approaches worth considering

#### 1. Grid search on synthetic benchmarks

This is the simplest project-level approach:

- choose a grid of `lambda` values
- recover all benchmark cases
- score them in `r` and `q`
- identify robust regions, not just single winners

This is not an automatic per-dataset selector, but it is likely the easiest and
most useful first step.

#### 2. L-curve

Choose `lambda` from the tradeoff curve between:

- data misfit `||W(Ac - y)||^2`
- smoothness penalty `||L c||^2`

This is classic and easy to explain, but the chosen corner can be sensitive to
how the curve is discretized.

#### 3. Discrepancy principle

Choose `lambda` so that the weighted residual is compatible with the expected
noise level, roughly:

`||W(Ac - y)||^2 ~= N_dof`

This is attractive when uncertainties are trustworthy.

#### 4. Generalized cross-validation (GCV)

Choose `lambda` by minimizing a predictive-risk proxy based on the effective
degrees of freedom of the regularized fit.

This is appealing because it is automated and widely used, but it requires a
bit more care in implementation and interpretation.

#### 5. UPRE / predictive risk methods

Unbiased predictive risk estimators are another standard path for selecting
regularization strength when the noise model is reasonably known.

#### 6. Empirical Bayes / evidence optimization

View the smoothness penalty as a Gaussian prior and estimate the prior strength
from the data. This is principled, but probably a larger conceptual step.

### Recommendation

For `0.4`, I would separate:

- benchmark-level lambda studies
- per-dataset automatic lambda selection

A good first step may be:

- benchmark-driven evaluation of lambda grids
- then maybe one automatic selector such as L-curve or discrepancy principle

## Candidate 0.4 direction C: different regularization operators

The current second-difference penalty is reasonable, but it is only one choice.

### Alternatives worth testing

#### 1. First-difference penalty

Penalty:

`sum_i (c_(i+1) - c_i)^2`

This prefers gentle coefficient changes, but is usually weaker than a curvature
penalty.

#### 2. Curvature penalty on sampled `P(r)` instead of coefficients

Rather than penalizing coefficient curvature, evaluate `P(r)` on a fine grid
and penalize something like:

`integral (P''(r))^2 dr`

This may align more directly with the shape of the physical function.

#### 3. Mixed penalties

For example:

`lambda_1 ||L1 c||^2 + lambda_2 ||L2 c||^2`

This is more flexible, though more complex to tune.

#### 4. Endpoint-aware penalties

Since `P(0) = P(Dmax) = 0` and endpoint shape matters scientifically, it may be
worth adding penalties or parameterizations that explicitly reinforce endpoint
behavior.

## Candidate 0.4 direction D: different solvers

### Current solver

The present solver:

- forms normal equations
- adds `lambda L^T L`
- solves with Cholesky

This is compact and clear, but normal equations can worsen conditioning.

### Alternatives worth testing

#### 1. QR-based least squares

More numerically stable than normal equations, especially when the design
matrix becomes ill-conditioned.

#### 2. SVD-based solve

Useful for diagnosing rank deficiency and for understanding difficult cases.
Potentially slower, but valuable for benchmark comparisons.

#### 3. Augmented system solve

Solve the stacked system:

`[W A    ] c ~= [W y]`
`[sqrt(lambda) L]     [ 0 ]`

This can be preferable numerically to explicitly forming normal equations.

#### 4. Constrained solvers

For example:

- non-negative least squares on sampled `P(r)` or coefficients
- quadratic programming with smoothness and positivity

This is scientifically appealing, especially for the default non-negative
`P(r)` scope, but more complex than simple unconstrained Tikhonov.

## Candidate 0.4 direction E: positivity-aware approaches

This may or may not belong in `0.4`, but it is worth noting.

If the project remains focused on the default non-negative `P(r)` regime, then
it is reasonable to test:

- coefficient non-negativity when basis behavior supports it
- sampled-`P(r)` non-negativity constraints
- soft positivity penalties instead of hard constraints

This could improve pathological cases, especially with noise, but it should be
benchmarked carefully because it can also bias solutions.

## Candidate 0.4 direction F: robust losses for noisy or negative observed data

The current objective is purely quadratic. That means large residuals can exert
strong influence.

Potential alternatives:

- Huber loss
- pseudo-Huber loss
- Tukey-style robust losses

These may help on noisy observed curves, but they also complicate optimization
because the problem is no longer a simple linear least-squares solve.

This feels more like an advanced `0.4` or `0.5` topic unless it becomes
clearly necessary.

## Recommended experimental structure for 0.4

I would recommend the following order:

### 1. Keep the current baseline fixed

Baseline:

- raw `I(q)` target
- uncertainty weighting by `1 / sigma`
- second-difference penalty
- user-chosen `lambda`
- Cholesky solve of the normal equations

This remains the reference method.

### 2. Add one reweighting/scaling framework

Introduce a configurable scaling or weighting strategy such as:

- none
- `q`
- `q^2`
- `q^alpha`

and compare it systematically on:

- noiseless benchmark suites
- noisy benchmark suites

### 3. Add one lambda-study framework

Not necessarily automatic selection first, but at least:

- a structured lambda scan
- suite-level reporting of how metrics move with lambda

### 4. Then compare one alternative solver or penalty

Good first candidates:

- augmented-system solve instead of normal equations
- QR solve
- function-space curvature penalty

## My recommendation

If `0.4` should stay focused and high value, my recommendation is:

1. benchmarkable scaling or weighting strategies
2. benchmarkable lambda-selection studies
3. optionally one more stable solver formulation

Concretely, that could become:

- configurable `q`-dependent scaling or weighting
- lambda scan support and reporting
- one automatic lambda selector, probably L-curve or discrepancy principle
- optional augmented-system or QR-based solve as a comparison baseline

## Questions to settle before `PRD_0p4.md`

1. Do we want to treat `q^alpha I(q)` as a transformed fitting target, or do we
prefer reweighting the residuals while still fitting raw `I(q)`?

2. Is `0.4` mainly about:

- data scaling and balancing, or
- lambda/regularization selection, or
- solver redesign

3. Do we want `0.4` to include a positivity-constrained method, or should that
stay for later?

4. Do we want only benchmark-level comparison infrastructure in `0.4`, or also
new user-facing CLI support for these options?
