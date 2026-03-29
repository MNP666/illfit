use std::error::Error;
use std::fmt;

use crate::analysis::{AnalysisError, FitSummary, summarize_fit};
use crate::basis::{BasisError, CubicBSplineBasis};
use crate::benchmark::{BenchmarkSuite, BenchmarkTruthCase};
use crate::data::{ParseCurveError, SaxsCurve, SaxsPoint};
use crate::solver::{FitResult, SolverError, solve_curve};
use crate::transform::{ForwardTransform, TransformError};

/// Configuration for running the existing iFT recovery pipeline on synthetic
/// benchmark truth cases.
///
/// The benchmark truth assets store noiseless `I(q)` values but do not yet
/// store uncertainties. For this first recovery path, we assign one uniform
/// synthetic sigma to every sampled `q` point so the existing weighted solver
/// can be reused without introducing a more elaborate uncertainty model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchmarkRecoveryConfig {
    pub dmax: f64,
    pub basis_size: usize,
    pub integration_intervals: usize,
    pub lambda: f64,
    pub pr_sample_points: usize,
    pub synthetic_sigma: f64,
}

/// Result of recovering one synthetic truth case.
#[derive(Debug, Clone)]
pub struct BenchmarkRecoveryResult {
    pub truth_case: BenchmarkTruthCase,
    pub observed_curve: SaxsCurve,
    pub transform: ForwardTransform,
    pub fit: FitResult,
    pub summary: FitSummary,
}

/// Result of recovering every accepted truth case in one suite.
#[derive(Debug, Clone)]
pub struct BenchmarkSuiteRecoveryResult {
    pub suite: BenchmarkSuite,
    pub case_results: Vec<BenchmarkRecoveryResult>,
}

#[derive(Debug)]
pub enum BenchmarkRecoveryError {
    InvalidSyntheticSigma { synthetic_sigma: f64 },
    Basis(BasisError),
    Curve(ParseCurveError),
    Transform(TransformError),
    Solver(SolverError),
    Analysis(AnalysisError),
}

impl fmt::Display for BenchmarkRecoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSyntheticSigma { synthetic_sigma } => write!(
                f,
                "synthetic sigma must be finite and positive, but was {synthetic_sigma}"
            ),
            Self::Basis(error) => write!(f, "{error}"),
            Self::Curve(error) => write!(f, "{error}"),
            Self::Transform(error) => write!(f, "{error}"),
            Self::Solver(error) => write!(f, "{error}"),
            Self::Analysis(error) => write!(f, "{error}"),
        }
    }
}

impl Error for BenchmarkRecoveryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Basis(error) => Some(error),
            Self::Curve(error) => Some(error),
            Self::Transform(error) => Some(error),
            Self::Solver(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::InvalidSyntheticSigma { .. } => None,
        }
    }
}

impl From<BasisError> for BenchmarkRecoveryError {
    fn from(value: BasisError) -> Self {
        Self::Basis(value)
    }
}

impl From<ParseCurveError> for BenchmarkRecoveryError {
    fn from(value: ParseCurveError) -> Self {
        Self::Curve(value)
    }
}

impl From<TransformError> for BenchmarkRecoveryError {
    fn from(value: TransformError) -> Self {
        Self::Transform(value)
    }
}

impl From<SolverError> for BenchmarkRecoveryError {
    fn from(value: SolverError) -> Self {
        Self::Solver(value)
    }
}

impl From<AnalysisError> for BenchmarkRecoveryError {
    fn from(value: AnalysisError) -> Self {
        Self::Analysis(value)
    }
}

pub fn recover_benchmark_truth_case(
    truth_case: &BenchmarkTruthCase,
    config: BenchmarkRecoveryConfig,
) -> Result<BenchmarkRecoveryResult, BenchmarkRecoveryError> {
    validate_recovery_config(config)?;

    let observed_curve = synthetic_observed_curve(truth_case, config.synthetic_sigma)?;
    recover_benchmark_observed_case(truth_case, observed_curve, config)
}

pub(crate) fn recover_benchmark_observed_case(
    truth_case: &BenchmarkTruthCase,
    observed_curve: SaxsCurve,
    config: BenchmarkRecoveryConfig,
) -> Result<BenchmarkRecoveryResult, BenchmarkRecoveryError> {
    let basis = CubicBSplineBasis::new(config.dmax, config.basis_size)?;
    let transform = ForwardTransform::new(basis, config.integration_intervals)?;
    let fit = solve_curve(&observed_curve, &transform, config.lambda)?;
    let summary = summarize_fit(&observed_curve, &transform, &fit, config.pr_sample_points)?;

    Ok(BenchmarkRecoveryResult {
        truth_case: truth_case.clone(),
        observed_curve,
        transform,
        fit,
        summary,
    })
}

pub fn recover_benchmark_suite(
    suite: &BenchmarkSuite,
    config: BenchmarkRecoveryConfig,
) -> Result<BenchmarkSuiteRecoveryResult, BenchmarkRecoveryError> {
    validate_recovery_config(config)?;

    let case_results = suite
        .truth_cases
        .iter()
        .map(|truth_case| recover_benchmark_truth_case(truth_case, config))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(BenchmarkSuiteRecoveryResult {
        suite: suite.clone(),
        case_results,
    })
}

fn validate_recovery_config(config: BenchmarkRecoveryConfig) -> Result<(), BenchmarkRecoveryError> {
    if !config.synthetic_sigma.is_finite() || config.synthetic_sigma <= 0.0 {
        return Err(BenchmarkRecoveryError::InvalidSyntheticSigma {
            synthetic_sigma: config.synthetic_sigma,
        });
    }

    Ok(())
}

fn synthetic_observed_curve(
    truth_case: &BenchmarkTruthCase,
    synthetic_sigma: f64,
) -> Result<SaxsCurve, ParseCurveError> {
    let points = truth_case
        .iq_truth
        .points()
        .iter()
        .map(|point| SaxsPoint {
            q: point.q,
            intensity: point.intensity,
            sigma: synthetic_sigma,
        })
        .collect::<Vec<_>>();

    SaxsCurve::new(points)
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkRecoveryConfig, BenchmarkRecoveryError, recover_benchmark_suite,
        recover_benchmark_truth_case,
    };
    use crate::benchmark::load_benchmark_suite;
    use std::path::PathBuf;

    fn synthetic_suite_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("regression")
            .join("clamped_spline")
    }

    #[test]
    fn rejects_invalid_synthetic_sigma() {
        let suite = load_benchmark_suite(synthetic_suite_path()).unwrap();
        let error = recover_benchmark_truth_case(
            &suite.truth_cases[0],
            BenchmarkRecoveryConfig {
                dmax: suite.summary.config.dmax,
                basis_size: suite.summary.config.n_weights + 2,
                integration_intervals: suite.summary.config.integration_intervals,
                lambda: 1.0e-2,
                pr_sample_points: suite.summary.config.r_points,
                synthetic_sigma: 0.0,
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            BenchmarkRecoveryError::InvalidSyntheticSigma {
                synthetic_sigma: 0.0
            }
        ));
    }

    #[test]
    fn recovers_one_truth_case_with_existing_fit_pipeline() {
        let suite = load_benchmark_suite(synthetic_suite_path()).unwrap();
        let truth_case = &suite.truth_cases[0];

        let result = recover_benchmark_truth_case(
            truth_case,
            BenchmarkRecoveryConfig {
                dmax: suite.summary.config.dmax,
                basis_size: suite.summary.config.n_weights + 2,
                integration_intervals: suite.summary.config.integration_intervals,
                lambda: 1.0e-2,
                pr_sample_points: suite.summary.config.r_points,
                synthetic_sigma: 0.05,
            },
        )
        .unwrap();

        assert_eq!(
            result.observed_curve.len(),
            truth_case.iq_truth.points().len()
        );
        assert_eq!(
            result.fit.predicted_intensities.len(),
            truth_case.iq_truth.points().len()
        );
        assert_eq!(
            result.summary.sampled_pr.len(),
            suite.summary.config.r_points
        );
        assert!(result.summary.i_zero > 0.0);
    }

    #[test]
    fn recovers_every_case_in_one_suite() {
        let suite = load_benchmark_suite(synthetic_suite_path()).unwrap();

        let result = recover_benchmark_suite(
            &suite,
            BenchmarkRecoveryConfig {
                dmax: suite.summary.config.dmax,
                basis_size: suite.summary.config.n_weights + 2,
                integration_intervals: suite.summary.config.integration_intervals,
                lambda: 1.0e-2,
                pr_sample_points: suite.summary.config.r_points,
                synthetic_sigma: 0.05,
            },
        )
        .unwrap();

        assert_eq!(result.case_results.len(), suite.truth_cases.len());
    }

    #[test]
    fn regression_suite_has_expected_case_count() {
        let suite = load_benchmark_suite(synthetic_suite_path()).unwrap();

        assert_eq!(suite.summary.accepted_count, 12);
        assert_eq!(suite.truth_cases.len(), 12);
    }
}
