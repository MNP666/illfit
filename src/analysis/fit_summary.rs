use std::error::Error;
use std::fmt;

use crate::solver::FitResult;
use crate::transform::{ForwardTransform, TransformError};
use crate::{basis::BasisError, data::SaxsCurve};

#[derive(Debug)]
pub enum AnalysisError {
    Transform(TransformError),
    Basis(BasisError),
    InvalidSamplePointCount { sample_point_count: usize },
    NonPositiveI0 { i0: f64 },
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transform(error) => write!(f, "{error}"),
            Self::Basis(error) => write!(f, "{error}"),
            Self::InvalidSamplePointCount { sample_point_count } => write!(
                f,
                "sample point count must be at least 2, but was {sample_point_count}"
            ),
            Self::NonPositiveI0 { i0 } => write!(
                f,
                "I(0) must be positive to compute Rg, but the fitted integral was {i0}"
            ),
        }
    }
}

impl Error for AnalysisError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transform(error) => Some(error),
            Self::Basis(error) => Some(error),
            Self::InvalidSamplePointCount { .. } | Self::NonPositiveI0 { .. } => None,
        }
    }
}

impl From<TransformError> for AnalysisError {
    fn from(value: TransformError) -> Self {
        Self::Transform(value)
    }
}

impl From<BasisError> for AnalysisError {
    fn from(value: BasisError) -> Self {
        Self::Basis(value)
    }
}

/// One sampled point from the fitted `P(r)` curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampledPrPoint {
    pub r: f64,
    pub p_of_r: f64,
}

/// Scientist-facing summary of a fit.
#[derive(Debug, Clone)]
pub struct FitSummary {
    pub sampled_pr: Vec<SampledPrPoint>,
    pub i_zero: f64,
    pub radius_of_gyration: f64,
    pub chi_square: f64,
    pub reduced_chi_square: Option<f64>,
    pub weighted_residual_sum_squares: f64,
    pub regularization_penalty: f64,
    pub objective_value: f64,
}

/// Build derived quantities from a solved fit.
pub fn summarize_fit(
    curve: &SaxsCurve,
    transform: &ForwardTransform,
    fit: &FitResult,
    sample_point_count: usize,
) -> Result<FitSummary, AnalysisError> {
    if sample_point_count < 2 {
        return Err(AnalysisError::InvalidSamplePointCount { sample_point_count });
    }

    let sampled_pr = sample_pr_curve(transform, &fit.coefficients, sample_point_count)?;
    let (i_zero, second_moment) = pr_moments(transform, &fit.coefficients)?;

    if i_zero <= 0.0 {
        return Err(AnalysisError::NonPositiveI0 { i0: i_zero });
    }

    let radius_of_gyration = (second_moment / (2.0 * i_zero)).sqrt();
    let chi_square = fit.weighted_residual_sum_squares / curve.len() as f64;
    let degrees_of_freedom = curve.len() as isize - fit.coefficients.len() as isize;
    let reduced_chi_square = if degrees_of_freedom > 0 {
        Some(fit.weighted_residual_sum_squares / degrees_of_freedom as f64)
    } else {
        None
    };

    Ok(FitSummary {
        sampled_pr,
        i_zero,
        radius_of_gyration,
        chi_square,
        reduced_chi_square,
        weighted_residual_sum_squares: fit.weighted_residual_sum_squares,
        regularization_penalty: fit.regularization_penalty,
        objective_value: fit.objective_value,
    })
}

fn sample_pr_curve(
    transform: &ForwardTransform,
    coefficients: &[f64],
    sample_point_count: usize,
) -> Result<Vec<SampledPrPoint>, AnalysisError> {
    let dmax = transform.basis().dmax();
    let spacing = dmax / (sample_point_count - 1) as f64;
    let mut sampled = Vec::with_capacity(sample_point_count);

    for sample_index in 0..sample_point_count {
        let r = sample_index as f64 * spacing;
        let basis_values = transform.basis().evaluate(r)?;
        let p_of_r = dot(&basis_values, coefficients);
        sampled.push(SampledPrPoint { r, p_of_r });
    }

    Ok(sampled)
}

fn pr_moments(
    transform: &ForwardTransform,
    coefficients: &[f64],
) -> Result<(f64, f64), AnalysisError> {
    let interval_count = transform.integration_intervals();
    let delta_r = transform.basis().dmax() / interval_count as f64;
    let mut i_zero = 0.0;
    let mut second_moment = 0.0;

    // Use the same midpoint quadrature style as the forward transform so the
    // derived real-space moments are numerically consistent with the rest of
    // the implementation.
    for interval_index in 0..interval_count {
        let r = (interval_index as f64 + 0.5) * delta_r;
        let basis_values = transform.basis().evaluate(r)?;
        let p_of_r = dot(&basis_values, coefficients);
        i_zero += p_of_r * delta_r;
        second_moment += r * r * p_of_r * delta_r;
    }

    Ok((i_zero, second_moment))
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

#[cfg(test)]
mod tests {
    use super::{AnalysisError, summarize_fit};
    use crate::basis::CubicBSplineBasis;
    use crate::data::{SaxsCurve, SaxsPoint};
    use crate::solver::solve_curve;
    use crate::transform::ForwardTransform;

    fn assert_close(left: f64, right: f64, tolerance: f64) {
        let delta = (left - right).abs();
        assert!(
            delta <= tolerance,
            "expected {left} to be within {tolerance} of {right}, difference was {delta}"
        );
    }

    fn synthetic_curve_from_coefficients(coefficients: &[f64]) -> (SaxsCurve, ForwardTransform) {
        let basis = CubicBSplineBasis::new(8.0, coefficients.len()).unwrap();
        let transform = ForwardTransform::new(basis, 800).unwrap();
        let q_values = (0..18)
            .map(|index| 0.05 + index as f64 * 0.08)
            .collect::<Vec<_>>();
        let intensities = transform.predict(&q_values, coefficients).unwrap();

        let curve = SaxsCurve::new(
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
        .unwrap();

        (curve, transform)
    }

    #[test]
    fn summarizes_constant_pr_fit_with_expected_i_zero_and_rg() {
        let coefficients = vec![1.0; 6];
        let (curve, transform) = synthetic_curve_from_coefficients(&coefficients);
        let fit = solve_curve(&curve, &transform, 1.0e-8).unwrap();

        let summary = summarize_fit(&curve, &transform, &fit, 41).unwrap();

        assert_close(summary.i_zero, 8.0, 1.0e-3);
        assert_close(summary.radius_of_gyration, (64.0_f64 / 6.0).sqrt(), 1.0e-2);
        assert_eq!(summary.sampled_pr.len(), 41);
        assert_close(summary.sampled_pr[0].p_of_r, 1.0, 2.0e-3);
        assert_close(summary.sampled_pr[40].p_of_r, 1.0, 2.0e-3);
    }

    #[test]
    fn reports_reduced_chi_square_when_degrees_of_freedom_are_positive() {
        let coefficients = vec![1.0; 6];
        let (curve, transform) = synthetic_curve_from_coefficients(&coefficients);
        let fit = solve_curve(&curve, &transform, 1.0e-8).unwrap();

        let summary = summarize_fit(&curve, &transform, &fit, 21).unwrap();

        assert!(summary.reduced_chi_square.is_some());
        assert!(summary.chi_square >= 0.0);
    }

    #[test]
    fn rejects_invalid_sample_count() {
        let coefficients = vec![1.0; 6];
        let (curve, transform) = synthetic_curve_from_coefficients(&coefficients);
        let fit = solve_curve(&curve, &transform, 1.0e-8).unwrap();
        let error = summarize_fit(&curve, &transform, &fit, 1).unwrap_err();

        assert!(matches!(
            error,
            AnalysisError::InvalidSamplePointCount {
                sample_point_count: 1
            }
        ));
    }

    #[test]
    fn rejects_non_positive_i_zero() {
        let basis = CubicBSplineBasis::new(8.0, 6).unwrap();
        let transform = ForwardTransform::new(basis, 800).unwrap();
        let q_values = (0..18)
            .map(|index| 0.05 + index as f64 * 0.08)
            .collect::<Vec<_>>();
        let intensities = transform.predict(&q_values, &[1.0; 6]).unwrap();
        let curve = SaxsCurve::new(
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
        .unwrap();
        let fit = crate::solver::FitResult {
            coefficients: vec![-1.0; 6],
            predicted_intensities: intensities.clone(),
            residuals: vec![0.0; intensities.len()],
            weighted_residual_sum_squares: 0.0,
            regularization_penalty: 0.0,
            objective_value: 0.0,
            effective_degrees_of_freedom: 0.0,
            lambda: 0.0,
        };

        let error = summarize_fit(&curve, &transform, &fit, 21).unwrap_err();
        assert!(matches!(error, AnalysisError::NonPositiveI0 { .. }));
    }
}
