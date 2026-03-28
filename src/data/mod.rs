//! Data ingestion and validation for SAXS curves.
//!
//! The parser is intentionally permissive about file formatting: any line with
//! exactly three whitespace-separated numeric fields is treated as data.
//! Validation is then applied at the domain level so downstream code can assume
//! the curve is scientifically well-formed.

mod parser;
mod reference_pr;

pub use parser::{
    ParseCurveError, SaxsCurve, SaxsPoint, parse_ascii_curve, parse_ascii_curve_file,
};
pub use reference_pr::{
    ParseReferencePrError, PrDistribution, PrPoint, parse_reference_pr, parse_reference_pr_file,
};
