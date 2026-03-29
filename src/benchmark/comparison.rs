use crate::benchmark::{BenchmarkRecoveryResult, BenchmarkSuiteRecoveryResult};

/// One residual point in `r` space between recovered and true `P(r)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchmarkPrResidualPoint {
    pub r: f64,
    pub true_p_of_r: f64,
    pub recovered_p_of_r: f64,
    pub residual: f64,
}

/// One residual point in `q` space between recovered and true `I(q)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchmarkIqResidualPoint {
    pub q: f64,
    pub true_intensity: f64,
    pub recovered_intensity: f64,
    pub residual: f64,
}

/// Summary metrics comparing true and recovered `P(r)`.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkPrComparison {
    pub residual_curve: Vec<BenchmarkPrResidualPoint>,
    pub rmse: f64,
    pub normalized_rmse: f64,
    pub correlation: f64,
    pub integrated_absolute_error: f64,
    pub radius_of_gyration_error: f64,
    pub i_zero_error: f64,
}

/// Summary metrics comparing true and recovered `I(q)`.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkIqComparison {
    pub residual_curve: Vec<BenchmarkIqResidualPoint>,
    pub rmse: f64,
    pub normalized_rmse: f64,
}

/// Comparison bundle for one recovered benchmark case.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkRecoveryComparison {
    pub case_id: String,
    pub pr: BenchmarkPrComparison,
    pub iq: BenchmarkIqComparison,
}

/// Comparison bundle for every recovered case in one benchmark suite.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkSuiteComparison {
    pub suite_name: String,
    pub case_comparisons: Vec<BenchmarkRecoveryComparison>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BenchmarkComparisonError {
    MissingRecoveredPrSamples,
    MissingRecoveredIqSamples,
}

impl std::fmt::Display for BenchmarkComparisonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRecoveredPrSamples => {
                write!(
                    f,
                    "recovered fit summary does not contain sampled P(r) points"
                )
            }
            Self::MissingRecoveredIqSamples => {
                write!(
                    f,
                    "recovered fit result does not contain predicted I(q) values"
                )
            }
        }
    }
}

impl std::error::Error for BenchmarkComparisonError {}

pub fn compare_benchmark_recovery(
    recovery: &BenchmarkRecoveryResult,
) -> Result<BenchmarkRecoveryComparison, BenchmarkComparisonError> {
    if recovery.summary.sampled_pr.is_empty() {
        return Err(BenchmarkComparisonError::MissingRecoveredPrSamples);
    }
    if recovery.fit.predicted_intensities.is_empty() {
        return Err(BenchmarkComparisonError::MissingRecoveredIqSamples);
    }

    let pr = compare_pr(recovery);
    let iq = compare_iq(recovery);

    Ok(BenchmarkRecoveryComparison {
        case_id: recovery.truth_case.metadata.candidate_id.clone(),
        pr,
        iq,
    })
}

pub fn compare_benchmark_suite(
    suite_recovery: &BenchmarkSuiteRecoveryResult,
) -> Result<BenchmarkSuiteComparison, BenchmarkComparisonError> {
    let case_comparisons = suite_recovery
        .case_results
        .iter()
        .map(compare_benchmark_recovery)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(BenchmarkSuiteComparison {
        suite_name: suite_recovery.suite.summary.suite_name.clone(),
        case_comparisons,
    })
}

fn compare_pr(recovery: &BenchmarkRecoveryResult) -> BenchmarkPrComparison {
    let recovered_r = recovery
        .summary
        .sampled_pr
        .iter()
        .map(|point| point.r)
        .collect::<Vec<_>>();
    let recovered_pr = recovery
        .summary
        .sampled_pr
        .iter()
        .map(|point| point.p_of_r)
        .collect::<Vec<_>>();

    let residual_curve = recovery
        .truth_case
        .pr_truth
        .points()
        .iter()
        .map(|point| {
            let recovered = interpolate_linear(&recovered_r, &recovered_pr, point.r);
            BenchmarkPrResidualPoint {
                r: point.r,
                true_p_of_r: point.p_of_r,
                recovered_p_of_r: recovered,
                residual: recovered - point.p_of_r,
            }
        })
        .collect::<Vec<_>>();

    let residuals = residual_curve
        .iter()
        .map(|point| point.residual)
        .collect::<Vec<_>>();
    let true_values = residual_curve
        .iter()
        .map(|point| point.true_p_of_r)
        .collect::<Vec<_>>();
    let recovered_values = residual_curve
        .iter()
        .map(|point| point.recovered_p_of_r)
        .collect::<Vec<_>>();

    let rmse = rmse(&residuals);
    let normalized_rmse = normalize_rmse(rmse, &true_values);
    let correlation = correlation(&true_values, &recovered_values);
    let integrated_absolute_error = trapezoid_integral_abs(&residual_curve);
    let radius_of_gyration_error =
        recovery.summary.radius_of_gyration - recovery.truth_case.metadata.rg;
    let i_zero_error = recovery.summary.i_zero - recovery.truth_case.metadata.i_zero;

    BenchmarkPrComparison {
        residual_curve,
        rmse,
        normalized_rmse,
        correlation,
        integrated_absolute_error,
        radius_of_gyration_error,
        i_zero_error,
    }
}

fn compare_iq(recovery: &BenchmarkRecoveryResult) -> BenchmarkIqComparison {
    let residual_curve = recovery
        .truth_case
        .iq_truth
        .points()
        .iter()
        .zip(recovery.fit.predicted_intensities.iter())
        .map(|(truth_point, &recovered)| BenchmarkIqResidualPoint {
            q: truth_point.q,
            true_intensity: truth_point.intensity,
            recovered_intensity: recovered,
            residual: recovered - truth_point.intensity,
        })
        .collect::<Vec<_>>();

    let residuals = residual_curve
        .iter()
        .map(|point| point.residual)
        .collect::<Vec<_>>();
    let true_values = residual_curve
        .iter()
        .map(|point| point.true_intensity)
        .collect::<Vec<_>>();

    let rmse = rmse(&residuals);
    let normalized_rmse = normalize_rmse(rmse, &true_values);

    BenchmarkIqComparison {
        residual_curve,
        rmse,
        normalized_rmse,
    }
}

fn interpolate_linear(x: &[f64], y: &[f64], target: f64) -> f64 {
    if target <= x[0] {
        return y[0];
    }
    if target >= x[x.len() - 1] {
        return y[y.len() - 1];
    }

    let upper_index = x.partition_point(|&value| value < target);
    let lower_index = upper_index - 1;
    let x0 = x[lower_index];
    let x1 = x[upper_index];
    let y0 = y[lower_index];
    let y1 = y[upper_index];

    if x1 == x0 {
        y0
    } else {
        let fraction = (target - x0) / (x1 - x0);
        y0 + fraction * (y1 - y0)
    }
}

fn rmse(residuals: &[f64]) -> f64 {
    if residuals.is_empty() {
        return 0.0;
    }

    let mean_square =
        residuals.iter().map(|value| value * value).sum::<f64>() / residuals.len() as f64;
    mean_square.sqrt()
}

fn normalize_rmse(rmse: f64, values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let min_value = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max_value = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = max_value - min_value;

    if range > 0.0 { rmse / range } else { 0.0 }
}

fn correlation(left: &[f64], right: &[f64]) -> f64 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }

    let mean_left = left.iter().sum::<f64>() / left.len() as f64;
    let mean_right = right.iter().sum::<f64>() / right.len() as f64;

    let mut numerator = 0.0;
    let mut sum_sq_left = 0.0;
    let mut sum_sq_right = 0.0;

    for (&left_value, &right_value) in left.iter().zip(right.iter()) {
        let centered_left = left_value - mean_left;
        let centered_right = right_value - mean_right;
        numerator += centered_left * centered_right;
        sum_sq_left += centered_left * centered_left;
        sum_sq_right += centered_right * centered_right;
    }

    let denominator = (sum_sq_left * sum_sq_right).sqrt();
    if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}

fn trapezoid_integral_abs(points: &[BenchmarkPrResidualPoint]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }

    let mut integral = 0.0;
    for window in points.windows(2) {
        let left = &window[0];
        let right = &window[1];
        let delta_r = right.r - left.r;
        let average_abs = 0.5 * (left.residual.abs() + right.residual.abs());
        integral += average_abs * delta_r;
    }
    integral
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkComparisonError, compare_benchmark_recovery, compare_benchmark_suite, correlation,
    };
    use crate::analysis::FitSummary;
    use crate::basis::CubicBSplineBasis;
    use crate::benchmark::{
        BenchmarkRecoveryConfig, BenchmarkRecoveryResult, load_benchmark_suite,
        recover_benchmark_suite, recover_benchmark_truth_case,
    };
    use crate::solver::FitResult;
    use crate::transform::ForwardTransform;
    use std::path::PathBuf;

    fn synthetic_suite_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("synthetic")
            .join("clamped_spline_seed42")
    }

    #[test]
    fn compares_one_recovered_case() {
        let suite = load_benchmark_suite(synthetic_suite_path()).unwrap();
        let recovery = recover_benchmark_truth_case(
            &suite.truth_cases[0],
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

        let comparison = compare_benchmark_recovery(&recovery).unwrap();

        assert_eq!(
            comparison.case_id,
            recovery.truth_case.metadata.candidate_id
        );
        assert_eq!(
            comparison.pr.residual_curve.len(),
            recovery.truth_case.pr_truth.points().len()
        );
        assert_eq!(
            comparison.iq.residual_curve.len(),
            recovery.truth_case.iq_truth.points().len()
        );
        assert!(comparison.pr.rmse >= 0.0);
        assert!(comparison.iq.rmse >= 0.0);
    }

    #[test]
    fn compares_every_case_in_one_suite() {
        let suite = load_benchmark_suite(synthetic_suite_path()).unwrap();
        let recovery = recover_benchmark_suite(
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

        let comparison = compare_benchmark_suite(&recovery).unwrap();

        assert_eq!(comparison.suite_name, suite.summary.suite_name);
        assert_eq!(comparison.case_comparisons.len(), suite.truth_cases.len());
    }

    #[test]
    fn rejects_missing_recovered_pr_samples() {
        let suite = load_benchmark_suite(synthetic_suite_path()).unwrap();
        let truth_case = suite.truth_cases[0].clone();
        let observed_curve = crate::data::SaxsCurve::new(
            truth_case
                .iq_truth
                .points()
                .iter()
                .map(|point| crate::data::SaxsPoint {
                    q: point.q,
                    intensity: point.intensity,
                    sigma: 0.05,
                })
                .collect(),
        )
        .unwrap();
        let transform = ForwardTransform::new(
            CubicBSplineBasis::new(
                suite.summary.config.dmax,
                suite.summary.config.n_weights + 2,
            )
            .unwrap(),
            suite.summary.config.integration_intervals,
        )
        .unwrap();
        let recovery = BenchmarkRecoveryResult {
            truth_case,
            observed_curve,
            transform,
            fit: FitResult {
                coefficients: vec![0.0; suite.summary.config.n_weights + 2],
                predicted_intensities: vec![0.0; suite.summary.config.q_points],
                residuals: vec![0.0; suite.summary.config.q_points],
                weighted_residual_sum_squares: 0.0,
                regularization_penalty: 0.0,
                objective_value: 0.0,
                lambda: 0.0,
            },
            summary: FitSummary {
                sampled_pr: Vec::new(),
                i_zero: 0.0,
                radius_of_gyration: 0.0,
                chi_square: 0.0,
                reduced_chi_square: None,
                weighted_residual_sum_squares: 0.0,
                regularization_penalty: 0.0,
                objective_value: 0.0,
            },
        };

        let error = compare_benchmark_recovery(&recovery).unwrap_err();
        assert_eq!(error, BenchmarkComparisonError::MissingRecoveredPrSamples);
    }

    #[test]
    fn reports_expected_correlation_for_identical_vectors() {
        let values = [1.0, 2.0, 3.0, 4.0];
        assert!((correlation(&values, &values) - 1.0).abs() < 1.0e-12);
    }
}
