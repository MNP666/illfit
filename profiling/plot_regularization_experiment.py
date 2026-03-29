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

SHOW_PLOTS = os.environ.get("ILLFIT_SHOW_PLOTS", "0") == "1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Plot overview diagnostics for a regularization experiment run."
    )
    parser.add_argument(
        "--run-dir",
        type=Path,
        required=True,
        help="Path to a regularization run directory produced by `profile-regularization`.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Optional output image path. Default: <run-dir>/regularization_overview.png",
    )
    return parser.parse_args()


def finalize_figure(figure: plt.Figure, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    figure.tight_layout()
    figure.savefig(output_path, dpi=180, bbox_inches="tight")
    if SHOW_PLOTS or hasattr(sys, "ps1"):
        plt.show()
    plt.close(figure)


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def load_run_name(report_path: Path) -> str:
    with report_path.open("r", encoding="utf-8") as handle:
        return json.load(handle)["run_name"]


def to_float(value: str) -> float:
    if value == "":
        return float("nan")
    return float(value)


def main() -> int:
    args = parse_args()
    run_dir = args.run_dir.resolve()
    output_path = args.output.resolve() if args.output else run_dir / "regularization_overview.png"

    strategy_rows = read_csv(run_dir / "strategy_summary.csv")
    selected_rows = read_csv(run_dir / "selected_lambdas.csv")
    run_name = load_run_name(run_dir / "experiment_report.json")

    grouped: dict[str, list[dict[str, float]]] = defaultdict(list)
    for row in strategy_rows:
        grouped[row["weighting_strategy"]].append(
            {
                "lambda": float(row["lambda"]),
                "mean_pr_correlation": float(row["mean_pr_correlation"]),
                "mean_q_rmse": float(row["mean_q_rmse"]),
                "mean_data_misfit": float(row["mean_data_misfit"]),
                "mean_regularization_penalty": float(row["mean_regularization_penalty"]),
                "mean_gcv_score": float(row["mean_gcv_score"]),
                "mean_effective_degrees_of_freedom": float(
                    row["mean_effective_degrees_of_freedom"]
                ),
                "l_curve_curvature": to_float(row["l_curve_curvature"]),
            }
        )

    strategy_names = sorted(grouped)
    cmap = plt.get_cmap("Spectral")
    colors = {
        name: cmap(value)
        for name, value in zip(strategy_names, np.linspace(0.1, 0.9, len(strategy_names)))
    }

    selectors: dict[tuple[str, str], float] = {}
    for row in selected_rows:
        selectors[(row["weighting_strategy"], row["method"])] = float(row["selected_lambda"])

    figure, axes = plt.subplots(2, 3, figsize=(15, 9))
    for strategy in strategy_names:
        rows = sorted(grouped[strategy], key=lambda row: row["lambda"])
        lambdas = np.array([row["lambda"] for row in rows])
        pr_corr = np.array([row["mean_pr_correlation"] for row in rows])
        q_rmse = np.array([row["mean_q_rmse"] for row in rows])
        misfit = np.array([row["mean_data_misfit"] for row in rows])
        penalty = np.array([row["mean_regularization_penalty"] for row in rows])
        gcv = np.array([row["mean_gcv_score"] for row in rows])
        dof = np.array([row["mean_effective_degrees_of_freedom"] for row in rows])
        curvature = np.array([row["l_curve_curvature"] for row in rows])
        color = colors[strategy]

        axes[0, 0].plot(lambdas, pr_corr, marker="o", color=color, label=strategy)
        axes[0, 1].plot(lambdas, q_rmse, marker="o", color=color, label=strategy)
        axes[0, 2].plot(lambdas, dof, marker="o", color=color, label=strategy)
        axes[1, 0].plot(misfit, penalty, marker="o", color=color, label=strategy)
        axes[1, 1].plot(lambdas, gcv, marker="o", color=color, label=strategy)
        axes[1, 2].plot(lambdas, misfit, marker="o", color=color, label=strategy)

        l_curve_lambda = selectors.get((strategy, "l_curve"))
        gcv_lambda = selectors.get((strategy, "gcv"))
        if l_curve_lambda is not None:
            index = int(np.argmin(np.abs(lambdas - l_curve_lambda)))
            axes[1, 0].scatter(
                misfit[index],
                penalty[index],
                marker="*",
                s=180,
                color=color,
                edgecolor="k",
                linewidth=0.6,
                zorder=5,
            )
            axes[0, 0].scatter(
                lambdas[index],
                pr_corr[index],
                marker="*",
                s=160,
                color=color,
                edgecolor="k",
                linewidth=0.6,
                zorder=5,
            )
        if gcv_lambda is not None:
            index = int(np.argmin(np.abs(lambdas - gcv_lambda)))
            axes[1, 1].scatter(
                lambdas[index],
                gcv[index],
                marker="D",
                s=70,
                color=color,
                edgecolor="k",
                linewidth=0.5,
                zorder=5,
            )
            axes[0, 1].scatter(
                lambdas[index],
                q_rmse[index],
                marker="D",
                s=70,
                color=color,
                edgecolor="k",
                linewidth=0.5,
                zorder=5,
            )

    axes[0, 0].set_title("Mean P(r) correlation")
    axes[0, 1].set_title("Mean I(q) RMSE")
    axes[0, 2].set_title("Effective degrees of freedom")
    axes[1, 0].set_title("L-curve")
    axes[1, 1].set_title("Mean GCV score")
    axes[1, 2].set_title("Mean data misfit")

    for axis in [axes[0, 0], axes[0, 1], axes[0, 2], axes[1, 1], axes[1, 2]]:
        axis.set_xscale("log")
        axis.set_xlabel(r"$\lambda$")
        axis.grid(alpha=0.25, linewidth=0.6)

    axes[1, 0].set_xscale("log")
    axes[1, 0].set_yscale("log")
    axes[1, 0].set_xlabel("mean data misfit")
    axes[1, 0].set_ylabel("mean penalty")
    axes[1, 0].grid(alpha=0.25, linewidth=0.6)

    axes[0, 0].set_ylabel("correlation")
    axes[0, 0].set_ylim(0.0, 1.02)
    axes[0, 1].set_ylabel("RMSE")
    axes[0, 2].set_ylabel("trace(H)")
    axes[1, 1].set_ylabel("GCV")
    axes[1, 2].set_ylabel(r"$||W S (Ac-y)||^2$")

    handles, labels = axes[0, 0].get_legend_handles_labels()
    figure.legend(
        handles,
        labels,
        loc="upper center",
        bbox_to_anchor=(0.5, 0.985),
        ncol=max(1, len(strategy_names)),
        frameon=False,
    )
    figure.suptitle(f"Regularization diagnostics: {run_name}", fontsize=14, y=1.04)

    finalize_figure(figure, output_path)
    print(f"saved figure to: {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
