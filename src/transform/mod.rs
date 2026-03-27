//! Forward transforms between real-space `P(r)` representations and `I(q)`.
//!
//! The initial implementation favors clarity over sophistication: we assemble a
//! numerical design matrix by integrating each basis function against the SAXS
//! kernel `sin(qr) / (qr)` on a dense `r` grid.

mod forward;

pub use forward::{ForwardTransform, TransformError};
