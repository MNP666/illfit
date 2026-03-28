//! Derived quantities and scientist-facing summaries built from fitted models.

mod dmax_scan;
mod fit_summary;
mod truncation_scan;

pub use dmax_scan::{
    DmaxScanConfig, DmaxScanEntry, DmaxScanError, DmaxScanMetricRange, DmaxScanResult,
    DmaxScanStabilitySummary, run_dmax_scan,
};
pub use fit_summary::{AnalysisError, FitSummary, SampledPrPoint, summarize_fit};
pub use truncation_scan::{
    TruncationQualityFlag, TruncationScanConfig, TruncationScanEntry, TruncationScanError,
    TruncationScanOutcome, TruncationScanResult, TruncationScanStabilitySummary,
    run_truncation_scan,
};
