use serde::Deserialize;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// One sampled point from a synthetic truth `P(r)` curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchmarkPrPoint {
    pub r: f64,
    pub p_of_r: f64,
}

/// A validated sampled synthetic truth `P(r)` curve.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkPrCurve {
    points: Vec<BenchmarkPrPoint>,
}

impl BenchmarkPrCurve {
    pub fn new(points: Vec<BenchmarkPrPoint>) -> Result<Self, LoadBenchmarkError> {
        if points.is_empty() {
            return Err(LoadBenchmarkError::NoCurveRows {
                curve_name: "pr_truth".to_string(),
            });
        }

        for point in &points {
            if !point.r.is_finite() || !point.p_of_r.is_finite() {
                return Err(LoadBenchmarkError::NonFiniteCurveValue {
                    curve_name: "pr_truth".to_string(),
                });
            }
        }

        for window in points.windows(2) {
            if window[0].r >= window[1].r {
                return Err(LoadBenchmarkError::NonMonotonicAxis {
                    curve_name: "pr_truth".to_string(),
                    previous_value: window[0].r,
                    current_value: window[1].r,
                });
            }
        }

        Ok(Self { points })
    }

    pub fn points(&self) -> &[BenchmarkPrPoint] {
        &self.points
    }
}

/// One sampled point from a synthetic truth `I(q)` curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchmarkIqPoint {
    pub q: f64,
    pub intensity: f64,
}

/// A validated sampled synthetic truth `I(q)` curve.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkIqCurve {
    points: Vec<BenchmarkIqPoint>,
}

impl BenchmarkIqCurve {
    pub fn new(points: Vec<BenchmarkIqPoint>) -> Result<Self, LoadBenchmarkError> {
        if points.is_empty() {
            return Err(LoadBenchmarkError::NoCurveRows {
                curve_name: "iq_truth".to_string(),
            });
        }

        for point in &points {
            if !point.q.is_finite() || !point.intensity.is_finite() {
                return Err(LoadBenchmarkError::NonFiniteCurveValue {
                    curve_name: "iq_truth".to_string(),
                });
            }
        }

        for window in points.windows(2) {
            if window[0].q >= window[1].q {
                return Err(LoadBenchmarkError::NonMonotonicAxis {
                    curve_name: "iq_truth".to_string(),
                    previous_value: window[0].q,
                    current_value: window[1].q,
                });
            }
        }

        Ok(Self { points })
    }

    pub fn points(&self) -> &[BenchmarkIqPoint] {
        &self.points
    }
}

/// Metadata describing one accepted synthetic truth case.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkCaseMetadata {
    pub candidate_id: String,
    pub family: String,
    pub seed: u64,
    pub weights: Vec<f64>,
    pub rg: f64,
    pub i_zero: f64,
    pub min_pr: f64,
    pub min_iq: f64,
    pub pr_at_zero: f64,
    pub pr_at_dmax: f64,
    pub derivative_at_zero: f64,
    pub derivative_at_dmax: f64,
    pub accepted: bool,
    pub rejection_reason: Option<String>,
}

/// One fully loaded synthetic truth case.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkTruthCase {
    pub metadata: BenchmarkCaseMetadata,
    pub pr_truth: BenchmarkPrCurve,
    pub iq_truth: BenchmarkIqCurve,
    pub case_dir: PathBuf,
}

/// Basic exported suite configuration recorded in `suite_summary.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkSuiteConfig {
    pub name: String,
    pub output_dir: String,
    pub seed: u64,
    pub candidate_count: usize,
    pub max_accepted: usize,
    pub dmax: f64,
    pub n_weights: usize,
    pub r_points: usize,
    pub weight_min: f64,
    pub weight_max: f64,
    pub q_min: f64,
    pub q_max: f64,
    pub q_points: usize,
    pub integration_intervals: usize,
    pub tolerance: f64,
}

/// Summary of one exported synthetic benchmark suite.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkSuiteSummary {
    pub suite_name: String,
    pub seed: u64,
    pub candidate_count: usize,
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub config: BenchmarkSuiteConfig,
}

/// One fully loaded synthetic benchmark suite.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkSuite {
    pub suite_dir: PathBuf,
    pub summary: BenchmarkSuiteSummary,
    pub truth_cases: Vec<BenchmarkTruthCase>,
}

#[derive(Debug)]
pub enum LoadBenchmarkError {
    Io(std::io::Error),
    Json(serde_json::Error),
    MissingFile {
        path: PathBuf,
    },
    InvalidCsvHeader {
        path: PathBuf,
        expected: &'static str,
        found: String,
    },
    InvalidCsvRow {
        path: PathBuf,
        row: String,
    },
    NoCurveRows {
        curve_name: String,
    },
    NonFiniteCurveValue {
        curve_name: String,
    },
    NonPositiveObservedSigma {
        sigma: f64,
    },
    NonMonotonicAxis {
        curve_name: String,
        previous_value: f64,
        current_value: f64,
    },
    ObservedTruthLengthMismatch {
        case_id: String,
        truth_len: usize,
        observed_len: usize,
    },
    MetadataSummaryMismatch {
        case_id: String,
        field_name: &'static str,
    },
    UnacceptedCaseInAcceptedSummary {
        case_id: String,
    },
    SuiteCaseCountMismatch {
        expected: usize,
        found: usize,
    },
}

impl fmt::Display for LoadBenchmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read benchmark asset: {error}"),
            Self::Json(error) => write!(f, "failed to parse benchmark JSON: {error}"),
            Self::MissingFile { path } => {
                write!(f, "missing benchmark asset file: {}", path.display())
            }
            Self::InvalidCsvHeader {
                path,
                expected,
                found,
            } => write!(
                f,
                "unexpected CSV header in {}: expected `{expected}`, found `{found}`",
                path.display()
            ),
            Self::InvalidCsvRow { path, row } => {
                write!(f, "invalid CSV row in {}: `{row}`", path.display())
            }
            Self::NoCurveRows { curve_name } => {
                write!(f, "no rows were found for benchmark curve `{curve_name}`")
            }
            Self::NonFiniteCurveValue { curve_name } => write!(
                f,
                "encountered a non-finite value in benchmark curve `{curve_name}`"
            ),
            Self::NonPositiveObservedSigma { sigma } => {
                write!(
                    f,
                    "encountered a non-positive observed sigma value: {sigma}"
                )
            }
            Self::NonMonotonicAxis {
                curve_name,
                previous_value,
                current_value,
            } => write!(
                f,
                "axis values in benchmark curve `{curve_name}` must be strictly increasing, but found {previous_value} followed by {current_value}"
            ),
            Self::MetadataSummaryMismatch {
                case_id,
                field_name,
            } => write!(
                f,
                "metadata and accepted summary disagree for case `{case_id}` on field `{field_name}`"
            ),
            Self::UnacceptedCaseInAcceptedSummary { case_id } => write!(
                f,
                "accepted summary contains case `{case_id}` marked as not accepted"
            ),
            Self::ObservedTruthLengthMismatch {
                case_id,
                truth_len,
                observed_len,
            } => write!(
                f,
                "noisy observed and truth I(q) lengths differ for case `{case_id}`: truth has {truth_len} rows but observed has {observed_len}"
            ),
            Self::SuiteCaseCountMismatch { expected, found } => write!(
                f,
                "suite summary expected {expected} accepted cases, but {found} were loaded"
            ),
        }
    }
}

impl Error for LoadBenchmarkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::MissingFile { .. }
            | Self::InvalidCsvHeader { .. }
            | Self::InvalidCsvRow { .. }
            | Self::NoCurveRows { .. }
            | Self::NonFiniteCurveValue { .. }
            | Self::NonPositiveObservedSigma { .. }
            | Self::NonMonotonicAxis { .. }
            | Self::MetadataSummaryMismatch { .. }
            | Self::UnacceptedCaseInAcceptedSummary { .. }
            | Self::ObservedTruthLengthMismatch { .. }
            | Self::SuiteCaseCountMismatch { .. } => None,
        }
    }
}

impl From<std::io::Error> for LoadBenchmarkError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for LoadBenchmarkError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawBenchmarkCaseMetadata {
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
struct RawBenchmarkSuiteConfig {
    name: String,
    output_dir: String,
    seed: u64,
    candidate_count: usize,
    max_accepted: usize,
    dmax: f64,
    n_weights: usize,
    r_points: usize,
    weight_min: f64,
    weight_max: f64,
    q_min: f64,
    q_max: f64,
    q_points: usize,
    integration_intervals: usize,
    tolerance: f64,
}

#[derive(Debug, Deserialize)]
struct RawBenchmarkSuiteSummary {
    suite_name: String,
    seed: u64,
    candidate_count: usize,
    accepted_count: usize,
    rejected_count: usize,
    config: RawBenchmarkSuiteConfig,
}

pub fn load_benchmark_truth_case(
    case_dir: impl AsRef<Path>,
) -> Result<BenchmarkTruthCase, LoadBenchmarkError> {
    let case_dir = case_dir.as_ref().to_path_buf();
    let metadata = load_case_metadata(case_dir.join("metadata.json"))?;
    let pr_truth = parse_pr_truth_csv(case_dir.join("pr_truth.csv"))?;
    let iq_truth = parse_iq_truth_csv(case_dir.join("iq_truth.csv"))?;

    Ok(BenchmarkTruthCase {
        metadata,
        pr_truth,
        iq_truth,
        case_dir,
    })
}

pub fn load_benchmark_suite(
    suite_dir: impl AsRef<Path>,
) -> Result<BenchmarkSuite, LoadBenchmarkError> {
    let suite_dir = suite_dir.as_ref().to_path_buf();
    let summary = load_suite_summary(suite_dir.join("suite_summary.json"))?;
    let accepted_rows = load_accepted_summary(suite_dir.join("accepted_summary.json"))?;

    let mut truth_cases = Vec::with_capacity(accepted_rows.len());
    for accepted_row in accepted_rows {
        if !accepted_row.accepted {
            return Err(LoadBenchmarkError::UnacceptedCaseInAcceptedSummary {
                case_id: accepted_row.candidate_id.clone(),
            });
        }

        let case = load_benchmark_truth_case(suite_dir.join(&accepted_row.candidate_id))?;
        assert_case_matches_summary(&case.metadata, &accepted_row)?;
        truth_cases.push(case);
    }

    if truth_cases.len() != summary.accepted_count {
        return Err(LoadBenchmarkError::SuiteCaseCountMismatch {
            expected: summary.accepted_count,
            found: truth_cases.len(),
        });
    }

    Ok(BenchmarkSuite {
        suite_dir,
        summary,
        truth_cases,
    })
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

fn load_suite_summary(path: PathBuf) -> Result<BenchmarkSuiteSummary, LoadBenchmarkError> {
    if !path.exists() {
        return Err(LoadBenchmarkError::MissingFile { path });
    }

    let raw: RawBenchmarkSuiteSummary = serde_json::from_str(&fs::read_to_string(&path)?)?;
    Ok(BenchmarkSuiteSummary {
        suite_name: raw.suite_name,
        seed: raw.seed,
        candidate_count: raw.candidate_count,
        accepted_count: raw.accepted_count,
        rejected_count: raw.rejected_count,
        config: BenchmarkSuiteConfig {
            name: raw.config.name,
            output_dir: raw.config.output_dir,
            seed: raw.config.seed,
            candidate_count: raw.config.candidate_count,
            max_accepted: raw.config.max_accepted,
            dmax: raw.config.dmax,
            n_weights: raw.config.n_weights,
            r_points: raw.config.r_points,
            weight_min: raw.config.weight_min,
            weight_max: raw.config.weight_max,
            q_min: raw.config.q_min,
            q_max: raw.config.q_max,
            q_points: raw.config.q_points,
            integration_intervals: raw.config.integration_intervals,
            tolerance: raw.config.tolerance,
        },
    })
}

fn load_accepted_summary(path: PathBuf) -> Result<Vec<BenchmarkCaseMetadata>, LoadBenchmarkError> {
    if !path.exists() {
        return Err(LoadBenchmarkError::MissingFile { path });
    }

    let raw_rows: Vec<RawBenchmarkCaseMetadata> =
        serde_json::from_str(&fs::read_to_string(&path)?)?;
    Ok(raw_rows
        .into_iter()
        .map(|raw| BenchmarkCaseMetadata {
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
        .collect())
}

fn parse_pr_truth_csv(path: PathBuf) -> Result<BenchmarkPrCurve, LoadBenchmarkError> {
    let rows = parse_two_column_csv(path, "r,p_of_r")?;
    let points = rows
        .into_iter()
        .map(|(r, p_of_r)| BenchmarkPrPoint { r, p_of_r })
        .collect();
    BenchmarkPrCurve::new(points)
}

fn parse_iq_truth_csv(path: PathBuf) -> Result<BenchmarkIqCurve, LoadBenchmarkError> {
    let rows = parse_two_column_csv(path, "q,i_of_q")?;
    let points = rows
        .into_iter()
        .map(|(q, intensity)| BenchmarkIqPoint { q, intensity })
        .collect();
    BenchmarkIqCurve::new(points)
}

pub(crate) fn parse_two_column_csv(
    path: PathBuf,
    expected_header: &'static str,
) -> Result<Vec<(f64, f64)>, LoadBenchmarkError> {
    if !path.exists() {
        return Err(LoadBenchmarkError::MissingFile { path });
    }

    let contents = fs::read_to_string(&path)?;
    let mut lines = contents.lines();
    let Some(header) = lines.next() else {
        return Err(LoadBenchmarkError::InvalidCsvHeader {
            path,
            expected: expected_header,
            found: String::new(),
        });
    };

    let normalized_header = header.trim().trim_matches('\u{feff}');
    if normalized_header != expected_header {
        return Err(LoadBenchmarkError::InvalidCsvHeader {
            path,
            expected: expected_header,
            found: normalized_header.to_string(),
        });
    }

    let mut rows = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((left, right)) = trimmed.split_once(',') else {
            return Err(LoadBenchmarkError::InvalidCsvRow {
                path,
                row: trimmed.to_string(),
            });
        };

        let Ok(first) = left.parse::<f64>() else {
            return Err(LoadBenchmarkError::InvalidCsvRow {
                path,
                row: trimmed.to_string(),
            });
        };
        let Ok(second) = right.parse::<f64>() else {
            return Err(LoadBenchmarkError::InvalidCsvRow {
                path,
                row: trimmed.to_string(),
            });
        };

        rows.push((first, second));
    }

    Ok(rows)
}

fn assert_case_matches_summary(
    metadata: &BenchmarkCaseMetadata,
    summary_row: &BenchmarkCaseMetadata,
) -> Result<(), LoadBenchmarkError> {
    if metadata.candidate_id != summary_row.candidate_id {
        return Err(LoadBenchmarkError::MetadataSummaryMismatch {
            case_id: metadata.candidate_id.clone(),
            field_name: "candidate_id",
        });
    }
    if metadata.family != summary_row.family {
        return Err(LoadBenchmarkError::MetadataSummaryMismatch {
            case_id: metadata.candidate_id.clone(),
            field_name: "family",
        });
    }
    if metadata.accepted != summary_row.accepted {
        return Err(LoadBenchmarkError::MetadataSummaryMismatch {
            case_id: metadata.candidate_id.clone(),
            field_name: "accepted",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LoadBenchmarkError, load_benchmark_suite, load_benchmark_truth_case};
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

    fn write_minimal_suite(root: &Path) {
        write_file(
            &root.join("suite_summary.json"),
            r#"{
  "suite_name": "demo_suite",
  "seed": 42,
  "candidate_count": 3,
  "accepted_count": 1,
  "rejected_count": 2,
  "config": {
    "name": "demo_suite",
    "output_dir": "data/regression/demo_suite",
    "seed": 42,
    "candidate_count": 3,
    "max_accepted": 1,
    "dmax": 120.0,
    "n_weights": 5,
    "r_points": 5,
    "weight_min": 0.2,
    "weight_max": 2.0,
    "q_min": 0.005,
    "q_max": 0.35,
    "q_points": 4,
    "integration_intervals": 100,
    "tolerance": 1e-8
  }
}"#,
        );
        write_file(
            &root.join("accepted_summary.json"),
            r#"[
  {
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
  }
]"#,
        );
        write_file(
            &root.join("case_0000").join("metadata.json"),
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
            &root.join("case_0000").join("pr_truth.csv"),
            "r,p_of_r\n0.0,0.0\n1.0,1.0\n2.0,0.0\n",
        );
        write_file(
            &root.join("case_0000").join("iq_truth.csv"),
            "q,i_of_q\n0.1,5.0\n0.2,4.0\n0.3,3.0\n",
        );
    }

    #[test]
    fn loads_one_truth_case_from_exported_layout() {
        let root = unique_temp_dir("benchmark_case");
        write_minimal_suite(&root);

        let case = load_benchmark_truth_case(root.join("case_0000")).unwrap();

        assert_eq!(case.metadata.candidate_id, "case_0000");
        assert_eq!(case.pr_truth.points()[1].p_of_r, 1.0);
        assert_eq!(case.iq_truth.points()[0].intensity, 5.0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loads_suite_and_links_truth_cases() {
        let root = unique_temp_dir("benchmark_suite");
        write_minimal_suite(&root);

        let suite = load_benchmark_suite(&root).unwrap();

        assert_eq!(suite.summary.suite_name, "demo_suite");
        assert_eq!(suite.truth_cases.len(), 1);
        assert_eq!(
            suite.truth_cases[0].metadata.family,
            "clamped_spline_random"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_non_monotonic_pr_truth_axis() {
        let root = unique_temp_dir("benchmark_bad_pr");
        write_minimal_suite(&root);
        write_file(
            &root.join("case_0000").join("pr_truth.csv"),
            "r,p_of_r\n0.0,0.0\n1.0,1.0\n0.5,0.0\n",
        );

        let error = load_benchmark_truth_case(root.join("case_0000")).unwrap_err();
        assert!(matches!(error, LoadBenchmarkError::NonMonotonicAxis { .. }));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_metadata_summary_mismatch() {
        let root = unique_temp_dir("benchmark_mismatch");
        write_minimal_suite(&root);
        write_file(
            &root.join("accepted_summary.json"),
            r#"[
  {
    "candidate_id": "case_0000",
    "family": "wrong_family",
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
  }
]"#,
        );

        let error = load_benchmark_suite(&root).unwrap_err();
        assert!(matches!(
            error,
            LoadBenchmarkError::MetadataSummaryMismatch { .. }
        ));

        fs::remove_dir_all(root).unwrap();
    }
}
