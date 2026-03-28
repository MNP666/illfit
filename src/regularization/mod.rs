//! Regularization operators for stabilizing inverse problems.
//!
//! The first regularizer is a second-difference penalty on neighboring basis
//! coefficients. It is simple, easy to inspect, and captures the intuition that
//! strongly oscillatory coefficient patterns should be discouraged.

mod second_difference;

pub use second_difference::{PenaltyError, SecondDifferencePenalty};
