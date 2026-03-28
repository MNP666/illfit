use std::error::Error;
use std::fmt;

use crate::analysis::{AnalysisError, DmaxScanMetricRange, FitSummary, summarize_fit};
use crate::basis::{BasisError, CubicBSplineBasis};
use crate::data::{ParseCurveError, SaxsCurve};
use crate::solver::{FitResult, SolverError, solve_curve};
use crate::transform::{ForwardTransform, TransformError};

#[derive(Debug, Clone)]
pub struct TruncationScanConfig {
    pub dmax: f64,
    pub baseline_drop_count: usize,
    pub step_size: usize,
    pub point_count: usize,
    pub basis_size: usize,
    pub integration_intervals: usize,
    pub lambda: f64,
    pub pr_sample_point_count: usize,
}

#[derive(Debug)]
pub enum TruncationScanError {
    Basis(BasisError),
    Transform(TransformError),
    Solver(SolverError),
    Analysis(AnalysisError),
    Curve(ParseCurveError),
    InvalidDmax { dmax: f64 },
    InvalidStepSize { step_size: usize },
    InvalidPointCount { point_count: usize },
}

impl fmt::Display for TruncationScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basis(error) => write!(f, "{error}"),
            Self::Transform(error) => write!(f, "{error}"),
            Self::Solver(error) => write!(f, "{error}"),
            Self::Analysis(error) => write!(f, "{error}"),
            Self::Curve(error) => write!(f, "{error}"),
            Self::InvalidDmax { dmax } => {
                write!(f, "scan Dmax must be finite and positive, but was {dmax}")
            }
            Self::InvalidStepSize { step_size } => {
                write!(
                    f,
                    "truncation scan step size must be at least 1, but was {step_size}"
                )
            }
            Self::InvalidPointCount { point_count } => write!(
                f,
                "truncation scan must use at least 1 point, but received {point_count}"
            ),
        }
    }
}

impl Error for TruncationScanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Basis(error) => Some(error),
            Self::Transform(error) => Some(error),
            Self::Solver(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::Curve(error) => Some(error),
            Self::InvalidDmax { .. }
            | Self::InvalidStepSize { .. }
            | Self::InvalidPointCount { .. } => None,
        }
    }
}

impl From<BasisError> for TruncationScanError {
    fn from(value: BasisError) -> Self {
        Self::Basis(value)
    }
}

impl From<TransformError> for TruncationScanError {
    fn from(value: TransformError) -> Self {
        Self::Transform(value)
    }
}

impl From<SolverError> for TruncationScanError {
    fn from(value: SolverError) -> Self {
        Self::Solver(value)
    }
}

impl From<AnalysisError> for TruncationScanError {
    fn from(value: AnalysisError) -> Self {
        Self::Analysis(value)
    }
}

impl From<ParseCurveError> for TruncationScanError {
    fn from(value: ParseCurveError) -> Self {
        Self::Curve(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncationQualityFlag {
    HighReducedChiSquare,
    NegativePrExcursion,
}

#[derive(Debug, Clone)]
pub enum TruncationScanOutcome {
    Successful {
        fit: FitResult,
        summary: FitSummary,
        quality_flags: Vec<TruncationQualityFlag>,
    },
    Failed {
        error_message: String,
    },
}

#[derive(Debug, Clone)]
pub struct TruncationScanEntry {
    pub dropped_point_count: usize,
    pub minimum_retained_q: Option<f64>,
    pub outcome: TruncationScanOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TruncationScanStabilitySummary {
    pub attempted_entry_count: usize,
    pub successful_entry_count: usize,
    pub failed_entry_count: usize,
    pub flagged_entry_count: usize,
    pub dropped_point_range: DmaxScanMetricRange,
    pub q_min_range: Option<DmaxScanMetricRange>,
    pub i_zero_range: Option<DmaxScanMetricRange>,
    pub radius_of_gyration_range: Option<DmaxScanMetricRange>,
    pub chi_square_range: Option<DmaxScanMetricRange>,
    pub objective_value_range: Option<DmaxScanMetricRange>,
}

#[derive(Debug, Clone)]
pub struct TruncationScanResult {
    pub entries: Vec<TruncationScanEntry>,
    pub stability_summary: TruncationScanStabilitySummary,
}

pub fn run_truncation_scan(
    curve: &SaxsCurve,
    config: &TruncationScanConfig,
) -> Result<TruncationScanResult, TruncationScanError> {
    validate_config(config)?;

    let basis = CubicBSplineBasis::new(config.dmax, config.basis_size)?;
    let transform = ForwardTransform::new(basis, config.integration_intervals)?;

    let mut entries = Vec::new();
    for dropped_point_count in build_drop_counts(config) {
        let truncated_curve = match curve.truncate_front(dropped_point_count) {
            Ok(truncated) => truncated,
            Err(error) => {
                entries.push(TruncationScanEntry {
                    dropped_point_count,
                    minimum_retained_q: None,
                    outcome: TruncationScanOutcome::Failed {
                        error_message: error.to_string(),
                    },
                });
                continue;
            }
        };

        let minimum_retained_q = truncated_curve.points().first().map(|point| point.q);

        let outcome = match solve_curve(&truncated_curve, &transform, config.lambda) {
            Ok(fit) => match summarize_fit(
                &truncated_curve,
                &transform,
                &fit,
                config.pr_sample_point_count,
            ) {
                Ok(summary) => TruncationScanOutcome::Successful {
                    quality_flags: assess_truncation_quality(&summary),
                    fit,
                    summary,
                },
                Err(error) => TruncationScanOutcome::Failed {
                    error_message: error.to_string(),
                },
            },
            Err(error) => TruncationScanOutcome::Failed {
                error_message: error.to_string(),
            },
        };

        entries.push(TruncationScanEntry {
            dropped_point_count,
            minimum_retained_q,
            outcome,
        });
    }

    let successful_entries = entries
        .iter()
        .filter_map(|entry| match &entry.outcome {
            TruncationScanOutcome::Successful { summary, .. } => Some((entry, summary)),
            TruncationScanOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();

    let stability_summary = TruncationScanStabilitySummary {
        attempted_entry_count: entries.len(),
        successful_entry_count: successful_entries.len(),
        failed_entry_count: entries.len() - successful_entries.len(),
        flagged_entry_count: entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.outcome,
                    TruncationScanOutcome::Successful {
                        ref quality_flags,
                        ..
                    } if !quality_flags.is_empty()
                )
            })
            .count(),
        dropped_point_range: metric_range(
            entries.iter().map(|entry| entry.dropped_point_count as f64),
        ),
        q_min_range: optional_metric_range(
            successful_entries
                .iter()
                .filter_map(|(entry, _)| entry.minimum_retained_q),
        ),
        i_zero_range: optional_metric_range(
            successful_entries.iter().map(|(_, summary)| summary.i_zero),
        ),
        radius_of_gyration_range: optional_metric_range(
            successful_entries
                .iter()
                .map(|(_, summary)| summary.radius_of_gyration),
        ),
        chi_square_range: optional_metric_range(
            successful_entries
                .iter()
                .map(|(_, summary)| summary.chi_square),
        ),
        objective_value_range: optional_metric_range(
            successful_entries
                .iter()
                .map(|(_, summary)| summary.objective_value),
        ),
    };

    Ok(TruncationScanResult {
        entries,
        stability_summary,
    })
}

fn validate_config(config: &TruncationScanConfig) -> Result<(), TruncationScanError> {
    if !config.dmax.is_finite() || config.dmax <= 0.0 {
        return Err(TruncationScanError::InvalidDmax { dmax: config.dmax });
    }

    if config.step_size == 0 {
        return Err(TruncationScanError::InvalidStepSize {
            step_size: config.step_size,
        });
    }

    if config.point_count == 0 {
        return Err(TruncationScanError::InvalidPointCount {
            point_count: config.point_count,
        });
    }

    Ok(())
}

fn build_drop_counts(config: &TruncationScanConfig) -> Vec<usize> {
    if config.point_count == 1 {
        return vec![config.baseline_drop_count];
    }

    let half_span = (config.point_count - 1) / 2;
    let mut counts = Vec::with_capacity(config.point_count);

    for index in 0..config.point_count {
        let offset_steps = index as isize - half_span as isize;
        let signed_drop =
            config.baseline_drop_count as isize + offset_steps * config.step_size as isize;
        counts.push(signed_drop.max(0) as usize);
    }

    counts
}

fn assess_truncation_quality(summary: &FitSummary) -> Vec<TruncationQualityFlag> {
    let mut flags = Vec::new();

    if summary.reduced_chi_square.unwrap_or(summary.chi_square) > 5.0 {
        flags.push(TruncationQualityFlag::HighReducedChiSquare);
    }

    let max_positive = summary
        .sampled_pr
        .iter()
        .map(|point| point.p_of_r)
        .fold(0.0_f64, |current, value| current.max(value));
    let min_value = summary
        .sampled_pr
        .iter()
        .map(|point| point.p_of_r)
        .fold(f64::INFINITY, |current, value| current.min(value));

    if max_positive > 0.0 && min_value < -0.05 * max_positive {
        flags.push(TruncationQualityFlag::NegativePrExcursion);
    }

    flags
}

fn metric_range(values: impl Iterator<Item = f64>) -> DmaxScanMetricRange {
    let collected = values.collect::<Vec<_>>();
    let min = collected
        .iter()
        .copied()
        .fold(f64::INFINITY, |left, right| left.min(right));
    let max = collected
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |left, right| left.max(right));

    DmaxScanMetricRange {
        min,
        max,
        span: max - min,
    }
}

fn optional_metric_range(values: impl Iterator<Item = f64>) -> Option<DmaxScanMetricRange> {
    let collected = values.collect::<Vec<_>>();
    if collected.is_empty() {
        None
    } else {
        Some(metric_range(collected.into_iter()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TruncationQualityFlag, TruncationScanConfig, TruncationScanError, TruncationScanOutcome,
        run_truncation_scan,
    };
    use crate::basis::CubicBSplineBasis;
    use crate::data::{SaxsCurve, SaxsPoint};
    use crate::transform::ForwardTransform;

    fn synthetic_curve_from_coefficients(coefficients: &[f64], dmax: f64) -> SaxsCurve {
        let basis = CubicBSplineBasis::new(dmax, coefficients.len()).unwrap();
        let transform = ForwardTransform::new(basis, 800).unwrap();
        let q_values = (0..40)
            .map(|index| 0.01 + index as f64 * 0.03)
            .collect::<Vec<_>>();
        let intensities = transform.predict(&q_values, coefficients).unwrap();

        SaxsCurve::new(
            q_values
                .iter()
                .zip(intensities.iter())
                .map(|(&q, &intensity)| SaxsPoint {
                    q,
                    intensity,
                    sigma: 0.05,
                })
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn runs_local_truncation_scan_and_collects_entries() {
        let curve = synthetic_curve_from_coefficients(&[1.0; 6], 8.0);
        let config = TruncationScanConfig {
            dmax: 8.0,
            baseline_drop_count: 10,
            step_size: 5,
            point_count: 5,
            basis_size: 6,
            integration_intervals: 400,
            lambda: 1.0e-8,
            pr_sample_point_count: 41,
        };

        let scan = run_truncation_scan(&curve, &config).unwrap();

        assert_eq!(scan.entries.len(), 5);
        assert_eq!(scan.entries[0].dropped_point_count, 0);
        assert_eq!(scan.entries[4].dropped_point_count, 20);
        assert_eq!(scan.stability_summary.attempted_entry_count, 5);
    }

    #[test]
    fn preserves_failed_nearby_truncation_results() {
        let curve = synthetic_curve_from_coefficients(&[1.0; 6], 8.0);
        let config = TruncationScanConfig {
            dmax: 8.0,
            baseline_drop_count: 38,
            step_size: 5,
            point_count: 3,
            basis_size: 6,
            integration_intervals: 400,
            lambda: 1.0e-8,
            pr_sample_point_count: 41,
        };

        let scan = run_truncation_scan(&curve, &config).unwrap();

        assert!(
            scan.entries
                .iter()
                .any(|entry| matches!(entry.outcome, TruncationScanOutcome::Failed { .. }))
        );
        assert!(scan.stability_summary.failed_entry_count >= 1);
    }

    #[test]
    fn rejects_invalid_truncation_scan_config() {
        let curve = synthetic_curve_from_coefficients(&[1.0; 6], 8.0);
        let config = TruncationScanConfig {
            dmax: 8.0,
            baseline_drop_count: 10,
            step_size: 0,
            point_count: 5,
            basis_size: 6,
            integration_intervals: 400,
            lambda: 1.0e-8,
            pr_sample_point_count: 41,
        };

        let error = run_truncation_scan(&curve, &config).unwrap_err();
        assert!(matches!(error, TruncationScanError::InvalidStepSize { .. }));
    }

    #[test]
    fn flags_suspicious_negative_pr_excursions() {
        let curve = synthetic_curve_from_coefficients(&[1.0; 6], 8.0);
        let config = TruncationScanConfig {
            dmax: 8.0,
            baseline_drop_count: 0,
            step_size: 5,
            point_count: 1,
            basis_size: 6,
            integration_intervals: 400,
            lambda: 100.0,
            pr_sample_point_count: 41,
        };

        let scan = run_truncation_scan(&curve, &config).unwrap();
        let flags = match &scan.entries[0].outcome {
            TruncationScanOutcome::Successful { quality_flags, .. } => quality_flags,
            TruncationScanOutcome::Failed { .. } => panic!("expected successful scan entry"),
        };

        assert!(flags.iter().all(|flag| matches!(
            flag,
            TruncationQualityFlag::HighReducedChiSquare
                | TruncationQualityFlag::NegativePrExcursion
        )));
    }
}
