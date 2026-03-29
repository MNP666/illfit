//! Solvers for inverse problems built on top of the forward transform.
//!
//! The initial solver is a weighted regularized least-squares solve using
//! normal equations and a smoothness penalty.

mod regularized_least_squares;

pub use regularized_least_squares::{FitResult, SolverError, solve_curve};
pub use regularized_least_squares::{
    LeastSquaresObservation, LinearSolveResult, solve_design_matrix,
};
