use std::error::Error;
use std::fmt;

/// Validation errors for basis construction.
#[derive(Debug, Clone, PartialEq)]
pub enum BasisError {
    InvalidDmax { dmax: f64 },
    TooFewBasisFunctions { basis_size: usize },
    NonFiniteSamplePoint { r: f64 },
}

impl fmt::Display for BasisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDmax { dmax } => {
                write!(f, "Dmax must be finite and positive, but was {dmax}")
            }
            Self::TooFewBasisFunctions { basis_size } => write!(
                f,
                "cubic B-splines require at least 4 basis functions, but received {basis_size}"
            ),
            Self::NonFiniteSamplePoint { r } => {
                write!(f, "basis evaluation point must be finite, but was {r}")
            }
        }
    }
}

impl Error for BasisError {}

/// Open, clamped cubic B-spline basis on the interval `0 <= r <= Dmax`.
///
/// The basis uses uniform knot spacing in the interior and repeats the endpoint
/// knots four times. This is the standard "open uniform" construction, which
/// ensures that the basis spans the full domain while keeping each basis
/// function locally supported.
#[derive(Debug, Clone, PartialEq)]
pub struct CubicBSplineBasis {
    dmax: f64,
    basis_size: usize,
    knots: Vec<f64>,
}

impl CubicBSplineBasis {
    pub const DEGREE: usize = 3;

    /// Create a cubic B-spline basis over `0 <= r <= Dmax`.
    ///
    /// `basis_size` is the number of basis functions. For cubic splines we need
    /// at least four basis functions to define the clamped basis.
    pub fn new(dmax: f64, basis_size: usize) -> Result<Self, BasisError> {
        if !dmax.is_finite() || dmax <= 0.0 {
            return Err(BasisError::InvalidDmax { dmax });
        }

        if basis_size < Self::DEGREE + 1 {
            return Err(BasisError::TooFewBasisFunctions { basis_size });
        }

        let knots = build_open_uniform_knots(dmax, basis_size, Self::DEGREE);

        Ok(Self {
            dmax,
            basis_size,
            knots,
        })
    }

    pub fn dmax(&self) -> f64 {
        self.dmax
    }

    pub fn basis_size(&self) -> usize {
        self.basis_size
    }

    pub fn knots(&self) -> &[f64] {
        &self.knots
    }

    /// Evaluate all basis functions at one `r` value.
    ///
    /// Outside the support interval `0 <= r <= Dmax`, all basis functions are
    /// zero. At `r == Dmax` we assign the entire weight to the last basis
    /// function so the clamped basis remains well-defined at the right edge.
    pub fn evaluate(&self, r: f64) -> Result<Vec<f64>, BasisError> {
        if !r.is_finite() {
            return Err(BasisError::NonFiniteSamplePoint { r });
        }

        if !(0.0..=self.dmax).contains(&r) {
            return Ok(vec![0.0; self.basis_size]);
        }

        if r == self.dmax {
            let mut values = vec![0.0; self.basis_size];
            values[self.basis_size - 1] = 1.0;
            return Ok(values);
        }

        let values = (0..self.basis_size)
            .map(|basis_index| self.basis_value(basis_index, Self::DEGREE, r))
            .collect();

        Ok(values)
    }

    /// Evaluate one basis function at one `r` value.
    pub fn evaluate_basis(&self, basis_index: usize, r: f64) -> Result<f64, BasisError> {
        if basis_index >= self.basis_size {
            return Ok(0.0);
        }

        Ok(self.evaluate(r)?[basis_index])
    }

    /// Evaluate the full basis on a grid of `r` values.
    ///
    /// Each row of the returned matrix corresponds to one input `r` value.
    pub fn evaluate_grid(&self, r_grid: &[f64]) -> Result<Vec<Vec<f64>>, BasisError> {
        r_grid.iter().map(|&r| self.evaluate(r)).collect()
    }

    fn basis_value(&self, basis_index: usize, degree: usize, r: f64) -> f64 {
        if degree == 0 {
            let left = self.knots[basis_index];
            let right = self.knots[basis_index + 1];

            return if left <= r && r < right { 1.0 } else { 0.0 };
        }

        // Cox-de Boor recursion expresses each degree-p basis function as a
        // weighted combination of two degree-(p-1) basis functions. The
        // weights are zero whenever a knot interval has zero width, which
        // happens naturally at the clamped endpoints.
        let left_denominator = self.knots[basis_index + degree] - self.knots[basis_index];
        let left_term = if left_denominator > 0.0 {
            let left_weight = (r - self.knots[basis_index]) / left_denominator;
            left_weight * self.basis_value(basis_index, degree - 1, r)
        } else {
            0.0
        };

        let right_denominator = self.knots[basis_index + degree + 1] - self.knots[basis_index + 1];
        let right_term = if right_denominator > 0.0 {
            let right_weight = (self.knots[basis_index + degree + 1] - r) / right_denominator;
            right_weight * self.basis_value(basis_index + 1, degree - 1, r)
        } else {
            0.0
        };

        left_term + right_term
    }
}

fn build_open_uniform_knots(dmax: f64, basis_size: usize, degree: usize) -> Vec<f64> {
    let span_count = basis_size - degree;
    let knot_count = basis_size + degree + 1;
    let mut knots = Vec::with_capacity(knot_count);

    for knot_index in 0..knot_count {
        if knot_index <= degree {
            knots.push(0.0);
        } else if knot_index >= basis_size {
            knots.push(dmax);
        } else {
            let step = dmax / span_count as f64;
            knots.push((knot_index - degree) as f64 * step);
        }
    }

    knots
}

#[cfg(test)]
mod tests {
    use super::{BasisError, CubicBSplineBasis};

    fn assert_close(left: f64, right: f64, tolerance: f64) {
        let delta = (left - right).abs();
        assert!(
            delta <= tolerance,
            "expected {left} to be within {tolerance} of {right}, difference was {delta}"
        );
    }

    #[test]
    fn rejects_invalid_construction_parameters() {
        assert_eq!(
            CubicBSplineBasis::new(0.0, 6).unwrap_err(),
            BasisError::InvalidDmax { dmax: 0.0 }
        );
        assert_eq!(
            CubicBSplineBasis::new(10.0, 3).unwrap_err(),
            BasisError::TooFewBasisFunctions { basis_size: 3 }
        );
    }

    #[test]
    fn constructs_expected_open_uniform_knot_vector() {
        let basis = CubicBSplineBasis::new(12.0, 6).unwrap();

        assert_eq!(
            basis.knots(),
            &[0.0, 0.0, 0.0, 0.0, 4.0, 8.0, 12.0, 12.0, 12.0, 12.0]
        );
    }

    #[test]
    fn partitions_unity_inside_support() {
        let basis = CubicBSplineBasis::new(10.0, 7).unwrap();

        for r in [0.0, 0.25, 1.5, 4.2, 7.75, 9.999] {
            let values = basis.evaluate(r).unwrap();
            let sum: f64 = values.iter().sum();
            assert_close(sum, 1.0, 1.0e-12);
        }
    }

    #[test]
    fn evaluates_right_endpoint_as_last_basis_function() {
        let basis = CubicBSplineBasis::new(10.0, 7).unwrap();
        let values = basis.evaluate(10.0).unwrap();

        assert_eq!(values.iter().filter(|&&value| value > 0.0).count(), 1);
        assert_eq!(values[6], 1.0);
    }

    #[test]
    fn returns_zero_outside_support() {
        let basis = CubicBSplineBasis::new(10.0, 7).unwrap();

        assert!(
            basis
                .evaluate(-0.1)
                .unwrap()
                .iter()
                .all(|&value| value == 0.0)
        );
        assert!(
            basis
                .evaluate(10.1)
                .unwrap()
                .iter()
                .all(|&value| value == 0.0)
        );
    }

    #[test]
    fn basis_functions_have_local_support() {
        let basis = CubicBSplineBasis::new(12.0, 6).unwrap();

        assert_eq!(basis.evaluate_basis(0, 9.0).unwrap(), 0.0);
        assert_eq!(basis.evaluate_basis(5, 1.0).unwrap(), 0.0);
    }

    #[test]
    fn evaluates_basis_on_a_grid() {
        let basis = CubicBSplineBasis::new(5.0, 5).unwrap();
        let grid = vec![0.0, 1.25, 2.5, 5.0];
        let matrix = basis.evaluate_grid(&grid).unwrap();

        assert_eq!(matrix.len(), grid.len());
        assert_eq!(matrix[0].len(), basis.basis_size());
        assert_eq!(matrix[3][basis.basis_size() - 1], 1.0);
    }

    #[test]
    fn rejects_non_finite_sample_points() {
        let basis = CubicBSplineBasis::new(5.0, 5).unwrap();
        let error = basis.evaluate(f64::NAN).unwrap_err();

        assert!(matches!(
            error,
            BasisError::NonFiniteSamplePoint { r } if r.is_nan()
        ));
    }
}
