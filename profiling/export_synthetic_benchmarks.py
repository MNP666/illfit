#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import math
import shutil
import tomllib
from dataclasses import asdict, dataclass
from pathlib import Path

import numpy as np
from scipy.interpolate import CubicSpline

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONFIG_PATH = ROOT / "profiling" / "benchmark_export.toml"


@dataclass(frozen=True)
class SuiteConfig:
    name: str
    output_dir: Path
    seed: int
    candidate_count: int
    max_accepted: int
    dmax: float
    n_weights: int
    r_points: int
    weight_min: float
    weight_max: float
    q_min: float
    q_max: float
    q_points: int
    integration_intervals: int
    tolerance: float


@dataclass(frozen=True)
class CandidateRecord:
    candidate_id: str
    family: str
    seed: int
    weights: list[float]
    rg: float
    i_zero: float
    min_pr: float
    min_iq: float
    pr_at_zero: float
    pr_at_dmax: float
    derivative_at_zero: float
    derivative_at_dmax: float
    accepted: bool
    rejection_reason: str | None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export deterministic clamped-spline synthetic SAXS benchmark assets."
    )
    parser.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG_PATH,
        help=f"TOML config file describing the benchmark export. Default: {DEFAULT_CONFIG_PATH}",
    )
    parser.add_argument(
        "--keep-existing-output",
        action="store_true",
        help="Keep an existing output directory instead of replacing it.",
    )
    return parser.parse_args()


def load_config(path: Path) -> SuiteConfig:
    with path.open("rb") as handle:
        raw = tomllib.load(handle)

    suite = raw["suite"]
    pr = raw["pr"]
    iq = raw["iq"]
    screening = raw["screening"]

    output_dir = ROOT / str(suite["output_dir"])

    return SuiteConfig(
        name=str(suite["name"]),
        output_dir=output_dir,
        seed=int(suite["seed"]),
        candidate_count=int(suite["candidate_count"]),
        max_accepted=int(suite["max_accepted"]),
        dmax=float(pr["dmax"]),
        n_weights=int(pr["n_weights"]),
        r_points=int(pr["r_points"]),
        weight_min=float(pr["weight_min"]),
        weight_max=float(pr["weight_max"]),
        q_min=float(iq["q_min"]),
        q_max=float(iq["q_max"]),
        q_points=int(iq["q_points"]),
        integration_intervals=int(iq["integration_intervals"]),
        tolerance=float(screening["tolerance"]),
    )


def prepare_output_dir(path: Path, keep_existing_output: bool) -> None:
    if path.exists() and not keep_existing_output:
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def make_clamped_spline(weights: np.ndarray, r_max: float) -> CubicSpline:
    x = np.linspace(0.0, r_max, weights.size + 2)
    y = np.concatenate([[0.0], weights, [0.0]])
    return CubicSpline(x, y, bc_type="clamped")


def forward_transform(
    p_of_r: np.ndarray,
    r_grid: np.ndarray,
    q_grid: np.ndarray,
    integration_intervals: int,
) -> np.ndarray:
    r_edges = np.linspace(float(r_grid[0]), float(r_grid[-1]), integration_intervals + 1)
    r_midpoints = 0.5 * (r_edges[:-1] + r_edges[1:])
    delta_r = r_edges[1] - r_edges[0]
    p_midpoints = np.interp(r_midpoints, r_grid, p_of_r)
    qr = np.outer(q_grid, r_midpoints)
    sinc = np.sinc(qr / np.pi)
    return sinc @ (p_midpoints * delta_r)


def compute_i_zero(p_of_r: np.ndarray, r_grid: np.ndarray) -> float:
    return float(np.trapezoid(p_of_r, r_grid))


def compute_rg(p_of_r: np.ndarray, r_grid: np.ndarray) -> float:
    i_zero = compute_i_zero(p_of_r, r_grid)
    if i_zero <= 0.0:
        return float("nan")
    second_moment = float(np.trapezoid((r_grid**2) * p_of_r, r_grid))
    return float(np.sqrt(second_moment / (2.0 * i_zero)))


def rejection_reason(
    p_of_r: np.ndarray,
    i_of_q: np.ndarray,
    spline: CubicSpline,
    tolerance: float,
    dmax: float,
) -> str | None:
    if float(np.min(p_of_r)) < -tolerance:
        return "negative_pr"
    if abs(float(p_of_r[0])) > tolerance:
        return "pr_nonzero_at_zero"
    if abs(float(p_of_r[-1])) > tolerance:
        return "pr_nonzero_at_dmax"
    if float(np.min(i_of_q)) < -tolerance:
        return "negative_iq"
    if not math.isfinite(float(spline(0.0, 1))) or not math.isfinite(float(spline(dmax, 1))):
        return "non_finite_endpoint_derivative"
    return None


def write_curve_csv(path: Path, x_label: str, y_label: str, x: np.ndarray, y: np.ndarray) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow([x_label, y_label])
        for x_value, y_value in zip(x, y, strict=True):
            writer.writerow([f"{x_value:.12g}", f"{y_value:.12g}"])


def write_summary_csv(path: Path, rows: list[CandidateRecord]) -> None:
    fieldnames = list(asdict(rows[0]).keys()) if rows else []
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        if fieldnames:
            writer.writeheader()
            for row in rows:
                writer.writerow(asdict(row))


def config_to_json_dict(config: SuiteConfig) -> dict[str, object]:
    return {
        "name": config.name,
        "output_dir": str(config.output_dir),
        "seed": config.seed,
        "candidate_count": config.candidate_count,
        "max_accepted": config.max_accepted,
        "dmax": config.dmax,
        "n_weights": config.n_weights,
        "r_points": config.r_points,
        "weight_min": config.weight_min,
        "weight_max": config.weight_max,
        "q_min": config.q_min,
        "q_max": config.q_max,
        "q_points": config.q_points,
        "integration_intervals": config.integration_intervals,
        "tolerance": config.tolerance,
    }


def main() -> int:
    args = parse_args()
    config = load_config(args.config)
    prepare_output_dir(config.output_dir, args.keep_existing_output)

    r_grid = np.linspace(0.0, config.dmax, config.r_points)
    q_grid = np.linspace(config.q_min, config.q_max, config.q_points)

    rng = np.random.default_rng(seed=config.seed)

    accepted: list[CandidateRecord] = []
    rejected: list[CandidateRecord] = []

    for candidate_index in range(config.candidate_count):
        weights = rng.uniform(config.weight_min, config.weight_max, size=config.n_weights)
        spline = make_clamped_spline(weights, config.dmax)
        p_of_r = spline(r_grid)
        i_of_q = forward_transform(p_of_r, r_grid, q_grid, config.integration_intervals)

        record = CandidateRecord(
            candidate_id=f"case_{candidate_index:04d}",
            family="clamped_spline_random",
            seed=config.seed,
            weights=[float(value) for value in weights],
            rg=compute_rg(p_of_r, r_grid),
            i_zero=compute_i_zero(p_of_r, r_grid),
            min_pr=float(np.min(p_of_r)),
            min_iq=float(np.min(i_of_q)),
            pr_at_zero=float(p_of_r[0]),
            pr_at_dmax=float(p_of_r[-1]),
            derivative_at_zero=float(spline(0.0, 1)),
            derivative_at_dmax=float(spline(config.dmax, 1)),
            accepted=False,
            rejection_reason=None,
        )

        reason = rejection_reason(p_of_r, i_of_q, spline, config.tolerance, config.dmax)
        if reason is None and len(accepted) < config.max_accepted:
            accepted_record = CandidateRecord(**{**asdict(record), "accepted": True})
            accepted.append(accepted_record)

            case_dir = config.output_dir / accepted_record.candidate_id
            case_dir.mkdir(parents=True, exist_ok=True)
            write_curve_csv(case_dir / "pr_truth.csv", "r", "p_of_r", r_grid, p_of_r)
            write_curve_csv(case_dir / "iq_truth.csv", "q", "i_of_q", q_grid, i_of_q)
            with (case_dir / "metadata.json").open("w", encoding="utf-8") as handle:
                json.dump(asdict(accepted_record), handle, indent=2)
        else:
            rejected.append(CandidateRecord(**{**asdict(record), "rejection_reason": reason or "accepted_limit_reached"}))

    summary = {
        "suite_name": config.name,
        "seed": config.seed,
        "candidate_count": config.candidate_count,
        "accepted_count": len(accepted),
        "rejected_count": len(rejected),
        "config": config_to_json_dict(config),
    }

    with (config.output_dir / "suite_summary.json").open("w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2)

    with (config.output_dir / "accepted_summary.json").open("w", encoding="utf-8") as handle:
        json.dump([asdict(row) for row in accepted], handle, indent=2)

    with (config.output_dir / "rejected_summary.json").open("w", encoding="utf-8") as handle:
        json.dump([asdict(row) for row in rejected], handle, indent=2)

    write_summary_csv(config.output_dir / "accepted_summary.csv", accepted)
    write_summary_csv(config.output_dir / "rejected_summary.csv", rejected)

    print(f"suite:     {config.name}")
    print(f"output:    {config.output_dir}")
    print(f"accepted:  {len(accepted)}")
    print(f"rejected:  {len(rejected)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
