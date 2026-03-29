#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
MPLCONFIGDIR = ROOT / "profiling" / ".matplotlib"
MPLCONFIGDIR.mkdir(parents=True, exist_ok=True)
os.environ.setdefault("MPLCONFIGDIR", str(MPLCONFIGDIR))

import matplotlib.pyplot as plt
import numpy as np

DEFAULT_OUTPUT_ROOT = ROOT / "profiling" / "output"
DEFAULT_OUTPUT_PATH = ROOT / "profiling" / "output" / "benchmark_recovery_overview.png"
SHOW_PLOTS = os.environ.get("ILLFIT_SHOW_PLOTS", "0") == "1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Plot summary metrics from Rust-produced benchmark recovery outputs."
    )
    parser.add_argument(
        "--recovery-dir",
        type=Path,
        required=True,
        help="Path to a benchmark recovery output directory produced by `illfit benchmark-recover`.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=DEFAULT_OUTPUT_PATH,
        help=f"Output image path. Default: {DEFAULT_OUTPUT_PATH}",
    )
    return parser.parse_args()


def finalize_figure(figure: plt.Figure, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    figure.tight_layout()
    figure.savefig(output_path, dpi=170)
    if SHOW_PLOTS or hasattr(sys, "ps1"):
        plt.show()
    plt.close(figure)


def load_suite_summary(path: Path) -> list[dict[str, float | str]]:
    rows: list[dict[str, float | str]] = []
    with path.open("r", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            rows.append(
                {
                    "case_id": row["case_id"],
                    "pr_rmse": float(row["pr_rmse"]),
                    "pr_normalized_rmse": float(row["pr_normalized_rmse"]),
                    "pr_correlation": float(row["pr_correlation"]),
                    "pr_integrated_absolute_error": float(row["pr_integrated_absolute_error"]),
                    "rg_error": float(row["rg_error"]),
                    "i_zero_error": float(row["i_zero_error"]),
                    "q_rmse": float(row["q_rmse"]),
                    "q_normalized_rmse": float(row["q_normalized_rmse"]),
                }
            )
    return rows


def main() -> int:
    args = parse_args()
    recovery_dir = args.recovery_dir.resolve()
    summary_path = recovery_dir / "benchmark_suite_summary.csv"
    if not summary_path.exists():
        raise FileNotFoundError(f"missing benchmark suite summary: {summary_path}")

    rows = load_suite_summary(summary_path)
    if not rows:
        raise ValueError(f"no rows found in {summary_path}")

    indices = np.arange(len(rows))
    colors = plt.get_cmap("Spectral")(np.linspace(0.05, 0.95, len(rows)))

    figure, axes = plt.subplots(2, 2, figsize=(12, 8))

    axes[0, 0].bar(indices, [row["pr_rmse"] for row in rows], color=colors)
    axes[0, 0].set_title("P(r) RMSE")
    axes[0, 0].set_ylabel("RMSE")

    axes[0, 1].bar(indices, [row["q_rmse"] for row in rows], color=colors)
    axes[0, 1].set_title("I(q) RMSE")
    axes[0, 1].set_ylabel("RMSE")

    axes[1, 0].bar(indices, [row["pr_correlation"] for row in rows], color=colors)
    axes[1, 0].set_title("P(r) correlation")
    axes[1, 0].set_ylabel("correlation")
    axes[1, 0].set_ylim(0.0, 1.05)

    axes[1, 1].bar(indices, [row["rg_error"] for row in rows], color=colors)
    axes[1, 1].set_title("Rg error")
    axes[1, 1].set_ylabel("error")

    case_labels = [str(row["case_id"]) for row in rows]
    for axis in axes.flat:
        axis.set_xlabel("case")
        axis.set_xticks(indices)
        axis.set_xticklabels(case_labels, rotation=90, fontsize=7)

    finalize_figure(figure, args.output.resolve())
    print(f"plotted {len(rows)} recovered case(s)")
    print(f"saved figure to: {args.output.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
