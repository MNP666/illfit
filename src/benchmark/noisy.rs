use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::benchmark::recovery::recover_benchmark_observed_case;
use crate::benchmark::{
    BenchmarkCaseMetadata, BenchmarkPrCurve, BenchmarkRecoveryComparison, BenchmarkRecoveryConfig,
    BenchmarkRecoveryError, BenchmarkRecoveryResult, BenchmarkSuiteComparison, BenchmarkTruthCase,
    LoadBenchmarkError, compare_benchmark_recovery,
};
use crate::data::{ParseCurveError, SaxsCurve, SaxsPoint};

/// One sampled point from a noisy observed `I(q)` curve paired with the truth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoisyBenchmarkIqPoint {
    pub q: f64,
    pub truth_intensity: f64,
    pub sigma: f64,
    pub observed_intensity: f64,
    pub noise: f64,
}

/// A validated sampled noisy observed `I(q)` curve.
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyBenchmarkIqCurve {
    points: Vec<NoisyBenchmarkIqPoint>,
}

impl NoisyBenchmarkIqCurve {
    pub fn new(points: Vec<NoisyBenchmarkIqPoint>) -> Result<Self, LoadBenchmarkError> {
        if points.is_empty() {
            return Err(LoadBenchmarkError::NoCurveRows {
                curve_name: "iq_observed".to_string(),
            });
        }

        for point in &points {
            if !point.q.is_finite()
                || !point.truth_intensity.is_finite()
                || !point.sigma.is_finite()
                || !point.observed_intensity.is_finite()
                || !point.noise.is_finite()
            {
                return Err(LoadBenchmarkError::NonFiniteCurveValue {
                    curve_name: "iq_observed".to_string(),
                });
            }
            if point.sigma <= 0.0 {
                return Err(LoadBenchmarkError::NonPositiveObservedSigma { sigma: point.sigma });
            }
        }

        for window in points.windows(2) {
            if window[0].q >= window[1].q {
                return Err(LoadBenchmarkError::NonMonotonicAxis {
                    curve_name: "iq_observed".to_string(),
                    previous_value: window[0].q,
                    current_value: window[1].q,
                });
            }
        }

        Ok(Self { points })
    }

    pub fn points(&self) -> &[NoisyBenchmarkIqPoint] {
        &self.points
    }
}

/// Per-case metadata about a noisy observed benchmark variant.
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyBenchmarkCaseMetadata {
    pub case_id: String,
    pub family: String,
    pub noise_level: f64,
    pub negative_value_count: usize,
    pub negative_value_fraction: f64,
    pub min_observed_intensity: f64,
    pub max_observed_intensity: f64,
    pub source_suite: String,
    pub source_seed: u64,
}

/// One fully loaded noisy observed benchmark case.
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyBenchmarkCase {
    pub truth_case: BenchmarkTruthCase,
    pub observed_iq: NoisyBenchmarkIqCurve,
    pub noise_metadata: NoisyBenchmarkCaseMetadata,
    pub case_dir: PathBuf,
}

/// Exported metadata recorded for a noisy benchmark suite.
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyBenchmarkSuiteSummary {
    pub source_dir: String,
    pub output_dir: String,
    pub seed: u64,
    pub noise_levels: Vec<f64>,
    pub scale_reference: String,
    pub case_count: usize,
    pub variant_count: usize,
}

/// One fully loaded noisy benchmark suite.
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyBenchmarkSuite {
    pub suite_dir: PathBuf,
    pub summary: NoisyBenchmarkSuiteSummary,
    pub cases: Vec<NoisyBenchmarkCase>,
}

/// Recovery result for every case in one noisy benchmark suite.
#[derive(Debug, Clone)]
pub struct NoisyBenchmarkSuiteRecoveryResult {
    pub suite: NoisyBenchmarkSuite,
    pub case_results: Vec<BenchmarkRecoveryResult>,
}

#[derive(Debug, Deserialize)]
struct RawNoisyBenchmarkCaseMetadata {
    case_id: String,
    family: String,
    noise_level: f64,
    negative_value_count: usize,
    negative_value_fraction: f64,
    min_observed_intensity: f64,
    max_observed_intensity: f64,
    source_suite: String,
    source_seed: u64,
}

#[derive(Debug, Deserialize)]
struct RawBenchmarkCaseMetadata {
    candidate_id: String,
    family: String,
    seed: u64,
    weights: Vec<f64>,
    rg: f64,
    i_zero: f64,
    min_pr: f64,
    min_iq: f64,
    pr_at_zero: f64,
    pr_at_dmax: f64,
    derivative_at_zero: f64,
    derivative_at_dmax: f64,
    accepted: bool,
    rejection_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawNoisyBenchmarkSuiteSummary {
    source_dir: String,
    output_dir: String,
    seed: u64,
    noise_levels: Vec<f64>,
    scale_reference: String,
    case_count: usize,
    variant_count: usize,
}

pub fn load_noisy_benchmark_suite(
    suite_dir: impl AsRef<Path>,
) -> Result<NoisyBenchmarkSuite, LoadBenchmarkError> {
    let suite_dir = suite_dir.as_ref().to_path_buf();
    let summary = load_noisy_suite_summary(suite_dir.join("suite_summary.json"))?;
    let rows = load_noisy_summary(suite_dir.join("noisy_summary.json"))?;

    let mut cases = Vec::with_capacity(rows.len());
    for row in rows {
        let level_dir = find_noise_level_dir(&suite_dir, row.noise_level)?;
        let case_dir = level_dir.join(&row.case_id);
        let truth_case = load_noisy_truth_case(&case_dir)?;
        let observed_iq = parse_noisy_iq_csv(case_dir.join("iq_observed.csv"))?;
        if truth_case.metadata.candidate_id != row.case_id {
            return Err(LoadBenchmarkError::MetadataSummaryMismatch {
                case_id: truth_case.metadata.candidate_id.clone(),
                field_name: "case_id",
            });
        }
        if truth_case.metadata.family != row.family {
            return Err(LoadBenchmarkError::MetadataSummaryMismatch {
                case_id: truth_case.metadata.candidate_id.clone(),
                field_name: "family",
            });
        }
        if observed_iq.points().len() != truth_case.iq_truth.points().len() {
            return Err(LoadBenchmarkError::ObservedTruthLengthMismatch {
                case_id: truth_case.metadata.candidate_id.clone(),
                truth_len: truth_case.iq_truth.points().len(),
                observed_len: observed_iq.points().len(),
            });
        }
        cases.push(NoisyBenchmarkCase {
            truth_case,
            observed_iq,
            noise_metadata: NoisyBenchmarkCaseMetadata {
                case_id: row.case_id,
                family: row.family,
                noise_level: row.noise_level,
                negative_value_count: row.negative_value_count,
                negative_value_fraction: row.negative_value_fraction,
                min_observed_intensity: row.min_observed_intensity,
                max_observed_intensity: row.max_observed_intensity,
                source_suite: row.source_suite,
                source_seed: row.source_seed,
            },
            case_dir,
        });
    }

    if cases.len() != summary.variant_count {
        return Err(LoadBenchmarkError::SuiteCaseCountMismatch {
            expected: summary.variant_count,
            found: cases.len(),
        });
    }

    Ok(NoisyBenchmarkSuite {
        suite_dir,
        summary,
        cases,
    })
}

pub fn recover_noisy_benchmark_case(
    case: &NoisyBenchmarkCase,
    config: BenchmarkRecoveryConfig,
) -> Result<BenchmarkRecoveryResult, BenchmarkRecoveryError> {
    recover_benchmark_observed_case(
        &case.truth_case,
        noisy_observed_curve(&case.observed_iq)?,
        config,
    )
}

pub fn recover_noisy_benchmark_suite(
    suite: &NoisyBenchmarkSuite,
    config: BenchmarkRecoveryConfig,
) -> Result<NoisyBenchmarkSuiteRecoveryResult, BenchmarkRecoveryError> {
    let case_results = suite
        .cases
        .iter()
        .map(|case| recover_noisy_benchmark_case(case, config))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(NoisyBenchmarkSuiteRecoveryResult {
        suite: suite.clone(),
        case_results,
    })
}

pub fn compare_noisy_benchmark_suite(
    suite_recovery: &NoisyBenchmarkSuiteRecoveryResult,
) -> Result<BenchmarkSuiteComparison, crate::benchmark::BenchmarkComparisonError> {
    let case_comparisons = suite_recovery
        .case_results
        .iter()
        .map(compare_benchmark_recovery)
        .collect::<Result<Vec<BenchmarkRecoveryComparison>, _>>()?;

    Ok(BenchmarkSuiteComparison {
        suite_name: suite_recovery
            .suite
            .suite_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("noisy_benchmark_suite")
            .to_string(),
        case_comparisons,
    })
}

fn load_noisy_truth_case(case_dir: &Path) -> Result<BenchmarkTruthCase, LoadBenchmarkError> {
    let metadata = load_case_metadata(case_dir.join("metadata.json"))?;
    let pr_truth = parse_pr_truth_csv(case_dir.join("pr_truth.csv"))?;
    let iq_truth = benchmark_iq_from_noisy_csv(case_dir.join("iq_observed.csv"))?;

    Ok(BenchmarkTruthCase {
        metadata,
        pr_truth,
        iq_truth,
        case_dir: case_dir.to_path_buf(),
    })
}

fn noisy_observed_curve(observed_iq: &NoisyBenchmarkIqCurve) -> Result<SaxsCurve, ParseCurveError> {
    let points = observed_iq
        .points()
        .iter()
        .map(|point| SaxsPoint {
            q: point.q,
            intensity: point.observed_intensity,
            sigma: point.sigma,
        })
        .collect::<Vec<_>>();
    SaxsCurve::new(points)
}

fn load_noisy_suite_summary(
    path: PathBuf,
) -> Result<NoisyBenchmarkSuiteSummary, LoadBenchmarkError> {
    if !path.exists() {
        return Err(LoadBenchmarkError::MissingFile { path });
    }

    let raw: RawNoisyBenchmarkSuiteSummary = serde_json::from_str(&fs::read_to_string(&path)?)?;
    Ok(NoisyBenchmarkSuiteSummary {
        source_dir: raw.source_dir,
        output_dir: raw.output_dir,
        seed: raw.seed,
        noise_levels: raw.noise_levels,
        scale_reference: raw.scale_reference,
        case_count: raw.case_count,
        variant_count: raw.variant_count,
    })
}

fn load_noisy_summary(
    path: PathBuf,
) -> Result<Vec<RawNoisyBenchmarkCaseMetadata>, LoadBenchmarkError> {
    if !path.exists() {
        return Err(LoadBenchmarkError::MissingFile { path });
    }

    Ok(serde_json::from_str(&fs::read_to_string(&path)?)?)
}

fn parse_noisy_iq_csv(path: PathBuf) -> Result<NoisyBenchmarkIqCurve, LoadBenchmarkError> {
    if !path.exists() {
        return Err(LoadBenchmarkError::MissingFile { path });
    }

    let contents = fs::read_to_string(&path)?;
    let mut lines = contents.lines();
    let Some(header) = lines.next() else {
        return Err(LoadBenchmarkError::InvalidCsvHeader {
            path,
            expected: "q,i_of_q_truth,sigma_q,i_of_q_observed,noise",
            found: String::new(),
        });
    };

    let normalized_header = header.trim().trim_matches('\u{feff}');
    let expected = "q,i_of_q_truth,sigma_q,i_of_q_observed,noise";
    if normalized_header != expected {
        return Err(LoadBenchmarkError::InvalidCsvHeader {
            path,
            expected,
            found: normalized_header.to_string(),
        });
    }

    let mut points = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let fields = trimmed.split(',').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(LoadBenchmarkError::InvalidCsvRow {
                path,
                row: trimmed.to_string(),
            });
        }

        let Ok(q) = fields[0].parse::<f64>() else {
            return Err(LoadBenchmarkError::InvalidCsvRow {
                path,
                row: trimmed.to_string(),
            });
        };
        let Ok(truth_intensity) = fields[1].parse::<f64>() else {
            return Err(LoadBenchmarkError::InvalidCsvRow {
                path,
                row: trimmed.to_string(),
            });
        };
        let Ok(sigma) = fields[2].parse::<f64>() else {
            return Err(LoadBenchmarkError::InvalidCsvRow {
                path,
                row: trimmed.to_string(),
            });
        };
        let Ok(observed_intensity) = fields[3].parse::<f64>() else {
            return Err(LoadBenchmarkError::InvalidCsvRow {
                path,
                row: trimmed.to_string(),
            });
        };
        let Ok(noise) = fields[4].parse::<f64>() else {
            return Err(LoadBenchmarkError::InvalidCsvRow {
                path,
                row: trimmed.to_string(),
            });
        };

        points.push(NoisyBenchmarkIqPoint {
            q,
            truth_intensity,
            sigma,
            observed_intensity,
            noise,
        });
    }

    NoisyBenchmarkIqCurve::new(points)
}

fn benchmark_iq_from_noisy_csv(
    path: PathBuf,
) -> Result<crate::benchmark::BenchmarkIqCurve, LoadBenchmarkError> {
    let observed = parse_noisy_iq_csv(path)?;
    let points = observed
        .points()
        .iter()
        .map(|point| crate::benchmark::BenchmarkIqPoint {
            q: point.q,
            intensity: point.truth_intensity,
        })
        .collect::<Vec<_>>();
    crate::benchmark::BenchmarkIqCurve::new(points)
}

fn load_case_metadata(path: PathBuf) -> Result<BenchmarkCaseMetadata, LoadBenchmarkError> {
    if !path.exists() {
        return Err(LoadBenchmarkError::MissingFile { path });
    }

    let raw: RawBenchmarkCaseMetadata = serde_json::from_str(&fs::read_to_string(&path)?)?;
    Ok(BenchmarkCaseMetadata {
        candidate_id: raw.candidate_id,
        family: raw.family,
        seed: raw.seed,
        weights: raw.weights,
        rg: raw.rg,
        i_zero: raw.i_zero,
        min_pr: raw.min_pr,
        min_iq: raw.min_iq,
        pr_at_zero: raw.pr_at_zero,
        pr_at_dmax: raw.pr_at_dmax,
        derivative_at_zero: raw.derivative_at_zero,
        derivative_at_dmax: raw.derivative_at_dmax,
        accepted: raw.accepted,
        rejection_reason: raw.rejection_reason,
    })
}

fn parse_pr_truth_csv(path: PathBuf) -> Result<BenchmarkPrCurve, LoadBenchmarkError> {
    let rows = super::suite::parse_two_column_csv(path, "r,p_of_r")?;
    let points = rows
        .into_iter()
        .map(|(r, p_of_r)| crate::benchmark::BenchmarkPrPoint { r, p_of_r })
        .collect();
    BenchmarkPrCurve::new(points)
}

fn find_noise_level_dir(suite_dir: &Path, noise_level: f64) -> Result<PathBuf, LoadBenchmarkError> {
    for entry in fs::read_dir(suite_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(suffix) = name.strip_prefix("noise_") else {
            continue;
        };
        let Ok(parsed_level) = suffix.parse::<f64>() else {
            continue;
        };
        if (parsed_level - noise_level).abs() <= 1.0e-6 {
            return Ok(path);
        }
    }

    Err(LoadBenchmarkError::MissingFile {
        path: suite_dir.join(format!("noise_{noise_level}")),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        compare_noisy_benchmark_suite, load_noisy_benchmark_suite, recover_noisy_benchmark_suite,
    };
    use crate::benchmark::BenchmarkRecoveryConfig;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("illfit_{name}_{nanos}"))
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn write_minimal_noisy_suite(root: &Path) {
        write_file(
            &root.join("suite_summary.json"),
            r#"{
  "source_dir": "data/regression/clamped_spline",
  "output_dir": "data/synthetic/noisy_demo",
  "seed": 321,
  "noise_levels": [0.2],
  "scale_reference": "pointwise_intensity",
  "case_count": 1,
  "variant_count": 1
}"#,
        );
        write_file(
            &root.join("noisy_summary.json"),
            r#"[
  {
    "case_id": "case_0000",
    "family": "clamped_spline_random",
    "noise_level": 0.2,
    "negative_value_count": 1,
    "negative_value_fraction": 0.3333333333,
    "min_observed_intensity": -0.1,
    "max_observed_intensity": 5.5,
    "source_suite": "clamped_spline",
    "source_seed": 40
  }
]"#,
        );
        write_file(
            &root
                .join("noise_0.2")
                .join("case_0000")
                .join("metadata.json"),
            r#"{
  "candidate_id": "case_0000",
  "family": "clamped_spline_random",
  "seed": 42,
  "weights": [0.5, 1.0],
  "rg": 10.0,
  "i_zero": 5.0,
  "min_pr": 0.0,
  "min_iq": 0.1,
  "pr_at_zero": 0.0,
  "pr_at_dmax": 0.0,
  "derivative_at_zero": 0.0,
  "derivative_at_dmax": 0.0,
  "accepted": true,
  "rejection_reason": null
}"#,
        );
        write_file(
            &root
                .join("noise_0.2")
                .join("case_0000")
                .join("pr_truth.csv"),
            "r,p_of_r\n0.0,0.0\n1.0,1.0\n2.0,0.0\n",
        );
        write_file(
            &root
                .join("noise_0.2")
                .join("case_0000")
                .join("iq_observed.csv"),
            "q,i_of_q_truth,sigma_q,i_of_q_observed,noise\n0.1,5.0,0.5,5.5,0.5\n0.2,4.0,0.4,3.9,-0.1\n0.3,3.0,0.3,-0.1,-3.1\n",
        );
    }

    #[test]
    fn loads_noisy_suite_layout() {
        let root = unique_temp_dir("noisy_suite");
        write_minimal_noisy_suite(&root);

        let suite = load_noisy_benchmark_suite(&root).unwrap();

        assert_eq!(suite.cases.len(), 1);
        assert_eq!(suite.cases[0].noise_metadata.noise_level, 0.2);
        assert_eq!(
            suite.cases[0].observed_iq.points()[2].observed_intensity,
            -0.1
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovers_and_compares_noisy_suite() {
        let root = unique_temp_dir("noisy_recovery");
        write_minimal_noisy_suite(&root);
        let suite = load_noisy_benchmark_suite(&root).unwrap();

        let recovery = recover_noisy_benchmark_suite(
            &suite,
            BenchmarkRecoveryConfig {
                dmax: 2.0,
                basis_size: 5,
                integration_intervals: 200,
                lambda: 1.0e-2,
                pr_sample_points: 21,
                synthetic_sigma: 0.1,
            },
        )
        .unwrap();
        let comparison = compare_noisy_benchmark_suite(&recovery).unwrap();

        assert_eq!(recovery.case_results.len(), 1);
        assert_eq!(comparison.case_comparisons.len(), 1);

        fs::remove_dir_all(root).unwrap();
    }
}
