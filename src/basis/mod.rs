//! Basis-function representations for smooth `P(r)` models.
//!
//! Version 0.2 starts with cubic B-splines because they give us local support,
//! smoothness, and a clear parameterization over the compact interval
//! `0 <= r <= Dmax`.

mod cubic_bspline;

pub use cubic_bspline::{BasisError, CubicBSplineBasis};
