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
