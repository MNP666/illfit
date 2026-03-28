use std::error::Error;
use std::fmt;

use crate::analysis::{AnalysisError, FitSummary, summarize_fit};
use crate::basis::{BasisError, CubicBSplineBasis};
use crate::data::SaxsCurve;
use crate::solver::{FitResult, SolverError, solve_curve};
use crate::transform::{ForwardTransform, TransformError};

#[derive(Debug, Clone)]
pub struct DmaxScanConfig {
    pub center_dmax: f64,
    pub half_width: f64,
    pub point_count: usize,
    pub basis_size: usize,
    pub integration_intervals: usize,
    pub lambda: f64,
    pub pr_sample_point_count: usize,
}

#[derive(Debug)]
pub enum DmaxScanError {
    Basis(BasisError),
    Transform(TransformError),
    Solver(SolverError),
    Analysis(AnalysisError),
    InvalidCenterDmax { center_dmax: f64 },
    InvalidHalfWidth { half_width: f64 },
    InvalidPointCount { point_count: usize },
    ScanRangeCrossesZero { minimum_dmax: f64 },
}

impl fmt::Display for DmaxScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basis(error) => write!(f, "{error}"),
            Self::Transform(error) => write!(f, "{error}"),
            Self::Solver(error) => write!(f, "{error}"),
            Self::Analysis(error) => write!(f, "{error}"),
            Self::InvalidCenterDmax { center_dmax } => write!(
                f,
                "center Dmax must be finite and positive, but was {center_dmax}"
            ),
            Self::InvalidHalfWidth { half_width } => write!(
                f,
                "Dmax half-width must be finite and non-negative, but was {half_width}"
            ),
            Self::InvalidPointCount { point_count } => write!(
                f,
                "Dmax scan must use at least 1 point, but received {point_count}"
            ),
            Self::ScanRangeCrossesZero { minimum_dmax } => write!(
                f,
                "Dmax scan range must stay positive, but the minimum scanned value would be {minimum_dmax}"
            ),
        }
    }
}

impl Error for DmaxScanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Basis(error) => Some(error),
            Self::Transform(error) => Some(error),
            Self::Solver(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::InvalidCenterDmax { .. }
            | Self::InvalidHalfWidth { .. }
            | Self::InvalidPointCount { .. }
            | Self::ScanRangeCrossesZero { .. } => None,
        }
    }
}

impl From<BasisError> for DmaxScanError {
    fn from(value: BasisError) -> Self {
        Self::Basis(value)
    }
}

impl From<TransformError> for DmaxScanError {
    fn from(value: TransformError) -> Self {
        Self::Transform(value)
    }
}

impl From<SolverError> for DmaxScanError {
    fn from(value: SolverError) -> Self {
        Self::Solver(value)
    }
}

impl From<AnalysisError> for DmaxScanError {
    fn from(value: AnalysisError) -> Self {
        Self::Analysis(value)
    }
}

#[derive(Debug, Clone)]
pub struct DmaxScanEntry {
    pub dmax: f64,
    pub fit: FitResult,
    pub summary: FitSummary,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DmaxScanMetricRange {
    pub min: f64,
    pub max: f64,
    pub span: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DmaxScanStabilitySummary {
    pub dmax_range: DmaxScanMetricRange,
    pub i_zero_range: DmaxScanMetricRange,
    pub radius_of_gyration_range: DmaxScanMetricRange,
    pub chi_square_range: DmaxScanMetricRange,
    pub objective_value_range: DmaxScanMetricRange,
}

#[derive(Debug, Clone)]
pub struct DmaxScanResult {
    pub entries: Vec<DmaxScanEntry>,
    pub stability_summary: DmaxScanStabilitySummary,
}

pub fn run_dmax_scan(
    curve: &SaxsCurve,
    config: &DmaxScanConfig,
) -> Result<DmaxScanResult, DmaxScanError> {
    validate_config(config)?;
    let dmax_values = build_dmax_values(config);

    let mut entries = Vec::with_capacity(dmax_values.len());

    for dmax in dmax_values {
        let basis = CubicBSplineBasis::new(dmax, config.basis_size)?;
        let transform = ForwardTransform::new(basis, config.integration_intervals)?;
        let fit = solve_curve(curve, &transform, config.lambda)?;
        let summary = summarize_fit(curve, &transform, &fit, config.pr_sample_point_count)?;

        entries.push(DmaxScanEntry { dmax, fit, summary });
    }

    let stability_summary = DmaxScanStabilitySummary {
        dmax_range: metric_range(entries.iter().map(|entry| entry.dmax)),
        i_zero_range: metric_range(entries.iter().map(|entry| entry.summary.i_zero)),
        radius_of_gyration_range: metric_range(
            entries.iter().map(|entry| entry.summary.radius_of_gyration),
        ),
        chi_square_range: metric_range(entries.iter().map(|entry| entry.summary.chi_square)),
        objective_value_range: metric_range(
            entries.iter().map(|entry| entry.summary.objective_value),
        ),
    };

    Ok(DmaxScanResult {
        entries,
        stability_summary,
    })
}

fn validate_config(config: &DmaxScanConfig) -> Result<(), DmaxScanError> {
    if !config.center_dmax.is_finite() || config.center_dmax <= 0.0 {
        return Err(DmaxScanError::InvalidCenterDmax {
            center_dmax: config.center_dmax,
        });
    }

    if !config.half_width.is_finite() || config.half_width < 0.0 {
        return Err(DmaxScanError::InvalidHalfWidth {
            half_width: config.half_width,
        });
    }

    if config.point_count == 0 {
        return Err(DmaxScanError::InvalidPointCount {
            point_count: config.point_count,
        });
    }

    let minimum_dmax = config.center_dmax - config.half_width;
    if minimum_dmax <= 0.0 {
        return Err(DmaxScanError::ScanRangeCrossesZero { minimum_dmax });
    }

    Ok(())
}

fn build_dmax_values(config: &DmaxScanConfig) -> Vec<f64> {
    if config.point_count == 1 {
        return vec![config.center_dmax];
    }

    let minimum = config.center_dmax - config.half_width;
    let maximum = config.center_dmax + config.half_width;
    let spacing = (maximum - minimum) / (config.point_count - 1) as f64;

    (0..config.point_count)
        .map(|index| minimum + index as f64 * spacing)
        .collect()
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

#[cfg(test)]
mod tests {
    use super::{DmaxScanConfig, DmaxScanError, run_dmax_scan};
    use crate::basis::CubicBSplineBasis;
    use crate::data::{SaxsCurve, SaxsPoint};
    use crate::transform::ForwardTransform;

    fn synthetic_curve_from_coefficients(coefficients: &[f64], dmax: f64) -> SaxsCurve {
        let basis = CubicBSplineBasis::new(dmax, coefficients.len()).unwrap();
        let transform = ForwardTransform::new(basis, 800).unwrap();
        let q_values = (0..18)
            .map(|index| 0.05 + index as f64 * 0.08)
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
    fn runs_local_dmax_scan_and_collects_entries() {
        let curve = synthetic_curve_from_coefficients(&[1.0; 6], 8.0);
        let config = DmaxScanConfig {
            center_dmax: 8.0,
            half_width: 1.0,
            point_count: 5,
            basis_size: 6,
            integration_intervals: 400,
            lambda: 1.0e-8,
            pr_sample_point_count: 41,
        };

        let scan = run_dmax_scan(&curve, &config).unwrap();

        assert_eq!(scan.entries.len(), 5);
        assert_eq!(scan.entries[0].dmax, 7.0);
        assert_eq!(scan.entries[4].dmax, 9.0);
        assert!(scan.stability_summary.radius_of_gyration_range.span >= 0.0);
    }

    #[test]
    fn supports_single_point_scan() {
        let curve = synthetic_curve_from_coefficients(&[1.0; 6], 8.0);
        let config = DmaxScanConfig {
            center_dmax: 8.0,
            half_width: 0.0,
            point_count: 1,
            basis_size: 6,
            integration_intervals: 400,
            lambda: 1.0e-8,
            pr_sample_point_count: 41,
        };

        let scan = run_dmax_scan(&curve, &config).unwrap();

        assert_eq!(scan.entries.len(), 1);
        assert_eq!(scan.stability_summary.dmax_range.span, 0.0);
    }

    #[test]
    fn rejects_invalid_scan_config() {
        let curve = synthetic_curve_from_coefficients(&[1.0; 6], 8.0);
        let config = DmaxScanConfig {
            center_dmax: 0.0,
            half_width: 1.0,
            point_count: 5,
            basis_size: 6,
            integration_intervals: 400,
            lambda: 1.0e-8,
            pr_sample_point_count: 41,
        };

        let error = run_dmax_scan(&curve, &config).unwrap_err();
        assert!(matches!(error, DmaxScanError::InvalidCenterDmax { .. }));
    }

    #[test]
    fn rejects_scan_ranges_that_cross_zero() {
        let curve = synthetic_curve_from_coefficients(&[1.0; 6], 8.0);
        let config = DmaxScanConfig {
            center_dmax: 1.0,
            half_width: 1.5,
            point_count: 5,
            basis_size: 6,
            integration_intervals: 400,
            lambda: 1.0e-8,
            pr_sample_point_count: 41,
        };

        let error = run_dmax_scan(&curve, &config).unwrap_err();
        assert!(matches!(error, DmaxScanError::ScanRangeCrossesZero { .. }));
    }
}
