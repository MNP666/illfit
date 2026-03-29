#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
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

DEFAULT_OUTPUT_DIR = ROOT / "profiling" / "output" / "noisy_benchmark_case_fits"
SHOW_PLOTS = os.environ.get("ILLFIT_SHOW_PLOTS", "0") == "1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Plot noisy observed I(q) and recovered P(r) for noisy benchmark recovery outputs."
    )
    parser.add_argument(
        "--recovery-dir",
        type=Path,
        required=True,
        help="Path to a benchmark recovery output directory produced by `illfit benchmark-recover-noisy`.",
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


def read_two_column_csv(path: Path) -> tuple[np.ndarray, np.ndarray]:
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
    return np.asarray(x_values, dtype=float), np.asarray(y_values, dtype=float)


def read_iq_observed_csv(path: Path) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    q_values: list[float] = []
    observed_values: list[float] = []
    sigma_values: list[float] = []
    with path.open("r", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            q_values.append(float(row["q"]))
            observed_values.append(float(row["i_of_q_observed"]))
            sigma_values.append(float(row["sigma"]))
    return (
        np.asarray(q_values, dtype=float),
        np.asarray(observed_values, dtype=float),
        np.asarray(sigma_values, dtype=float),
    )


def read_iq_recovered_csv(path: Path) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    q_values: list[float] = []
    truth_values: list[float] = []
    recovered_values: list[float] = []
    with path.open("r", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            q_values.append(float(row["q"]))
            truth_values.append(float(row["i_of_q_truth"]))
            recovered_values.append(float(row["i_of_q_recovered"]))
    return (
        np.asarray(q_values, dtype=float),
        np.asarray(truth_values, dtype=float),
        np.asarray(recovered_values, dtype=float),
    )


def read_noise_metadata(path: Path) -> dict[str, float | int | str]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def discover_case_variants(
    recovery_dir: Path,
) -> dict[str, list[dict[str, Path | float]]]:
    grouped: dict[str, list[dict[str, Path | float]]] = defaultdict(list)

    for noise_dir in sorted(path for path in recovery_dir.iterdir() if path.is_dir()):
        if not noise_dir.name.startswith("noise_"):
            continue

        for case_dir in sorted(path for path in noise_dir.iterdir() if path.is_dir()):
            metadata = read_noise_metadata(case_dir / "noise_metadata.json")
            grouped[case_dir.name].append(
                {
                    "case_dir": case_dir,
                    "noise_level": float(metadata["noise_level"]),
                    "negative_value_fraction": float(metadata["negative_value_fraction"]),
                }
            )

    for variants in grouped.values():
        variants.sort(key=lambda variant: float(variant["noise_level"]))

    return dict(grouped)


def plot_case_variants(
    case_id: str,
    variants: list[dict[str, Path | float]],
    output_dir: Path,
) -> Path:
    ncols = len(variants)
    figure, axes = plt.subplots(
        2,
        ncols,
        figsize=(4.8 * ncols, 7.2),
        squeeze=False,
        sharex="row",
    )
    colors = plt.get_cmap("Spectral")(np.linspace(0.08, 0.92, ncols))

    for column, (variant, color) in enumerate(zip(variants, colors, strict=True)):
        case_dir = Path(variant["case_dir"])
        noise_level = float(variant["noise_level"])
        negative_fraction = float(variant["negative_value_fraction"])

        r_truth, pr_truth = read_two_column_csv(case_dir / "pr_truth.csv")
        r_recovered, pr_recovered = read_two_column_csv(case_dir / "pr_recovered.csv")
        q_observed, iq_observed, sigma = read_iq_observed_csv(case_dir / "iq_observed.csv")
        q_truth, iq_truth, iq_recovered = read_iq_recovered_csv(case_dir / "iq_recovered.csv")

        pr_axis = axes[0, column]
        iq_axis = axes[1, column]

        pr_axis.plot(r_truth, pr_truth, color="black", linewidth=2.0, label="truth")
        pr_axis.plot(
            r_recovered,
            pr_recovered,
            color=color,
            linewidth=2.0,
            label="recovered",
        )
        pr_axis.set_title(
            f"{case_id}\nnoise={noise_level:.3g}, neg={negative_fraction:.3f}"
        )
        pr_axis.set_xlabel("r")
        pr_axis.set_ylabel("P(r)")
        pr_axis.grid(alpha=0.25, linewidth=0.6)

        iq_axis.scatter(
            q_observed,
            iq_observed,
            s=20,
            color=color,
            edgecolor="k",
            linewidth=0.4,
            alpha=0.5,
            label="observed",
            zorder=3,
        )
        iq_axis.plot(q_truth, iq_truth, color="0.45", linewidth=1.4, label="truth")
        iq_axis.plot(
            q_truth,
            iq_recovered,
            color=color,
            linewidth=2.0,
            label="fit",
        )
        iq_axis.set_xscale("log")
        iq_axis.set_yscale("log")
        iq_axis.set_xlabel("q")
        iq_axis.set_ylabel("I(q)")
        iq_axis.grid(alpha=0.25, linewidth=0.6)

        positive_observed = iq_observed[iq_observed > 0.0]
        if positive_observed.size:
            y_min = min(positive_observed.min(), iq_truth[iq_truth > 0.0].min()) * 0.8
            y_max = max(iq_observed.max(), iq_recovered.max(), iq_truth.max()) * 1.2
            iq_axis.set_ylim(y_min, y_max)

        if column == 0:
            pr_axis.legend(frameon=False, loc="best")
            iq_axis.legend(frameon=False, loc="best")

    figure.suptitle(f"Noisy benchmark recovery: {case_id}", fontsize=14)
    output_path = output_dir / f"{case_id}.png"
    finalize_figure(figure, output_path)
    return output_path


def main() -> int:
    args = parse_args()
    recovery_dir = args.recovery_dir.resolve()
    output_dir = args.output_dir.resolve()
    if not recovery_dir.exists():
        raise FileNotFoundError(f"missing recovery directory: {recovery_dir}")

    grouped = discover_case_variants(recovery_dir)
    if not grouped:
        raise ValueError(f"no noisy case variants found in {recovery_dir}")

    selected_case_ids = args.case_id or sorted(grouped)
    written_paths: list[Path] = []

    for case_id in selected_case_ids:
        if case_id not in grouped:
            raise KeyError(f"case id `{case_id}` not found in {recovery_dir}")
        written_paths.append(plot_case_variants(case_id, grouped[case_id], output_dir))

    print(f"plotted {len(written_paths)} noisy case figure(s)")
    print(f"saved figures under: {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
