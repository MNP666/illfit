#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
MPLCONFIGDIR = ROOT / "profiling" / ".matplotlib"
MPLCONFIGDIR.mkdir(parents=True, exist_ok=True)
os.environ.setdefault("MPLCONFIGDIR", str(MPLCONFIGDIR))

import matplotlib.pyplot as plt
import numpy as np

DEFAULT_SYNTHETIC_ROOT = ROOT / "data" / "synthetic"
DEFAULT_SUITE_DIR = ROOT / "data" / "synthetic" / "clamped_spline_seed42"
DEFAULT_OUTPUT_PATH = ROOT / "profiling" / "output" / "synthetic_suite_overview.png"
SHOW_PLOTS = os.environ.get("ILLFIT_SHOW_PLOTS", "0") == "1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Plot accepted synthetic P(r) and I(q) curves from exported synthetic suites."
    )
    parser.add_argument(
        "--synthetic-root",
        type=Path,
        default=DEFAULT_SYNTHETIC_ROOT,
        help=f"Root directory containing synthetic suite folders. Default: {DEFAULT_SYNTHETIC_ROOT}",
    )
    parser.add_argument(
        "--suite-dir",
        type=Path,
        default=None,
        help="Optional path to one exported synthetic suite. If omitted, all suite folders under --synthetic-root are plotted.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=DEFAULT_OUTPUT_PATH,
        help=f"Output image path. Default: {DEFAULT_OUTPUT_PATH}",
    )
    return parser.parse_args()


def read_curve_csv(path: Path) -> tuple[np.ndarray, np.ndarray]:
    x_values: list[float] = []
    y_values: list[float] = []
    with path.open("r", encoding="utf-8") as handle:
        reader = csv.reader(handle)
        next(reader, None)
        for row in reader:
            if len(row) != 2:
                continue
            x_values.append(float(row[0]))
            y_values.append(float(row[1]))
    return np.asarray(x_values, dtype=float), np.asarray(y_values, dtype=float)


def finalize_figure(figure: plt.Figure, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    figure.tight_layout()
    figure.savefig(output_path, dpi=170)
    if SHOW_PLOTS or hasattr(sys, "ps1"):
        plt.show()
    plt.close(figure)


def discover_suite_dirs(synthetic_root: Path, suite_dir: Path | None) -> list[Path]:
    if suite_dir is not None:
        return [suite_dir.resolve()]

    if not synthetic_root.exists():
        raise FileNotFoundError(f"synthetic root does not exist: {synthetic_root}")

    suite_dirs = sorted(path.resolve() for path in synthetic_root.iterdir() if path.is_dir())
    if not suite_dirs:
        raise ValueError(f"no synthetic suite directories found under: {synthetic_root}")
    return suite_dirs


def main() -> int:
    args = parse_args()
    suite_dirs = discover_suite_dirs(args.synthetic_root.resolve(), args.suite_dir)

    figure, axes = plt.subplots(
        2,
        len(suite_dirs),
        figsize=(6 * len(suite_dirs), 9),
        sharex=False,
        squeeze=False,
    )

    total_cases = 0

    for column_index, suite_dir in enumerate(suite_dirs):
        accepted_summary_path = suite_dir / "accepted_summary.json"
        if not accepted_summary_path.exists():
            raise FileNotFoundError(f"missing accepted summary file: {accepted_summary_path}")

        with accepted_summary_path.open("r", encoding="utf-8") as handle:
            accepted_rows = json.load(handle)

        if not accepted_rows:
            raise ValueError(f"no accepted cases listed in {accepted_summary_path}")

        colors = plt.get_cmap("Spectral")(np.linspace(0.05, 0.95, len(accepted_rows)))
        ax_pr = axes[0, column_index]
        ax_iq = axes[1, column_index]

        for color, row in zip(colors, accepted_rows, strict=True):
            case_id = row["candidate_id"]
            case_dir = suite_dir / case_id

            r_values, p_of_r = read_curve_csv(case_dir / "pr_truth.csv")
            q_values, i_of_q = read_curve_csv(case_dir / "iq_truth.csv")

            ax_pr.plot(r_values, p_of_r, color=color, linewidth=1.6, alpha=0.9)
            ax_iq.plot(q_values, i_of_q, color=color, linewidth=1.6, alpha=0.9)

        ax_pr.set_title(f"{suite_dir.name}")
        ax_pr.set_xlabel("r")
        if column_index == 0:
            ax_pr.set_ylabel("P(r)")
        ax_pr.axhline(0.0, color="black", linewidth=0.8)

        ax_iq.set_xscale("log")
        ax_iq.set_yscale("log")
        ax_iq.set_xlabel("q")
        if column_index == 0:
            ax_iq.set_ylabel("I(q)")

        total_cases += len(accepted_rows)

    finalize_figure(figure, args.output.resolve())
    print(f"plotted {len(suite_dirs)} suite(s)")
    print(f"plotted {total_cases} accepted case(s) in total")
    print(f"saved figure to: {args.output.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
