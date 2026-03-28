//! Output writing for fit artifacts and reports.

mod results;

pub use results::{
    OutputError, write_dmax_scan_outputs, write_fit_outputs, write_truncation_scan_outputs,
};
