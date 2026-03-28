//! Derived quantities and scientist-facing summaries built from fitted models.

mod fit_summary;

pub use fit_summary::{AnalysisError, FitSummary, SampledPrPoint, summarize_fit};
