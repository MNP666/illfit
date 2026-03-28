use std::error::Error;
use std::fmt;

use crate::data::SaxsCurve;
use crate::regularization::{PenaltyError, SecondDifferencePenalty};
use crate::transform::{ForwardTransform, TransformError};

#[derive(Debug)]
pub enum SolverError {
    Transform(TransformError),
    Penalty(PenaltyError),
    InvalidLambda { lambda: f64 },
    LinearSolveFailed,
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transform(error) => write!(f, "{error}"),
            Self::Penalty(error) => write!(f, "{error}"),
            Self::InvalidLambda { lambda } => write!(
                f,
                "regularization strength lambda must be finite and non-negative, but was {lambda}"
            ),
            Self::LinearSolveFailed => write!(
                f,
                "failed to solve the regularized normal equations; the system may be singular"
            ),
        }
    }
}

impl Error for SolverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transform(error) => Some(error),
            Self::Penalty(error) => Some(error),
            Self::InvalidLambda { .. } | Self::LinearSolveFailed => None,
        }
    }
}

impl From<TransformError> for SolverError {
    fn from(value: TransformError) -> Self {
        Self::Transform(value)
    }
}

impl From<PenaltyError> for SolverError {
    fn from(value: PenaltyError) -> Self {
        Self::Penalty(value)
    }
}

/// Result of a weighted regularized least-squares fit.
#[derive(Debug, Clone)]
pub struct FitResult {
    pub coefficients: Vec<f64>,
    pub predicted_intensities: Vec<f64>,
    pub residuals: Vec<f64>,
    pub weighted_residual_sum_squares: f64,
    pub regularization_penalty: f64,
    pub objective_value: f64,
    pub lambda: f64,
}

/// Solve for `P(r)` basis coefficients using weighted regularized least squares.
///
/// The objective is:
///
/// `||W (A c - y)||^2 + lambda ||L c||^2`
///
/// where:
///
/// - `A` is the forward-transform design matrix
/// - `W` is the diagonal weight matrix with entries `1 / sigma_i`
/// - `L` is the second-difference regularization operator
pub fn solve_curve(
    curve: &SaxsCurve,
    transform: &ForwardTransform,
    lambda: f64,
) -> Result<FitResult, SolverError> {
    if !lambda.is_finite() || lambda < 0.0 {
        return Err(SolverError::InvalidLambda { lambda });
    }

    let design_matrix = transform.design_matrix_for_curve(curve)?;
    let penalty = SecondDifferencePenalty::new(transform.basis().basis_size())?;
    let normal_penalty = penalty.normal_matrix();

    let mut system_matrix =
        vec![vec![0.0; transform.basis().basis_size()]; transform.basis().basis_size()];
    let mut right_hand_side = vec![0.0; transform.basis().basis_size()];

    for (row, point) in design_matrix.iter().zip(curve.points()) {
        let weight = 1.0 / point.sigma;

        for column_index in 0..row.len() {
            right_hand_side[column_index] += row[column_index] * point.intensity * weight * weight;

            for other_index in 0..row.len() {
                system_matrix[column_index][other_index] +=
                    row[column_index] * row[other_index] * weight * weight;
            }
        }
    }

    for row_index in 0..system_matrix.len() {
        for column_index in 0..system_matrix.len() {
            system_matrix[row_index][column_index] +=
                lambda * normal_penalty[row_index][column_index];
        }
    }

    let coefficients = solve_symmetric_positive_definite(&system_matrix, &right_hand_side)
        .ok_or(SolverError::LinearSolveFailed)?;

    let predicted_intensities = transform.predict_for_curve(curve, &coefficients)?;
    let residuals = curve
        .points()
        .iter()
        .zip(predicted_intensities.iter())
        .map(|(point, predicted)| predicted - point.intensity)
        .collect::<Vec<_>>();

    let weighted_residual_sum_squares = curve
        .points()
        .iter()
        .zip(residuals.iter())
        .map(|(point, residual)| {
            let weighted = residual / point.sigma;
            weighted * weighted
        })
        .sum::<f64>();

    let regularization_penalty = penalty.penalty_value(&coefficients);
    let objective_value = weighted_residual_sum_squares + lambda * regularization_penalty;

    Ok(FitResult {
        coefficients,
        predicted_intensities,
        residuals,
        weighted_residual_sum_squares,
        regularization_penalty,
        objective_value,
        lambda,
    })
}

fn solve_symmetric_positive_definite(matrix: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    let cholesky = cholesky_decompose(matrix)?;
    let y = forward_substitute(&cholesky, rhs)?;
    backward_substitute_transpose(&cholesky, &y)
}

fn cholesky_decompose(matrix: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = matrix.len();
    if n == 0 || rhs_shape_mismatch(matrix) {
        return None;
    }

    let mut lower = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..=i {
            let sum = (0..j).map(|k| lower[i][k] * lower[j][k]).sum::<f64>();

            if i == j {
                let diagonal = matrix[i][i] - sum;
                if diagonal <= 0.0 {
                    return None;
                }
                lower[i][j] = diagonal.sqrt();
            } else {
                lower[i][j] = (matrix[i][j] - sum) / lower[j][j];
            }
        }
    }

    Some(lower)
}

fn rhs_shape_mismatch(matrix: &[Vec<f64>]) -> bool {
    let n = matrix.len();
    matrix.iter().any(|row| row.len() != n)
}

fn forward_substitute(lower: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    if lower.len() != rhs.len() {
        return None;
    }

    let n = rhs.len();
    let mut solution = vec![0.0; n];

    for i in 0..n {
        let sum = (0..i).map(|j| lower[i][j] * solution[j]).sum::<f64>();
        let diagonal = lower[i][i];
        if diagonal == 0.0 {
            return None;
        }
        solution[i] = (rhs[i] - sum) / diagonal;
    }

    Some(solution)
}

fn backward_substitute_transpose(lower: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    if lower.len() != rhs.len() {
        return None;
    }

    let n = rhs.len();
    let mut solution = vec![0.0; n];

    for i in (0..n).rev() {
        let sum = ((i + 1)..n).map(|j| lower[j][i] * solution[j]).sum::<f64>();
        let diagonal = lower[i][i];
        if diagonal == 0.0 {
            return None;
        }
        solution[i] = (rhs[i] - sum) / diagonal;
    }

    Some(solution)
}

#[cfg(test)]
mod tests {
    use super::{SolverError, solve_curve};
    use crate::basis::CubicBSplineBasis;
    use crate::data::{SaxsCurve, SaxsPoint};
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
    fn rejects_invalid_lambda() {
        let (curve, transform) = synthetic_curve_from_coefficients(&[1.0; 6]);
        let error = solve_curve(&curve, &transform, -1.0).unwrap_err();

        assert!(matches!(error, SolverError::InvalidLambda { lambda: -1.0 }));
    }

    #[test]
    fn recovers_smooth_coefficients_from_synthetic_curve() {
        let expected_coefficients = vec![1.0, 1.2, 1.4, 1.5, 1.45, 1.3];
        let (curve, transform) = synthetic_curve_from_coefficients(&expected_coefficients);

        let fit = solve_curve(&curve, &transform, 1.0e-6).unwrap();

        for (recovered, expected) in fit.coefficients.iter().zip(expected_coefficients.iter()) {
            assert_close(*recovered, *expected, 2.0e-3);
        }

        assert_close(fit.weighted_residual_sum_squares, 0.0, 1.0e-6);
    }

    #[test]
    fn reports_prediction_and_residual_lengths_consistently() {
        let (curve, transform) = synthetic_curve_from_coefficients(&[1.0; 6]);
        let fit = solve_curve(&curve, &transform, 1.0e-4).unwrap();

        assert_eq!(fit.predicted_intensities.len(), curve.len());
        assert_eq!(fit.residuals.len(), curve.len());
    }

    #[test]
    fn objective_includes_regularization_penalty() {
        let (curve, transform) = synthetic_curve_from_coefficients(&[1.0, 2.0, 1.0, 2.0, 1.0, 2.0]);
        let lambda = 0.5;
        let fit = solve_curve(&curve, &transform, lambda).unwrap();

        assert_close(
            fit.objective_value,
            fit.weighted_residual_sum_squares + lambda * fit.regularization_penalty,
            1.0e-12,
        );
        assert!(fit.regularization_penalty >= 0.0);
    }
}
