#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import shutil
import tomllib
from dataclasses import dataclass, asdict
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONFIG_PATH = ROOT / "profiling" / "noisy_benchmark_signal_scaled.toml"


@dataclass(frozen=True)
class NoisySuiteConfig:
    source_dir: Path
    output_dir: Path
    seed: int
    levels: list[float]
    scale_reference: str


@dataclass(frozen=True)
class NoisyCaseRecord:
    case_id: str
    family: str
    noise_level: float
    negative_value_count: int
    negative_value_fraction: float
    min_observed_intensity: float
    max_observed_intensity: float
    source_suite: str
    source_seed: int


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export deterministic pointwise signal-scaled noisy I(q) variants from a committed regression suite."
    )
    parser.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG_PATH,
        help=f"TOML config file describing the noisy export. Default: {DEFAULT_CONFIG_PATH}",
    )
    parser.add_argument(
        "--keep-existing-output",
        action="store_true",
        help="Keep an existing output directory instead of replacing it.",
    )
    return parser.parse_args()


def load_config(path: Path) -> NoisySuiteConfig:
    with path.open("rb") as handle:
        raw = tomllib.load(handle)

    suite = raw["suite"]
    noise = raw["noise"]
    return NoisySuiteConfig(
        source_dir=ROOT / str(suite["source_dir"]),
        output_dir=ROOT / str(suite["output_dir"]),
        seed=int(suite["seed"]),
        levels=[float(value) for value in noise["levels"]],
        scale_reference=str(noise.get("scale_reference", "pointwise_intensity")),
    )


def prepare_output_dir(path: Path, keep_existing_output: bool) -> None:
    if path.exists() and not keep_existing_output:
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def read_json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def read_iq_truth_csv(path: Path) -> tuple[np.ndarray, np.ndarray]:
    q_values: list[float] = []
    intensities: list[float] = []
    with path.open("r", encoding="utf-8") as handle:
        reader = csv.reader(handle)
        next(reader, None)
        for row in reader:
            if len(row) != 2:
                continue
            q_values.append(float(row[0]))
            intensities.append(float(row[1]))
    return np.asarray(q_values, dtype=float), np.asarray(intensities, dtype=float)


def write_noisy_iq_csv(
    path: Path,
    q_values: np.ndarray,
    truth_iq: np.ndarray,
    observed_iq: np.ndarray,
    sigma_q: np.ndarray,
    noise: np.ndarray,
) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["q", "i_of_q_truth", "sigma_q", "i_of_q_observed", "noise"])
        for q_value, truth, sigma_value, observed, delta in zip(
            q_values, truth_iq, sigma_q, observed_iq, noise, strict=True
        ):
            writer.writerow(
                [
                    f"{q_value:.12g}",
                    f"{truth:.12g}",
                    f"{sigma_value:.12g}",
                    f"{observed:.12g}",
                    f"{delta:.12g}",
                ]
            )


def write_summary_csv(path: Path, rows: list[NoisyCaseRecord]) -> None:
    fieldnames = list(asdict(rows[0]).keys()) if rows else []
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        if fieldnames:
            writer.writeheader()
            for row in rows:
                writer.writerow(asdict(row))


def deterministic_seed(base_seed: int, case_id: str, noise_level_index: int) -> int:
    case_numeric = int(case_id.split("_")[-1])
    return base_seed + 10_000 * noise_level_index + case_numeric


def main() -> int:
    args = parse_args()
    config = load_config(args.config)
    if config.scale_reference != "pointwise_intensity":
        raise ValueError(f"unsupported scale_reference: {config.scale_reference}")

    prepare_output_dir(config.output_dir, args.keep_existing_output)

    accepted_summary = read_json(config.source_dir / "accepted_summary.json")
    suite_summary = read_json(config.source_dir / "suite_summary.json")
    source_suite_name = str(suite_summary["suite_name"])
    source_seed = int(suite_summary["seed"])

    all_records: list[NoisyCaseRecord] = []

    for level_index, noise_level in enumerate(config.levels):
        level_dir = config.output_dir / f"noise_{noise_level:g}"
        level_dir.mkdir(parents=True, exist_ok=True)

        for row in accepted_summary:
            case_id = str(row["candidate_id"])
            family = str(row["family"])
            source_case_dir = config.source_dir / case_id
            target_case_dir = level_dir / case_id
            target_case_dir.mkdir(parents=True, exist_ok=True)

            q_values, truth_iq = read_iq_truth_csv(source_case_dir / "iq_truth.csv")
            sigma_q = noise_level * truth_iq

            rng = np.random.default_rng(
                deterministic_seed(config.seed, case_id, level_index)
            )
            noise = rng.normal(loc=0.0, scale=sigma_q, size=truth_iq.shape)
            observed_iq = truth_iq + noise

            negative_count = int(np.sum(observed_iq < 0.0))
            record = NoisyCaseRecord(
                case_id=case_id,
                family=family,
                noise_level=noise_level,
                negative_value_count=negative_count,
                negative_value_fraction=negative_count / observed_iq.size,
                min_observed_intensity=float(np.min(observed_iq)),
                max_observed_intensity=float(np.max(observed_iq)),
                source_suite=source_suite_name,
                source_seed=source_seed,
            )
            all_records.append(record)

            write_noisy_iq_csv(
                target_case_dir / "iq_observed.csv",
                q_values,
                truth_iq,
                observed_iq,
                sigma_q,
                noise,
            )
            shutil.copy2(source_case_dir / "pr_truth.csv", target_case_dir / "pr_truth.csv")
            shutil.copy2(source_case_dir / "metadata.json", target_case_dir / "metadata.json")
            with (target_case_dir / "noise_metadata.json").open("w", encoding="utf-8") as handle:
                json.dump(asdict(record), handle, indent=2)

    with (config.output_dir / "suite_summary.json").open("w", encoding="utf-8") as handle:
        json.dump(
            {
                "source_dir": str(config.source_dir),
                "output_dir": str(config.output_dir),
                "seed": config.seed,
                "noise_levels": config.levels,
                "scale_reference": config.scale_reference,
                "case_count": len(accepted_summary),
                "variant_count": len(all_records),
            },
            handle,
            indent=2,
        )

    with (config.output_dir / "noisy_summary.json").open("w", encoding="utf-8") as handle:
        json.dump([asdict(record) for record in all_records], handle, indent=2)
    write_summary_csv(config.output_dir / "noisy_summary.csv", all_records)

    print(f"source suite:  {config.source_dir}")
    print(f"output suite:  {config.output_dir}")
    print(f"noise levels:  {config.levels}")
    print(f"variants:      {len(all_records)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
