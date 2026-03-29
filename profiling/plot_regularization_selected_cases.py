#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import math
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
MPLCONFIGDIR = ROOT / "profiling" / ".matplotlib"
MPLCONFIGDIR.mkdir(parents=True, exist_ok=True)
os.environ.setdefault("MPLCONFIGDIR", str(MPLCONFIGDIR))

import matplotlib.pyplot as plt
import numpy as np

SHOW_PLOTS = os.environ.get("ILLFIT_SHOW_PLOTS", "0") == "1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Plot worst recovered selector-chosen cases from a regularization run."
    )
    parser.add_argument(
        "--run-dir",
        type=Path,
        required=True,
        help="Path to a regularization run directory produced by `profile-regularization`.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Optional directory for generated figures. Default: <run-dir>/selected_case_plots",
    )
    parser.add_argument(
        "--top-n",
        type=int,
        default=4,
        help="How many worst cases to show per selector combination. Default: 4",
    )
    return parser.parse_args()


def finalize_figure(figure: plt.Figure, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    figure.tight_layout()
    figure.savefig(output_path, dpi=180, bbox_inches="tight")
    if SHOW_PLOTS or hasattr(sys, "ps1"):
        plt.show()
    plt.close(figure)


def read_csv_rows(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def read_two_column_csv(path: Path) -> tuple[list[float], list[float]]:
    x_values: list[float] = []
    y_values: list[float] = []
    with path.open("r", encoding="utf-8") as handle:
        reader = csv.reader(handle)
        next(reader, None)
        for row in reader:
            if len(row) < 2:
                continue
            x_values.append(float(row[0]))
            y_values.append(float(row[1]))
    return x_values, y_values


def read_iq_case(case_dir: Path) -> tuple[list[float], list[float], list[float], list[float]]:
    q_truth, iq_truth = read_two_column_csv(case_dir / "iq_truth.csv")
    q_fit, iq_fit = read_two_column_csv(case_dir / "iq_recovered.csv")
    observed_rows = read_csv_rows(case_dir / "iq_observed.csv")
    q_observed = [float(row["q"]) for row in observed_rows]
    iq_observed = [float(row["i_of_q_observed"]) for row in observed_rows]
    if q_truth != q_fit or q_truth != q_observed:
        raise ValueError(f"inconsistent q-grid in {case_dir}")
    return q_truth, iq_truth, iq_observed, iq_fit


def selection_dirs(selected_root: Path) -> list[Path]:
    return sorted(path for path in selected_root.glob("*/*/lambda_*") if path.is_dir())


def plot_selection(selection_dir: Path, output_dir: Path, top_n: int) -> Path:
    method = selection_dir.parts[-3]
    strategy = selection_dir.parts[-2]
    lambda_name = selection_dir.parts[-1]
    rows = read_csv_rows(selection_dir / "selected_case_summary.csv")
    rows.sort(key=lambda row: float(row["pr_correlation"]))
    rows = rows[:top_n]

    column_count = len(rows)
    figure, axes = plt.subplots(2, column_count, figsize=(4.2 * column_count, 6.5), squeeze=False)
    color = plt.get_cmap("Spectral")(0.16)

    for column, row in enumerate(rows):
        case_dir = selection_dir / row["case_id"]
        r_truth, pr_truth = read_two_column_csv(case_dir / "pr_truth.csv")
        r_recovered, pr_recovered = read_two_column_csv(case_dir / "pr_recovered.csv")
        q_values, iq_truth, iq_observed, iq_fit = read_iq_case(case_dir)

        pr_ax = axes[0, column]
        iq_ax = axes[1, column]

        pr_ax.plot(r_truth, pr_truth, color="black", linewidth=2.0, label="truth")
        pr_ax.plot(r_recovered, pr_recovered, color=color, linewidth=2.0, label="recovered")
        pr_ax.set_title(
            f"{row['case_id']}\ncor={float(row['pr_correlation']):.3f}, rmse={float(row['pr_rmse']):.3f}",
            fontsize=10,
        )
        pr_ax.set_xlabel("r")
        pr_ax.set_ylabel("P(r)")
        pr_ax.grid(alpha=0.25, linewidth=0.6)

        iq_ax.scatter(
            q_values,
            iq_observed,
            s=20,
            color=color,
            edgecolor="k",
            linewidth=0.4,
            alpha=0.5,
            label="observed",
            zorder=3,
        )
        iq_ax.plot(q_values, iq_truth, color="0.4", linewidth=1.4, label="truth")
        iq_ax.plot(q_values, iq_fit, color=color, linewidth=2.0, label="fit")
        iq_ax.set_xlabel("q")
        iq_ax.set_ylabel("I(q)")
        iq_ax.grid(alpha=0.25, linewidth=0.6)

        positive_values = [value for value in iq_observed + iq_truth + iq_fit if value > 0.0]
        if positive_values:
            iq_ax.set_xscale("log")
            iq_ax.set_yscale("log")
        else:
            iq_ax.axhline(0.0, color="0.5", linewidth=1.0)

        if column == 0:
            pr_ax.legend(frameon=False, loc="best")
            iq_ax.legend(frameon=False, loc="best")

    figure.suptitle(
        f"Worst selected cases: {method} / {strategy} / {lambda_name}",
        fontsize=14,
        y=1.02,
    )
    output_path = output_dir / f"{method}_{strategy}_{lambda_name}.png"
    finalize_figure(figure, output_path)
    return output_path


def main() -> int:
    args = parse_args()
    run_dir = args.run_dir.resolve()
    selected_root = run_dir / "selected"
    if not selected_root.exists():
        raise FileNotFoundError(f"missing selected output directory: {selected_root}")

    output_dir = (
        args.output_dir.resolve()
        if args.output_dir is not None
        else run_dir / "selected_case_plots"
    )
    written = [
        plot_selection(selection_dir, output_dir, args.top_n)
        for selection_dir in selection_dirs(selected_root)
    ]
    print(f"plotted {len(written)} selector case panel(s)")
    print(f"saved figures under: {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
