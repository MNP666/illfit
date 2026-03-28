use std::error::Error;
use std::fmt;

/// Errors produced while building a regularization operator.
#[derive(Debug, Clone, PartialEq)]
pub enum PenaltyError {
    TooFewCoefficients { coefficient_count: usize },
}

impl fmt::Display for PenaltyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewCoefficients { coefficient_count } => write!(
                f,
                "second-difference regularization requires at least 3 coefficients, but received {coefficient_count}"
            ),
        }
    }
}

impl Error for PenaltyError {}

/// Second-difference penalty on neighboring coefficients.
///
/// Each row of the operator computes:
///
/// `c_i - 2 c_(i+1) + c_(i+2)`
///
/// which is the discrete analogue of curvature. Penalizing the squared norm of
/// this quantity favors smooth coefficient sequences.
#[derive(Debug, Clone, PartialEq)]
pub struct SecondDifferencePenalty {
    coefficient_count: usize,
    operator: Vec<Vec<f64>>,
}

impl SecondDifferencePenalty {
    pub fn new(coefficient_count: usize) -> Result<Self, PenaltyError> {
        if coefficient_count < 3 {
            return Err(PenaltyError::TooFewCoefficients { coefficient_count });
        }

        let mut operator = Vec::with_capacity(coefficient_count - 2);

        for row_index in 0..(coefficient_count - 2) {
            let mut row = vec![0.0; coefficient_count];
            row[row_index] = 1.0;
            row[row_index + 1] = -2.0;
            row[row_index + 2] = 1.0;
            operator.push(row);
        }

        Ok(Self {
            coefficient_count,
            operator,
        })
    }

    pub fn coefficient_count(&self) -> usize {
        self.coefficient_count
    }

    pub fn row_count(&self) -> usize {
        self.operator.len()
    }

    pub fn operator(&self) -> &[Vec<f64>] {
        &self.operator
    }

    /// Return `L^T L`, which is the matrix that appears in the normal equations.
    pub fn normal_matrix(&self) -> Vec<Vec<f64>> {
        let mut normal = vec![vec![0.0; self.coefficient_count]; self.coefficient_count];

        for row in &self.operator {
            for i in 0..self.coefficient_count {
                for j in 0..self.coefficient_count {
                    normal[i][j] += row[i] * row[j];
                }
            }
        }

        normal
    }

    /// Compute the unscaled smoothness penalty `||L c||^2`.
    pub fn penalty_value(&self, coefficients: &[f64]) -> f64 {
        self.operator
            .iter()
            .map(|row| {
                let value = row
                    .iter()
                    .zip(coefficients.iter())
                    .map(|(entry, coefficient)| entry * coefficient)
                    .sum::<f64>();
                value * value
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::{PenaltyError, SecondDifferencePenalty};

    fn assert_close(left: f64, right: f64, tolerance: f64) {
        let delta = (left - right).abs();
        assert!(
            delta <= tolerance,
            "expected {left} to be within {tolerance} of {right}, difference was {delta}"
        );
    }

    #[test]
    fn rejects_too_few_coefficients() {
        let error = SecondDifferencePenalty::new(2).unwrap_err();
        assert_eq!(
            error,
            PenaltyError::TooFewCoefficients {
                coefficient_count: 2
            }
        );
    }

    #[test]
    fn constructs_expected_operator() {
        let penalty = SecondDifferencePenalty::new(5).unwrap();

        assert_eq!(penalty.row_count(), 3);
        assert_eq!(penalty.operator()[0], vec![1.0, -2.0, 1.0, 0.0, 0.0]);
        assert_eq!(penalty.operator()[1], vec![0.0, 1.0, -2.0, 1.0, 0.0]);
        assert_eq!(penalty.operator()[2], vec![0.0, 0.0, 1.0, -2.0, 1.0]);
    }

    #[test]
    fn constant_coefficients_have_zero_penalty() {
        let penalty = SecondDifferencePenalty::new(6).unwrap();
        let coefficients = vec![3.0; 6];

        assert_close(penalty.penalty_value(&coefficients), 0.0, 1.0e-12);
    }

    #[test]
    fn normal_matrix_is_symmetric() {
        let penalty = SecondDifferencePenalty::new(6).unwrap();
        let normal = penalty.normal_matrix();

        for (row_index, row) in normal.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                assert_eq!(*value, normal[column_index][row_index]);
            }
        }
    }
}
