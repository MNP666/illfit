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

DEFAULT_OUTPUT_DIR = ROOT / "profiling" / "output" / "benchmark_case_fits"
SHOW_PLOTS = os.environ.get("ILLFIT_SHOW_PLOTS", "0") == "1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Plot truth and recovered curves for noiseless benchmark recovery outputs."
    )
    parser.add_argument(
        "--recovery-dir",
        type=Path,
        required=True,
        help="Path to a benchmark recovery output directory produced by `illfit benchmark-recover`.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUTPUT_DIR,
        help=f"Directory for generated figures. Default: {DEFAULT_OUTPUT_DIR}",
    )
    parser.add_argument(
        "--case-id",
        action="append",
        default=[],
        help="Optional case id to restrict plotting. Can be passed multiple times.",
    )
    return parser.parse_args()


def finalize_figure(figure: plt.Figure, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    figure.tight_layout()
    figure.savefig(output_path, dpi=170, bbox_inches="tight")
    if SHOW_PLOTS or hasattr(sys, "ps1"):
        plt.show()
    plt.close(figure)


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


def read_iq_recovered_csv(path: Path) -> tuple[list[float], list[float], list[float]]:
    q_values: list[float] = []
    truth_values: list[float] = []
    recovered_values: list[float] = []
    with path.open("r", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            q_values.append(float(row["q"]))
            truth_values.append(float(row["i_of_q_truth"]))
            recovered_values.append(float(row["i_of_q_recovered"]))
    return q_values, truth_values, recovered_values


def discover_case_dirs(recovery_dir: Path) -> list[Path]:
    return sorted(
        path
        for path in recovery_dir.iterdir()
        if path.is_dir() and path.name.startswith("case_")
    )


def plot_case(case_dir: Path, output_dir: Path) -> Path:
    case_id = case_dir.name
    r_truth, pr_truth = read_two_column_csv(case_dir / "pr_truth.csv")
    r_recovered, pr_recovered = read_two_column_csv(case_dir / "pr_recovered.csv")
    q_values, iq_truth, iq_recovered = read_iq_recovered_csv(case_dir / "iq_recovered.csv")

    figure, axes = plt.subplots(2, 1, figsize=(7.5, 7.0))
    color = plt.get_cmap("Spectral")(0.18)

    axes[0].plot(r_truth, pr_truth, color="black", linewidth=2.0, label="truth")
    axes[0].plot(r_recovered, pr_recovered, color=color, linewidth=2.0, label="recovered")
    axes[0].set_title(f"Noiseless benchmark recovery: {case_id}")
    axes[0].set_xlabel("r")
    axes[0].set_ylabel("P(r)")
    axes[0].grid(alpha=0.25, linewidth=0.6)
    axes[0].legend(frameon=False, loc="best")

    axes[1].scatter(
        q_values,
        iq_truth,
        s=22,
        color=color,
        edgecolor="k",
        linewidth=0.4,
        alpha=0.5,
        label="truth",
        zorder=3,
    )
    axes[1].plot(q_values, iq_truth, color="0.45", linewidth=1.4, alpha=0.8)
    axes[1].plot(q_values, iq_recovered, color=color, linewidth=2.0, label="fit")
    axes[1].set_xscale("log")
    axes[1].set_yscale("log")
    axes[1].set_xlabel("q")
    axes[1].set_ylabel("I(q)")
    axes[1].grid(alpha=0.25, linewidth=0.6)
    axes[1].legend(frameon=False, loc="best")

    output_path = output_dir / f"{case_id}.png"
    finalize_figure(figure, output_path)
    return output_path


def main() -> int:
    args = parse_args()
    recovery_dir = args.recovery_dir.resolve()
    output_dir = args.output_dir.resolve()
    if not recovery_dir.exists():
        raise FileNotFoundError(f"missing recovery directory: {recovery_dir}")

    case_dirs = discover_case_dirs(recovery_dir)
    if not case_dirs:
        raise ValueError(f"no benchmark case directories found in {recovery_dir}")

    if args.case_id:
        selected = []
        for case_id in args.case_id:
            case_dir = recovery_dir / case_id
            if not case_dir.exists():
                raise KeyError(f"case id `{case_id}` not found in {recovery_dir}")
            selected.append(case_dir)
        case_dirs = selected

    written_paths = [plot_case(case_dir, output_dir) for case_dir in case_dirs]
    print(f"plotted {len(written_paths)} noiseless benchmark case figure(s)")
    print(f"saved figures under: {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
