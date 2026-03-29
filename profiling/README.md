# Profiling

This folder is for lightweight tooling that helps us evaluate the Rust CLI
against example and reference datasets while the project is still evolving.

The main script in this folder is intended for iterative development:

- discover `.dat` files in `data/examples/`
- match them to reference `.out` files in `data/reference/`
- run the Rust CLI
- compare generated `P(r)` curves to the reference `P(r)` block
- write plots and summary tables

The script reads its parameters from
[`profiling/config.toml`](/Users/air/Documents/illfit/profiling/config.toml), so
the normal workflow is:

```bash
python3 profiling/compare_reference_pr.py
```

It also supports small parameter sweeps so we can quickly test whether
differences are driven by:

- basis size
- regularization strength
- low-`q` front truncation

Example `config.toml` edit:

```toml
[profile]
stems = ["SASDYU3"]
score = "rmse"
timeout_seconds = 8.0

[sweep]
basis_sizes = [12, 16, 20]
lambda_values = [1e-4, 1e-3, 1e-2]
drop_first_values = [0, 5, 10, 15]
```

The script keeps the best trial per dataset in `summary.csv` / `summary.json`
and writes every attempted trial to `sweep_trials.csv`.

This is intentionally separate from the Rust CLI itself. Using Python for
comparison plots is the fastest way to inspect progress while the scientific
implementation is still changing.

## Synthetic benchmark exploration

[`explore_synthetic_pr.py`](/Users/air/Documents/illfit/profiling/explore_synthetic_pr.py)
is a lightweight interactive script for exploring deterministic `P(r)` family
generation ideas for the planned `0.3` benchmark machinery.

It is written with `# %%` cells so it works nicely as:

- a regular Python script
- a VS Code interactive script
- an editor-driven notebook-like workflow without committing to a full notebook

The script:

- builds a cubic B-spline basis in Python
- generates a few deterministic coefficient-family strategies
- screens cases with simple `P(r)` and `I(q)` acceptance rules, including
  `P(0) ~= 0` and `P(Dmax) ~= 0`
- plots accepted and rejected examples

Run it with:

```bash
python3 profiling/explore_synthetic_pr.py
```

Plots are written under `profiling/output/synthetic_exploration/`.

If you want the script to also open figures interactively during a normal run,
set:

```bash
ILLFIT_SHOW_PLOTS=1 python3 profiling/explore_synthetic_pr.py
```

[`explore_gaussian_pr.py`](/Users/air/Documents/illfit/profiling/explore_gaussian_pr.py)
is a companion exploration script that generates smooth truth cases from
overlapping Gaussians and tapered Gaussian-like families instead of spline
coefficients.

It is useful when we want benchmark truth cases that are smooth and realistic
without being "too on-model" for a spline-based recovery method.

Run it with:

```bash
python3 profiling/explore_gaussian_pr.py
```

Plots are written under `profiling/output/gaussian_exploration/`, and accepted
`I(q)` curves are plotted on log-log axes.

## Structured benchmark export

[`export_synthetic_benchmarks.py`](/Users/air/Documents/illfit/profiling/export_synthetic_benchmarks.py)
is a more structured tool for exporting deterministic synthetic benchmark
assets that Rust can later parse and consume.

It currently uses clamped cubic splines with:

- `P(0) = 0`
- `P(Dmax) = 0`
- zero first derivative at both endpoints

The exporter reads its settings from
[`profiling/benchmark_export.toml`](/Users/air/Documents/illfit/profiling/benchmark_export.toml)
and writes an exploratory suite under `data/synthetic/`.

Run it with:

```bash
python3 profiling/export_synthetic_benchmarks.py
```

Each accepted case is written with:

- `pr_truth.csv`
- `iq_truth.csv`
- `metadata.json`

and the suite root also gets accepted/rejected summaries plus `suite_summary.json`.

Once benchmark recovery outputs have been written by the Rust CLI, you can plot
their suite-level metrics with:

```bash
python3 profiling/plot_benchmark_recovery.py \
  --recovery-dir /path/to/benchmark_recovery_output
```

## Synthetic suite overview plot

[`plot_synthetic_suite.py`](/Users/air/Documents/illfit/profiling/plot_synthetic_suite.py)
plots exported synthetic suites in a two-row overview:

- one column per suite folder under `data/synthetic/` by default
- `P(r)` curves on the top row
- `I(q)` curves on the bottom row

The script uses the `Spectral` colormap and plots `I(q)` on log-log axes.

Run it with:

```bash
python3 profiling/plot_synthetic_suite.py
```

or target one suite explicitly with:

```bash
python3 profiling/plot_synthetic_suite.py \
  --suite-dir data/synthetic/clamped_spline_seed42
```

The reviewed committed regression suite currently lives at
[`data/regression/clamped_spline`](/Users/air/Documents/illfit/data/regression/clamped_spline).
