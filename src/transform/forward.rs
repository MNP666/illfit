use std::error::Error;
use std::fmt;

use crate::basis::{BasisError, CubicBSplineBasis};
use crate::data::SaxsCurve;

/// Errors produced while assembling or applying the forward transform.
#[derive(Debug)]
pub enum TransformError {
    Basis(BasisError),
    InvalidIntegrationIntervals { intervals: usize },
    NonFiniteQ { q: f64 },
    CoefficientLengthMismatch { expected: usize, actual: usize },
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Basis(error) => write!(f, "{error}"),
            Self::InvalidIntegrationIntervals { intervals } => write!(
                f,
                "integration interval count must be positive, but was {intervals}"
            ),
            Self::NonFiniteQ { q } => write!(f, "q values must be finite, but found {q}"),
            Self::CoefficientLengthMismatch { expected, actual } => write!(
                f,
                "coefficient count must match basis size: expected {expected}, found {actual}"
            ),
        }
    }
}

impl Error for TransformError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Basis(error) => Some(error),
            Self::InvalidIntegrationIntervals { .. }
            | Self::NonFiniteQ { .. }
            | Self::CoefficientLengthMismatch { .. } => None,
        }
    }
}

impl From<BasisError> for TransformError {
    fn from(value: BasisError) -> Self {
        Self::Basis(value)
    }
}

/// Numerical forward transform for a cubic B-spline `P(r)` basis.
///
/// For basis coefficients `c_j` and basis functions `B_j(r)`, the real-space
/// model is:
///
/// `P(r) = sum_j c_j B_j(r)`
///
/// and the predicted scattering curve is:
///
/// `I(q) = integral_0^Dmax P(r) * sinc(q r) dr`
///
/// where `sinc(x) = sin(x) / x` with `sinc(0) = 1`.
///
/// We approximate the integral with a composite midpoint rule. This keeps the
/// implementation easy to inspect and gives a clear design-matrix
/// interpretation that will be useful once we move on to fitting.
#[derive(Debug, Clone)]
pub struct ForwardTransform {
    basis: CubicBSplineBasis,
    integration_intervals: usize,
}

impl ForwardTransform {
    pub fn new(
        basis: CubicBSplineBasis,
        integration_intervals: usize,
    ) -> Result<Self, TransformError> {
        if integration_intervals == 0 {
            return Err(TransformError::InvalidIntegrationIntervals {
                intervals: integration_intervals,
            });
        }

        Ok(Self {
            basis,
            integration_intervals,
        })
    }

    pub fn basis(&self) -> &CubicBSplineBasis {
        &self.basis
    }

    pub fn integration_intervals(&self) -> usize {
        self.integration_intervals
    }

    /// Assemble the forward-transform design matrix for a list of `q` values.
    ///
    /// Each row corresponds to one `q` value and each column corresponds to one
    /// basis coefficient.
    pub fn design_matrix(&self, q_values: &[f64]) -> Result<Vec<Vec<f64>>, TransformError> {
        q_values
            .iter()
            .map(|&q| self.design_matrix_row(q))
            .collect()
    }

    /// Convenience wrapper that uses the `q` values from a validated SAXS curve.
    pub fn design_matrix_for_curve(
        &self,
        curve: &SaxsCurve,
    ) -> Result<Vec<Vec<f64>>, TransformError> {
        let q_values = curve
            .points()
            .iter()
            .map(|point| point.q)
            .collect::<Vec<_>>();
        self.design_matrix(&q_values)
    }

    /// Predict `I(q)` for a list of `q` values.
    pub fn predict(
        &self,
        q_values: &[f64],
        coefficients: &[f64],
    ) -> Result<Vec<f64>, TransformError> {
        self.validate_coefficients(coefficients)?;

        let matrix = self.design_matrix(q_values)?;
        Ok(matrix
            .iter()
            .map(|row| dot(row, coefficients))
            .collect::<Vec<_>>())
    }

    /// Predict `I(q)` using the `q` values from a validated SAXS curve.
    pub fn predict_for_curve(
        &self,
        curve: &SaxsCurve,
        coefficients: &[f64],
    ) -> Result<Vec<f64>, TransformError> {
        self.validate_coefficients(coefficients)?;

        let q_values = curve
            .points()
            .iter()
            .map(|point| point.q)
            .collect::<Vec<_>>();
        self.predict(&q_values, coefficients)
    }

    fn design_matrix_row(&self, q: f64) -> Result<Vec<f64>, TransformError> {
        if !q.is_finite() {
            return Err(TransformError::NonFiniteQ { q });
        }

        let delta_r = self.basis.dmax() / self.integration_intervals as f64;
        let mut row = vec![0.0; self.basis.basis_size()];

        for interval_index in 0..self.integration_intervals {
            let r = (interval_index as f64 + 0.5) * delta_r;
            let basis_values = self.basis.evaluate(r)?;
            let kernel = sinc(q * r);

            for (entry, basis_value) in row.iter_mut().zip(basis_values) {
                *entry += basis_value * kernel * delta_r;
            }
        }

        Ok(row)
    }

    fn validate_coefficients(&self, coefficients: &[f64]) -> Result<(), TransformError> {
        if coefficients.len() != self.basis.basis_size() {
            return Err(TransformError::CoefficientLengthMismatch {
                expected: self.basis.basis_size(),
                actual: coefficients.len(),
            });
        }

        Ok(())
    }
}

fn sinc(x: f64) -> f64 {
    if x == 0.0 { 1.0 } else { x.sin() / x }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

#[cfg(test)]
mod tests {
    use super::{ForwardTransform, TransformError};
    use crate::basis::CubicBSplineBasis;
    use crate::data::{SaxsCurve, SaxsPoint};

    fn assert_close(left: f64, right: f64, tolerance: f64) {
        let delta = (left - right).abs();
        assert!(
            delta <= tolerance,
            "expected {left} to be within {tolerance} of {right}, difference was {delta}"
        );
    }

    fn reference_constant_pr_intensity(q: f64, dmax: f64, intervals: usize) -> f64 {
        let delta_r = dmax / intervals as f64;
        let mut sum = 0.0;

        for interval_index in 0..intervals {
            let r = (interval_index as f64 + 0.5) * delta_r;
            let kernel = if q == 0.0 {
                1.0
            } else {
                (q * r).sin() / (q * r)
            };
            sum += kernel * delta_r;
        }

        sum
    }

    #[test]
    fn rejects_invalid_transform_parameters() {
        let basis = CubicBSplineBasis::new(10.0, 6).unwrap();
        let error = ForwardTransform::new(basis, 0).unwrap_err();

        assert!(matches!(
            error,
            TransformError::InvalidIntegrationIntervals { intervals: 0 }
        ));
    }

    #[test]
    fn predicts_i_of_zero_as_integral_of_p_of_r() {
        let basis = CubicBSplineBasis::new(10.0, 6).unwrap();
        let transform = ForwardTransform::new(basis, 400).unwrap();
        let coefficients = vec![1.0; 6];

        let intensities = transform.predict(&[0.0], &coefficients).unwrap();

        assert_close(intensities[0], 10.0, 1.0e-12);
    }

    #[test]
    fn predicts_constant_pr_curve_consistently() {
        let basis = CubicBSplineBasis::new(8.0, 7).unwrap();
        let transform = ForwardTransform::new(basis, 800).unwrap();
        let coefficients = vec![1.0; 7];
        let q_values = vec![0.0, 0.25, 0.75, 1.5];

        let predicted = transform.predict(&q_values, &coefficients).unwrap();

        for (predicted_value, q) in predicted.iter().zip(q_values) {
            let reference = reference_constant_pr_intensity(q, 8.0, 50_000);
            assert_close(*predicted_value, reference, 1.0e-3);
        }
    }

    #[test]
    fn assembles_design_matrix_for_validated_curve() {
        let basis = CubicBSplineBasis::new(8.0, 7).unwrap();
        let transform = ForwardTransform::new(basis, 200).unwrap();
        let curve = SaxsCurve::new(vec![
            SaxsPoint {
                q: 0.1,
                intensity: 1.0,
                sigma: 0.1,
            },
            SaxsPoint {
                q: 0.2,
                intensity: 0.8,
                sigma: 0.1,
            },
        ])
        .unwrap();

        let matrix = transform.design_matrix_for_curve(&curve).unwrap();

        assert_eq!(matrix.len(), curve.len());
        assert_eq!(matrix[0].len(), transform.basis().basis_size());
    }

    #[test]
    fn rejects_non_finite_q_values() {
        let basis = CubicBSplineBasis::new(8.0, 7).unwrap();
        let transform = ForwardTransform::new(basis, 200).unwrap();
        let coefficients = vec![1.0; 7];
        let error = transform.predict(&[f64::NAN], &coefficients).unwrap_err();

        assert!(matches!(error, TransformError::NonFiniteQ { q } if q.is_nan()));
    }

    #[test]
    fn rejects_coefficient_length_mismatch() {
        let basis = CubicBSplineBasis::new(8.0, 7).unwrap();
        let transform = ForwardTransform::new(basis, 200).unwrap();
        let error = transform.predict(&[0.1, 0.2], &[1.0; 6]).unwrap_err();

        assert!(matches!(
            error,
            TransformError::CoefficientLengthMismatch {
                expected: 7,
                actual: 6
            }
        ));
    }
}
