#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import os
from collections import defaultdict
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
MPLCONFIGDIR = ROOT / "profiling" / ".matplotlib"
MPLCONFIGDIR.mkdir(parents=True, exist_ok=True)
os.environ.setdefault("MPLCONFIGDIR", str(MPLCONFIGDIR))

import matplotlib.pyplot as plt
import numpy as np

DEFAULT_OUTPUT_PATH = (
    ROOT / "profiling" / "output" / "noisy_benchmark_recovery_overview.png"
)
SHOW_PLOTS = os.environ.get("ILLFIT_SHOW_PLOTS", "0") == "1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Plot degradation trends from noisy benchmark recovery outputs."
    )
    parser.add_argument(
        "--recovery-dir",
        type=Path,
        required=True,
        help="Path to a benchmark recovery output directory produced by `illfit benchmark-recover-noisy`.",
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
                    "noise_level": float(row["noise_level"]),
                    "negative_value_fraction": float(row["negative_value_fraction"]),
                    "pr_rmse": float(row["pr_rmse"]),
                    "pr_normalized_rmse": float(row["pr_normalized_rmse"]),
                    "pr_correlation": float(row["pr_correlation"]),
                    "q_rmse": float(row["q_rmse"]),
                    "q_normalized_rmse": float(row["q_normalized_rmse"]),
                }
            )
    return rows


def group_rows_by_case(
    rows: list[dict[str, float | str]],
) -> dict[str, list[dict[str, float | str]]]:
    grouped: dict[str, list[dict[str, float | str]]] = defaultdict(list)
    for row in rows:
        grouped[str(row["case_id"])].append(row)

    for case_rows in grouped.values():
        case_rows.sort(key=lambda row: float(row["noise_level"]))

    return dict(grouped)


def add_case_lines(
    axis: plt.Axes,
    grouped_rows: dict[str, list[dict[str, float | str]]],
    key: str,
    colors: np.ndarray,
) -> None:
    for color, (case_id, case_rows) in zip(colors, grouped_rows.items(), strict=True):
        x = [float(row["noise_level"]) for row in case_rows]
        y = [float(row[key]) for row in case_rows]
        axis.plot(
            x,
            y,
            marker="o",
            linewidth=1.5,
            markersize=4,
            color=color,
            alpha=0.9,
            label=case_id,
        )


def add_mean_trend(
    axis: plt.Axes,
    rows: list[dict[str, float | str]],
    key: str,
) -> None:
    unique_levels = sorted({float(row["noise_level"]) for row in rows})
    means = []
    for level in unique_levels:
        level_values = [float(row[key]) for row in rows if float(row["noise_level"]) == level]
        means.append(float(np.mean(level_values)))

    axis.plot(
        unique_levels,
        means,
        color="black",
        linewidth=2.5,
        marker="s",
        markersize=5,
        label="mean",
        zorder=10,
    )


def main() -> int:
    args = parse_args()
    recovery_dir = args.recovery_dir.resolve()
    summary_path = recovery_dir / "benchmark_suite_summary.csv"
    if not summary_path.exists():
        raise FileNotFoundError(f"missing benchmark suite summary: {summary_path}")

    rows = load_suite_summary(summary_path)
    if not rows:
        raise ValueError(f"no rows found in {summary_path}")

    grouped_rows = group_rows_by_case(rows)
    colors = plt.get_cmap("Spectral")(np.linspace(0.05, 0.95, len(grouped_rows)))

    figure, axes = plt.subplots(2, 2, figsize=(12, 8), sharex=True)

    add_case_lines(axes[0, 0], grouped_rows, "pr_rmse", colors)
    add_mean_trend(axes[0, 0], rows, "pr_rmse")
    axes[0, 0].set_title("P(r) RMSE")
    axes[0, 0].set_ylabel("RMSE")

    add_case_lines(axes[0, 1], grouped_rows, "q_rmse", colors)
    add_mean_trend(axes[0, 1], rows, "q_rmse")
    axes[0, 1].set_title("I(q) RMSE")
    axes[0, 1].set_ylabel("RMSE")

    add_case_lines(axes[1, 0], grouped_rows, "pr_correlation", colors)
    add_mean_trend(axes[1, 0], rows, "pr_correlation")
    axes[1, 0].set_title("P(r) Correlation")
    axes[1, 0].set_ylabel("correlation")
    axes[1, 0].set_ylim(0.0, 1.05)

    add_case_lines(axes[1, 1], grouped_rows, "negative_value_fraction", colors)
    add_mean_trend(axes[1, 1], rows, "negative_value_fraction")
    axes[1, 1].set_title("Negative I(q) Fraction")
    axes[1, 1].set_ylabel("fraction")

    for axis in axes.flat:
        axis.set_xscale("log")
        axis.set_xlabel("noise level")
        axis.grid(alpha=0.25, linewidth=0.6)

    handles, labels = axes[0, 0].get_legend_handles_labels()
    figure.legend(
        handles,
        labels,
        loc="center left",
        bbox_to_anchor=(1.02, 0.5),
        frameon=False,
        title="case",
    )

    finalize_figure(figure, args.output.resolve())
    print(f"plotted {len(rows)} noisy recovered variant(s)")
    print(f"saved figure to: {args.output.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
