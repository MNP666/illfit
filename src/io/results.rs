use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::analysis::{DmaxScanResult, FitSummary, TruncationScanOutcome, TruncationScanResult};
use crate::data::SaxsCurve;
use crate::solver::FitResult;
use crate::transform::ForwardTransform;

#[derive(Debug)]
pub enum OutputError {
    Io(std::io::Error),
}

impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to write output files: {error}"),
        }
    }
}

impl Error for OutputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for OutputError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Write the standard fit artifacts for one solved SAXS iFT run.
pub fn write_fit_outputs(
    output_dir: impl AsRef<Path>,
    curve: &SaxsCurve,
    transform: &ForwardTransform,
    fit: &FitResult,
    summary: &FitSummary,
) -> Result<(), OutputError> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    write_pr_csv(output_dir.join("pr.csv"), summary)?;
    write_fit_csv(output_dir.join("fit.csv"), curve, fit)?;
    write_residuals_csv(output_dir.join("residuals.csv"), curve, fit)?;
    write_report_json(output_dir.join("report.json"), transform, fit, summary)?;

    Ok(())
}

/// Write the standard summary artifacts for a local `Dmax` scan.
pub fn write_dmax_scan_outputs(
    output_dir: impl AsRef<Path>,
    scan: &DmaxScanResult,
) -> Result<(), OutputError> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    write_dmax_scan_csv(output_dir.join("dmax_scan.csv"), scan)?;
    write_dmax_scan_report_json(output_dir.join("dmax_scan_report.json"), scan)?;

    Ok(())
}

/// Write the standard summary artifacts for a local low-q truncation scan.
pub fn write_truncation_scan_outputs(
    output_dir: impl AsRef<Path>,
    scan: &TruncationScanResult,
) -> Result<(), OutputError> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    write_truncation_scan_csv(output_dir.join("truncation_scan.csv"), scan)?;
    write_truncation_scan_report_json(output_dir.join("truncation_scan_report.json"), scan)?;

    Ok(())
}

fn write_pr_csv(path: impl AsRef<Path>, summary: &FitSummary) -> Result<(), OutputError> {
    let mut contents = String::from("r,p_of_r\n");

    for point in &summary.sampled_pr {
        contents.push_str(&format!("{:.12e},{:.12e}\n", point.r, point.p_of_r));
    }

    fs::write(path, contents)?;
    Ok(())
}

fn write_fit_csv(
    path: impl AsRef<Path>,
    curve: &SaxsCurve,
    fit: &FitResult,
) -> Result<(), OutputError> {
    let mut contents = String::from("q,intensity_measured,sigma,intensity_fitted\n");

    for (point, predicted) in curve.points().iter().zip(fit.predicted_intensities.iter()) {
        contents.push_str(&format!(
            "{:.12e},{:.12e},{:.12e},{:.12e}\n",
            point.q, point.intensity, point.sigma, predicted
        ));
    }

    fs::write(path, contents)?;
    Ok(())
}

fn write_residuals_csv(
    path: impl AsRef<Path>,
    curve: &SaxsCurve,
    fit: &FitResult,
) -> Result<(), OutputError> {
    let mut contents = String::from("q,residual,weighted_residual\n");

    for (point, residual) in curve.points().iter().zip(fit.residuals.iter()) {
        contents.push_str(&format!(
            "{:.12e},{:.12e},{:.12e}\n",
            point.q,
            residual,
            residual / point.sigma
        ));
    }

    fs::write(path, contents)?;
    Ok(())
}

fn write_report_json(
    path: impl AsRef<Path>,
    transform: &ForwardTransform,
    fit: &FitResult,
    summary: &FitSummary,
) -> Result<(), OutputError> {
    let reduced_chi_square = summary
        .reduced_chi_square
        .map(|value| format!("{value:.12e}"))
        .unwrap_or_else(|| String::from("null"));

    let coefficients_json = join_numeric_array(&fit.coefficients);

    let contents = format!(
        concat!(
            "{{\n",
            "  \"dmax\": {dmax:.12e},\n",
            "  \"basis_size\": {basis_size},\n",
            "  \"integration_intervals\": {integration_intervals},\n",
            "  \"lambda\": {lambda:.12e},\n",
            "  \"i_zero\": {i_zero:.12e},\n",
            "  \"radius_of_gyration\": {radius_of_gyration:.12e},\n",
            "  \"chi_square\": {chi_square:.12e},\n",
            "  \"reduced_chi_square\": {reduced_chi_square},\n",
            "  \"weighted_residual_sum_squares\": {wrss:.12e},\n",
            "  \"regularization_penalty\": {regularization_penalty:.12e},\n",
            "  \"objective_value\": {objective_value:.12e},\n",
            "  \"pr_sample_count\": {pr_sample_count},\n",
            "  \"coefficients\": {coefficients}\n",
            "}}\n"
        ),
        dmax = transform.basis().dmax(),
        basis_size = transform.basis().basis_size(),
        integration_intervals = transform.integration_intervals(),
        lambda = fit.lambda,
        i_zero = summary.i_zero,
        radius_of_gyration = summary.radius_of_gyration,
        chi_square = summary.chi_square,
        reduced_chi_square = reduced_chi_square,
        wrss = summary.weighted_residual_sum_squares,
        regularization_penalty = summary.regularization_penalty,
        objective_value = summary.objective_value,
        pr_sample_count = summary.sampled_pr.len(),
        coefficients = coefficients_json,
    );

    fs::write(path, contents)?;
    Ok(())
}

fn write_dmax_scan_csv(path: impl AsRef<Path>, scan: &DmaxScanResult) -> Result<(), OutputError> {
    let mut contents = String::from(
        "dmax,i_zero,radius_of_gyration,chi_square,reduced_chi_square,objective_value\n",
    );

    for entry in &scan.entries {
        let reduced = entry
            .summary
            .reduced_chi_square
            .map(|value| format!("{value:.12e}"))
            .unwrap_or_default();
        contents.push_str(
            &format!(
                "{:.12e},{:.12e},{:.12e},{:.12e},{}, {:.12e}\n",
                entry.dmax,
                entry.summary.i_zero,
                entry.summary.radius_of_gyration,
                entry.summary.chi_square,
                reduced,
                entry.summary.objective_value
            )
            .replace(", ", ","),
        );
    }

    fs::write(path, contents)?;
    Ok(())
}

fn write_dmax_scan_report_json(
    path: impl AsRef<Path>,
    scan: &DmaxScanResult,
) -> Result<(), OutputError> {
    let contents = format!(
        concat!(
            "{{\n",
            "  \"entry_count\": {entry_count},\n",
            "  \"dmax_range\": {dmax_range},\n",
            "  \"i_zero_range\": {i_zero_range},\n",
            "  \"radius_of_gyration_range\": {radius_of_gyration_range},\n",
            "  \"chi_square_range\": {chi_square_range},\n",
            "  \"objective_value_range\": {objective_value_range}\n",
            "}}\n"
        ),
        entry_count = scan.entries.len(),
        dmax_range = metric_range_json(scan.stability_summary.dmax_range),
        i_zero_range = metric_range_json(scan.stability_summary.i_zero_range),
        radius_of_gyration_range =
            metric_range_json(scan.stability_summary.radius_of_gyration_range),
        chi_square_range = metric_range_json(scan.stability_summary.chi_square_range),
        objective_value_range = metric_range_json(scan.stability_summary.objective_value_range),
    );

    fs::write(path, contents)?;
    Ok(())
}

fn write_truncation_scan_csv(
    path: impl AsRef<Path>,
    scan: &TruncationScanResult,
) -> Result<(), OutputError> {
    let mut contents = String::from(
        "dropped_point_count,q_min,status,quality_flags,i_zero,radius_of_gyration,chi_square,reduced_chi_square,objective_value,error_message\n",
    );

    for entry in &scan.entries {
        match &entry.outcome {
            TruncationScanOutcome::Successful {
                summary,
                quality_flags,
                ..
            } => {
                let q_min = entry
                    .minimum_retained_q
                    .map(|value| format!("{value:.12e}"))
                    .unwrap_or_default();
                let reduced = summary
                    .reduced_chi_square
                    .map(|value| format!("{value:.12e}"))
                    .unwrap_or_default();
                let flags = quality_flags
                    .iter()
                    .map(|flag| format!("{flag:?}"))
                    .collect::<Vec<_>>()
                    .join("|");

                contents.push_str(&format!(
                    "{},{},success,{},{:.12e},{:.12e},{:.12e},{},{:.12e},\n",
                    entry.dropped_point_count,
                    q_min,
                    flags,
                    summary.i_zero,
                    summary.radius_of_gyration,
                    summary.chi_square,
                    reduced,
                    summary.objective_value
                ));
            }
            TruncationScanOutcome::Failed { error_message } => {
                let q_min = entry
                    .minimum_retained_q
                    .map(|value| format!("{value:.12e}"))
                    .unwrap_or_default();
                contents.push_str(&format!(
                    "{},{},failed,,,,,,,\"{}\"\n",
                    entry.dropped_point_count,
                    q_min,
                    error_message.replace('"', "'")
                ));
            }
        }
    }

    fs::write(path, contents)?;
    Ok(())
}

fn write_truncation_scan_report_json(
    path: impl AsRef<Path>,
    scan: &TruncationScanResult,
) -> Result<(), OutputError> {
    let q_min_range = scan
        .stability_summary
        .q_min_range
        .map(metric_range_json)
        .unwrap_or_else(|| String::from("null"));
    let i_zero_range = scan
        .stability_summary
        .i_zero_range
        .map(metric_range_json)
        .unwrap_or_else(|| String::from("null"));
    let radius_of_gyration_range = scan
        .stability_summary
        .radius_of_gyration_range
        .map(metric_range_json)
        .unwrap_or_else(|| String::from("null"));
    let chi_square_range = scan
        .stability_summary
        .chi_square_range
        .map(metric_range_json)
        .unwrap_or_else(|| String::from("null"));
    let objective_value_range = scan
        .stability_summary
        .objective_value_range
        .map(metric_range_json)
        .unwrap_or_else(|| String::from("null"));

    let contents = format!(
        concat!(
            "{{\n",
            "  \"attempted_entry_count\": {attempted_entry_count},\n",
            "  \"successful_entry_count\": {successful_entry_count},\n",
            "  \"failed_entry_count\": {failed_entry_count},\n",
            "  \"flagged_entry_count\": {flagged_entry_count},\n",
            "  \"dropped_point_range\": {dropped_point_range},\n",
            "  \"q_min_range\": {q_min_range},\n",
            "  \"i_zero_range\": {i_zero_range},\n",
            "  \"radius_of_gyration_range\": {radius_of_gyration_range},\n",
            "  \"chi_square_range\": {chi_square_range},\n",
            "  \"objective_value_range\": {objective_value_range}\n",
            "}}\n"
        ),
        attempted_entry_count = scan.stability_summary.attempted_entry_count,
        successful_entry_count = scan.stability_summary.successful_entry_count,
        failed_entry_count = scan.stability_summary.failed_entry_count,
        flagged_entry_count = scan.stability_summary.flagged_entry_count,
        dropped_point_range = metric_range_json(scan.stability_summary.dropped_point_range),
        q_min_range = q_min_range,
        i_zero_range = i_zero_range,
        radius_of_gyration_range = radius_of_gyration_range,
        chi_square_range = chi_square_range,
        objective_value_range = objective_value_range,
    );

    fs::write(path, contents)?;
    Ok(())
}

fn join_numeric_array(values: &[f64]) -> String {
    let body = values
        .iter()
        .map(|value| format!("{value:.12e}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{body}]")
}

fn metric_range_json(range: crate::analysis::DmaxScanMetricRange) -> String {
    format!(
        "{{\"min\": {:.12e}, \"max\": {:.12e}, \"span\": {:.12e}}}",
        range.min, range.max, range.span
    )
}

#[cfg(test)]
mod tests {
    use super::{write_dmax_scan_outputs, write_fit_outputs, write_truncation_scan_outputs};
    use crate::analysis::{
        DmaxScanConfig, TruncationScanConfig, run_dmax_scan, run_truncation_scan, summarize_fit,
    };
    use crate::basis::CubicBSplineBasis;
    use crate::data::{SaxsCurve, SaxsPoint};
    use crate::solver::solve_curve;
    use crate::transform::ForwardTransform;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn synthetic_curve_from_coefficients(coefficients: &[f64]) -> (SaxsCurve, ForwardTransform) {
        let basis = CubicBSplineBasis::new(8.0, coefficients.len()).unwrap();
        let transform = ForwardTransform::new(basis, 800).unwrap();
        let q_values = (0..18)
            .map(|index| 0.05 + index as f64 * 0.08)
            .collect::<Vec<_>>();
        let intensities = transform.predict(&q_values, coefficients).unwrap();

        let curve = SaxsCurve::new(
            q_values
                .iter()
                .zip(intensities.iter())
                .map(|(&q, &intensity)| SaxsPoint {
                    q,
                    intensity,
                    sigma: 0.05,
                })
                .collect(),
        )
        .unwrap();

        (curve, transform)
    }

    fn temp_output_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("illfit-output-test-{unique}"))
    }

    #[test]
    fn writes_standard_fit_artifacts() {
        let (curve, transform) = synthetic_curve_from_coefficients(&[1.0; 6]);
        let fit = solve_curve(&curve, &transform, 1.0e-8).unwrap();
        let summary = summarize_fit(&curve, &transform, &fit, 41).unwrap();
        let output_dir = temp_output_dir();

        write_fit_outputs(&output_dir, &curve, &transform, &fit, &summary).unwrap();

        let pr_csv = fs::read_to_string(output_dir.join("pr.csv")).unwrap();
        let fit_csv = fs::read_to_string(output_dir.join("fit.csv")).unwrap();
        let residuals_csv = fs::read_to_string(output_dir.join("residuals.csv")).unwrap();
        let report_json = fs::read_to_string(output_dir.join("report.json")).unwrap();

        assert!(pr_csv.starts_with("r,p_of_r\n"));
        assert!(fit_csv.starts_with("q,intensity_measured,sigma,intensity_fitted\n"));
        assert!(residuals_csv.starts_with("q,residual,weighted_residual\n"));
        assert!(report_json.contains("\"i_zero\""));
        assert!(report_json.contains("\"radius_of_gyration\""));
        assert!(report_json.contains("\"coefficients\""));

        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn writes_dmax_scan_artifacts() {
        let (curve, _) = synthetic_curve_from_coefficients(&[1.0; 6]);
        let scan = run_dmax_scan(
            &curve,
            &DmaxScanConfig {
                center_dmax: 8.0,
                half_width: 1.0,
                point_count: 5,
                basis_size: 6,
                integration_intervals: 400,
                lambda: 1.0e-8,
                pr_sample_point_count: 41,
            },
        )
        .unwrap();
        let output_dir = temp_output_dir();

        write_dmax_scan_outputs(&output_dir, &scan).unwrap();

        let scan_csv = fs::read_to_string(output_dir.join("dmax_scan.csv")).unwrap();
        let scan_report = fs::read_to_string(output_dir.join("dmax_scan_report.json")).unwrap();

        assert!(scan_csv.starts_with(
            "dmax,i_zero,radius_of_gyration,chi_square,reduced_chi_square,objective_value\n"
        ));
        assert!(scan_report.contains("\"entry_count\""));
        assert!(scan_report.contains("\"radius_of_gyration_range\""));

        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn writes_truncation_scan_artifacts() {
        let (curve, _) = synthetic_curve_from_coefficients(&[1.0; 6]);
        let scan = run_truncation_scan(
            &curve,
            &TruncationScanConfig {
                dmax: 8.0,
                baseline_drop_count: 10,
                step_size: 5,
                point_count: 5,
                basis_size: 6,
                integration_intervals: 400,
                lambda: 1.0e-8,
                pr_sample_point_count: 41,
            },
        )
        .unwrap();
        let output_dir = temp_output_dir();

        write_truncation_scan_outputs(&output_dir, &scan).unwrap();

        let scan_csv = fs::read_to_string(output_dir.join("truncation_scan.csv")).unwrap();
        let scan_report =
            fs::read_to_string(output_dir.join("truncation_scan_report.json")).unwrap();

        assert!(scan_csv.starts_with(
            "dropped_point_count,q_min,status,quality_flags,i_zero,radius_of_gyration,chi_square,reduced_chi_square,objective_value,error_message\n"
        ));
        assert!(scan_report.contains("\"attempted_entry_count\""));
        assert!(scan_report.contains("\"failed_entry_count\""));

        fs::remove_dir_all(output_dir).unwrap();
    }
}
