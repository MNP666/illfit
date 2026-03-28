use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::analysis::FitSummary;
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

fn join_numeric_array(values: &[f64]) -> String {
    let body = values
        .iter()
        .map(|value| format!("{value:.12e}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{body}]")
}

#[cfg(test)]
mod tests {
    use super::write_fit_outputs;
    use crate::analysis::summarize_fit;
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
}
