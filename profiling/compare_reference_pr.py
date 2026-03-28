#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import math
import os
import shutil
import subprocess
import sys
import tomllib
from dataclasses import dataclass, asdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MPLCONFIGDIR = ROOT / "profiling" / ".matplotlib"
MPLCONFIGDIR.mkdir(parents=True, exist_ok=True)
os.environ.setdefault("MPLCONFIGDIR", str(MPLCONFIGDIR))

import matplotlib.pyplot as plt
import numpy as np

EXAMPLES_DIR = ROOT / "data" / "examples"
REFERENCE_DIR = ROOT / "data" / "reference"
DEFAULT_OUTPUT_DIR = ROOT / "profiling" / "output" / "latest"
BINARY_PATH = ROOT / "target" / "debug" / "illfit"
DEFAULT_CONFIG_PATH = ROOT / "profiling" / "config.toml"


@dataclass
class ReferencePr:
    stem: str
    r: np.ndarray
    p_of_r: np.ndarray
    error: np.ndarray


@dataclass
class ComparisonResult:
    stem: str
    status: str
    data_path: str
    reference_path: str
    output_dir: str
    dmax: float | None
    pr_sample_points: int | None
    basis_size: int | None
    integration_intervals: int | None
    lambda_value: float | None
    drop_first: int | None
    rmse: float | None
    normalized_rmse: float | None
    max_abs_error: float | None
    correlation: float | None
    message: str
    sweep_trial_count: int | None = None


@dataclass(frozen=True)
class SweepConfig:
    basis_size: int
    lambda_value: float
    drop_first: int


@dataclass(frozen=True)
class ScriptConfig:
    stems: list[str] | None
    output_dir: Path
    integration_intervals: int
    basis_sizes: list[int]
    lambda_values: list[float]
    drop_first_values: list[int]
    score: str
    timeout_seconds: float
    keep_existing_output: bool


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the Rust CLI on example SAXS files and compare generated P(r) curves to reference .out files."
    )
    parser.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG_PATH,
        help=f"TOML config file describing the profiling run. Default: {DEFAULT_CONFIG_PATH}",
    )
    return parser.parse_args()


def load_config(path: Path) -> ScriptConfig:
    with path.open("rb") as handle:
        raw = tomllib.load(handle)

    profile = raw.get("profile", {})
    sweep = raw.get("sweep", {})

    stems = profile.get("stems")
    if stems is not None and not isinstance(stems, list):
        raise ValueError("`profile.stems` must be a list when provided")

    score = profile.get("score", "rmse")
    if score not in {"rmse", "normalized-rmse", "correlation"}:
        raise ValueError("`profile.score` must be one of: rmse, normalized-rmse, correlation")

    output_dir = ROOT / profile.get("output_dir", str(DEFAULT_OUTPUT_DIR.relative_to(ROOT)))

    return ScriptConfig(
        stems=stems,
        output_dir=output_dir,
        integration_intervals=int(profile.get("integration_intervals", 1200)),
        basis_sizes=[int(value) for value in sweep.get("basis_sizes", [16])],
        lambda_values=[float(value) for value in sweep.get("lambda_values", [1.0e-2])],
        drop_first_values=[int(value) for value in sweep.get("drop_first_values", [0])],
        score=score,
        timeout_seconds=float(profile.get("timeout_seconds", 30.0)),
        keep_existing_output=bool(profile.get("keep_existing_output", False)),
    )


def discover_matching_stems(selected_stems: list[str] | None) -> list[str]:
    example_stems = {path.stem for path in EXAMPLES_DIR.glob("*.dat")}
    reference_stems = {path.stem for path in REFERENCE_DIR.glob("*.out")}
    stems = sorted(example_stems & reference_stems)

    if selected_stems:
        selected = set(selected_stems)
        stems = [stem for stem in stems if stem in selected]

    return stems


def parse_reference_pr(path: Path) -> ReferencePr:
    seen_header = False
    rows: list[tuple[float, float, float]] = []
    previous_r: float | None = None

    contents = path.read_text(encoding="utf-8", errors="replace")
    normalized_lines = contents.replace("\r\n", "\n").replace("\r", "\n").split("\n")

    for raw_line in normalized_lines:
        line = raw_line.strip()
        if not seen_header:
            if line.split() == ["R", "P(R)", "ERROR"]:
                seen_header = True
            continue

        if not line:
            continue

        fields = line.split()
        if len(fields) != 3:
            if rows:
                break
            continue

        try:
            row = tuple(float(field) for field in fields)
        except ValueError:
            if rows:
                break
            continue

        r = row[0]
        if previous_r is not None and r <= previous_r:
            break

        rows.append(row)
        previous_r = r

    if not seen_header:
        raise ValueError(f"could not find `R P(R) ERROR` block in {path}")
    if not rows:
        raise ValueError(f"no reference P(r) rows found in {path}")

    array = np.asarray(rows, dtype=float)
    return ReferencePr(
        stem=path.stem,
        r=array[:, 0],
        p_of_r=array[:, 1],
        error=array[:, 2],
    )


def sweep_configs(config: ScriptConfig) -> list[SweepConfig]:
    configs: list[SweepConfig] = []
    for basis_size in config.basis_sizes:
        for lambda_value in config.lambda_values:
            for drop_first in config.drop_first_values:
                configs.append(
                    SweepConfig(
                        basis_size=basis_size,
                        lambda_value=lambda_value,
                        drop_first=drop_first,
                    )
                )
    return configs


def build_rust_binary(timeout_seconds: float) -> None:
    run_command(["cargo", "build"], cwd=ROOT, timeout_seconds=timeout_seconds)


def run_cli_fit(
    data_path: Path,
    output_dir: Path,
    dmax: float,
    pr_sample_points: int,
    basis_size: int,
    integration_intervals: int,
    lambda_value: float,
    drop_first: int,
    timeout_seconds: float,
) -> None:
    command = [
        str(BINARY_PATH),
        "fit",
        "--data",
        str(data_path),
        "--dmax",
        f"{dmax}",
        "--basis-size",
        str(basis_size),
        "--integration-intervals",
        str(integration_intervals),
        "--lambda",
        f"{lambda_value}",
        "--pr-sample-points",
        str(pr_sample_points),
        "--output-dir",
        str(output_dir),
    ]

    if drop_first:
        command.extend(["--drop-first", str(drop_first)])

    run_command(command, cwd=ROOT, timeout_seconds=timeout_seconds)


def read_generated_pr(path: Path) -> tuple[np.ndarray, np.ndarray]:
    data = np.genfromtxt(path, delimiter=",", names=True)
    return np.asarray(data["r"], dtype=float), np.asarray(data["p_of_r"], dtype=float)


def compare_curves(reference: ReferencePr, generated_r: np.ndarray, generated_p: np.ndarray) -> dict[str, float]:
    if generated_r.shape == reference.r.shape and np.allclose(generated_r, reference.r):
        aligned_generated = generated_p
    else:
        aligned_generated = np.interp(reference.r, generated_r, generated_p)

    difference = aligned_generated - reference.p_of_r
    rmse = float(np.sqrt(np.mean(difference**2)))
    value_range = float(reference.p_of_r.max() - reference.p_of_r.min())
    normalized_rmse = rmse / value_range if value_range > 0.0 else math.nan
    max_abs_error = float(np.max(np.abs(difference)))
    correlation = float(np.corrcoef(reference.p_of_r, aligned_generated)[0, 1])

    return {
        "rmse": rmse,
        "normalized_rmse": normalized_rmse,
        "max_abs_error": max_abs_error,
        "correlation": correlation,
        "aligned_generated": aligned_generated,
        "difference": difference,
    }


def write_plot(
    stem: str,
    reference: ReferencePr,
    aligned_generated: np.ndarray,
    difference: np.ndarray,
    output_path: Path,
) -> None:
    figure, axes = plt.subplots(2, 1, figsize=(8, 7), sharex=True)

    axes[0].plot(reference.r, reference.p_of_r, label="Reference .out", linewidth=2)
    axes[0].plot(reference.r, aligned_generated, label="Rust CLI", linewidth=2, linestyle="--")
    axes[0].set_ylabel("P(r)")
    axes[0].set_title(stem)
    axes[0].legend()

    axes[1].plot(reference.r, difference, color="tab:red", linewidth=1.5)
    axes[1].axhline(0.0, color="black", linewidth=0.8)
    axes[1].set_xlabel("r")
    axes[1].set_ylabel("Delta P(r)")

    figure.tight_layout()
    figure.savefig(output_path, dpi=160)
    plt.close(figure)


def score_value(result: ComparisonResult, score_name: str) -> float:
    if result.status != "ok":
        return math.inf
    if score_name == "rmse":
        return result.rmse if result.rmse is not None else math.inf
    if score_name == "normalized-rmse":
        return result.normalized_rmse if result.normalized_rmse is not None else math.inf
    if score_name == "correlation":
        correlation = result.correlation if result.correlation is not None else -math.inf
        return -correlation
    raise ValueError(f"unsupported score name: {score_name}")


def run_command(command: list[str], cwd: Path, timeout_seconds: float) -> None:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            text=True,
            capture_output=True,
            timeout=timeout_seconds,
        )
    except subprocess.TimeoutExpired as error:
        raise RuntimeError(
            f"command timed out after {timeout_seconds:g} seconds: {' '.join(command)}"
        ) from error
    if result.returncode != 0:
        raise RuntimeError(
            f"command failed: {' '.join(command)}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )


def prepare_output_dir(path: Path, keep_existing_output: bool) -> None:
    if path.exists() and not keep_existing_output:
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def write_summary_files(output_dir: Path, results: list[ComparisonResult]) -> None:
    csv_path = output_dir / "summary.csv"
    json_path = output_dir / "summary.json"

    fieldnames = list(asdict(results[0]).keys()) if results else [
        "stem",
        "status",
        "message",
    ]

    with csv_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for result in results:
            writer.writerow(asdict(result))

    with json_path.open("w", encoding="utf-8") as handle:
        json.dump([asdict(result) for result in results], handle, indent=2)


def write_per_stem_sweep_summary(output_dir: Path, results: list[ComparisonResult]) -> None:
    if not results:
        return

    csv_path = output_dir / "sweep_trials.csv"
    fieldnames = list(asdict(results[0]).keys())
    with csv_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for result in results:
            writer.writerow(asdict(result))


def print_summary(results: list[ComparisonResult]) -> None:
    if not results:
        print("No matching .dat/.out pairs were found.")
        return

    print(
        f"{'stem':<12} {'status':<8} {'rmse':>12} {'norm_rmse':>12} {'corr':>10} {'message'}"
    )
    for result in results:
        rmse = f"{result.rmse:.4e}" if result.rmse is not None else "-"
        nrmse = f"{result.normalized_rmse:.4e}" if result.normalized_rmse is not None else "-"
        corr = f"{result.correlation:.4f}" if result.correlation is not None else "-"
        print(f"{result.stem:<12} {result.status:<8} {rmse:>12} {nrmse:>12} {corr:>10} {result.message}")


def main() -> int:
    args = parse_args()
    config = load_config(args.config)
    stems = discover_matching_stems(config.stems)
    prepare_output_dir(config.output_dir, config.keep_existing_output)

    if not stems:
        print("No matching .dat/.out pairs were found.")
        return 0

    build_rust_binary(config.timeout_seconds)

    results: list[ComparisonResult] = []
    all_trials: list[ComparisonResult] = []
    configs = sweep_configs(config)
    for stem in stems:
        data_path = EXAMPLES_DIR / f"{stem}.dat"
        reference_path = REFERENCE_DIR / f"{stem}.out"
        run_output_dir = config.output_dir / stem
        run_output_dir.mkdir(parents=True, exist_ok=True)

        try:
            reference = parse_reference_pr(reference_path)
            dmax = float(reference.r.max())
            pr_sample_points = int(reference.r.size)

            stem_trials: list[ComparisonResult] = []
            best_plot_payload = None
            best_result = None

            for trial_index, sweep_config in enumerate(configs, start=1):
                trial_output_dir = run_output_dir / (
                    f"basis_{sweep_config.basis_size}_lambda_{sweep_config.lambda_value:g}_drop_{sweep_config.drop_first}"
                )
                trial_output_dir.mkdir(parents=True, exist_ok=True)

                try:
                    run_cli_fit(
                        data_path=data_path,
                        output_dir=trial_output_dir,
                        dmax=dmax,
                        pr_sample_points=pr_sample_points,
                        basis_size=sweep_config.basis_size,
                        integration_intervals=config.integration_intervals,
                        lambda_value=sweep_config.lambda_value,
                        drop_first=sweep_config.drop_first,
                        timeout_seconds=config.timeout_seconds,
                    )

                    generated_r, generated_p = read_generated_pr(trial_output_dir / "pr.csv")
                    comparison = compare_curves(reference, generated_r, generated_p)

                    result = ComparisonResult(
                        stem=stem,
                        status="ok",
                        data_path=str(data_path),
                        reference_path=str(reference_path),
                        output_dir=str(trial_output_dir),
                        dmax=dmax,
                        pr_sample_points=pr_sample_points,
                        basis_size=sweep_config.basis_size,
                        integration_intervals=config.integration_intervals,
                        lambda_value=sweep_config.lambda_value,
                        drop_first=sweep_config.drop_first,
                        rmse=comparison["rmse"],
                        normalized_rmse=comparison["normalized_rmse"],
                        max_abs_error=comparison["max_abs_error"],
                        correlation=comparison["correlation"],
                        message="comparison completed",
                        sweep_trial_count=len(configs),
                    )

                    if best_result is None or score_value(result, config.score) < score_value(best_result, config.score):
                        best_result = result
                        best_plot_payload = comparison
                except Exception as error:  # noqa: BLE001
                    result = ComparisonResult(
                        stem=stem,
                        status="failed",
                        data_path=str(data_path),
                        reference_path=str(reference_path),
                        output_dir=str(trial_output_dir),
                        dmax=dmax,
                        pr_sample_points=pr_sample_points,
                        basis_size=sweep_config.basis_size,
                        integration_intervals=config.integration_intervals,
                        lambda_value=sweep_config.lambda_value,
                        drop_first=sweep_config.drop_first,
                        rmse=None,
                        normalized_rmse=None,
                        max_abs_error=None,
                        correlation=None,
                        message=str(error),
                        sweep_trial_count=len(configs),
                    )

                stem_trials.append(result)
                all_trials.append(result)

            if best_result is None:
                results.append(
                    ComparisonResult(
                        stem=stem,
                        status="failed",
                        data_path=str(data_path),
                        reference_path=str(reference_path),
                        output_dir=str(run_output_dir),
                        dmax=dmax,
                        pr_sample_points=pr_sample_points,
                        basis_size=None,
                        integration_intervals=config.integration_intervals,
                        lambda_value=None,
                        drop_first=None,
                        rmse=None,
                        normalized_rmse=None,
                        max_abs_error=None,
                        correlation=None,
                        message="all sweep trials failed",
                        sweep_trial_count=len(configs),
                    )
                )
            else:
                write_plot(
                    stem=stem,
                    reference=reference,
                    aligned_generated=best_plot_payload["aligned_generated"],
                    difference=best_plot_payload["difference"],
                    output_path=run_output_dir / "best_pr_comparison.png",
                )
                results.append(
                    ComparisonResult(
                        **{
                            **asdict(best_result),
                            "output_dir": str(run_output_dir),
                            "message": f"best of {len(configs)} trial(s)",
                        }
                    )
                )
        except Exception as error:  # noqa: BLE001
            results.append(
                ComparisonResult(
                    stem=stem,
                    status="failed",
                    data_path=str(data_path),
                    reference_path=str(reference_path),
                    output_dir=str(run_output_dir),
                    dmax=None,
                    pr_sample_points=None,
                    basis_size=None,
                    integration_intervals=config.integration_intervals,
                    lambda_value=None,
                    drop_first=None,
                    rmse=None,
                    normalized_rmse=None,
                    max_abs_error=None,
                    correlation=None,
                    message=str(error),
                    sweep_trial_count=len(configs),
                )
            )

    write_summary_files(config.output_dir, results)
    write_per_stem_sweep_summary(config.output_dir, all_trials)
    print_summary(results)
    print(f"\nDetailed outputs written to: {config.output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
