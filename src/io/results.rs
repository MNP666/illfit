use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::analysis::{DmaxScanResult, FitSummary, TruncationScanOutcome, TruncationScanResult};
use crate::benchmark::{
    BenchmarkRecoveryComparison, BenchmarkRecoveryResult, BenchmarkSuiteComparison,
    BenchmarkSuiteRecoveryResult, NoisyBenchmarkSuiteRecoveryResult,
};
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

/// Write truth, recovery, and comparison artifacts for one recovered benchmark case.
pub fn write_benchmark_case_outputs(
    output_dir: impl AsRef<Path>,
    recovery: &BenchmarkRecoveryResult,
    comparison: &BenchmarkRecoveryComparison,
) -> Result<(), OutputError> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    write_benchmark_truth_pr_csv(output_dir.join("pr_truth.csv"), recovery)?;
    write_benchmark_truth_iq_csv(output_dir.join("iq_truth.csv"), recovery)?;
    write_benchmark_observed_iq_csv(output_dir.join("iq_observed.csv"), recovery)?;
    write_benchmark_recovered_pr_csv(output_dir.join("pr_recovered.csv"), recovery)?;
    write_benchmark_recovered_iq_csv(output_dir.join("iq_recovered.csv"), recovery)?;
    write_benchmark_pr_comparison_csv(output_dir.join("pr_comparison.csv"), comparison)?;
    write_benchmark_iq_comparison_csv(output_dir.join("iq_comparison.csv"), comparison)?;
    write_benchmark_case_report_json(
        output_dir.join("benchmark_report.json"),
        recovery,
        comparison,
    )?;

    Ok(())
}

/// Write per-case and suite-level artifacts for one recovered benchmark suite.
pub fn write_benchmark_suite_outputs(
    output_dir: impl AsRef<Path>,
    suite_recovery: &BenchmarkSuiteRecoveryResult,
    suite_comparison: &BenchmarkSuiteComparison,
) -> Result<(), OutputError> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    for case_result in &suite_recovery.case_results {
        let comparison = suite_comparison
            .case_comparisons
            .iter()
            .find(|entry| entry.case_id == case_result.truth_case.metadata.candidate_id)
            .expect("benchmark comparison must exist for every recovered case");

        write_benchmark_case_outputs(
            output_dir.join(&case_result.truth_case.metadata.candidate_id),
            case_result,
            comparison,
        )?;
    }

    write_benchmark_suite_summary_csv(
        output_dir.join("benchmark_suite_summary.csv"),
        suite_comparison,
    )?;
    write_benchmark_suite_report_json(
        output_dir.join("benchmark_suite_report.json"),
        suite_recovery,
        suite_comparison,
    )?;

    Ok(())
}

/// Write per-case and suite-level artifacts for one recovered noisy benchmark suite.
pub fn write_noisy_benchmark_suite_outputs(
    output_dir: impl AsRef<Path>,
    suite_recovery: &NoisyBenchmarkSuiteRecoveryResult,
    suite_comparison: &BenchmarkSuiteComparison,
) -> Result<(), OutputError> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    for ((noisy_case, recovery), comparison) in suite_recovery
        .suite
        .cases
        .iter()
        .zip(suite_recovery.case_results.iter())
        .zip(suite_comparison.case_comparisons.iter())
    {
        let case_output_dir = output_dir
            .join(format!(
                "noise_{}",
                format_noise_level(noisy_case.noise_metadata.noise_level)
            ))
            .join(&noisy_case.truth_case.metadata.candidate_id);
        write_benchmark_case_outputs(&case_output_dir, recovery, comparison)?;
        write_noisy_case_metadata_json(case_output_dir.join("noise_metadata.json"), noisy_case)?;
    }

    write_noisy_benchmark_suite_summary_csv(
        output_dir.join("benchmark_suite_summary.csv"),
        suite_recovery,
        suite_comparison,
    )?;
    write_noisy_benchmark_suite_report_json(
        output_dir.join("benchmark_suite_report.json"),
        suite_recovery,
        suite_comparison,
    )?;

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

fn write_benchmark_truth_pr_csv(
    path: impl AsRef<Path>,
    recovery: &BenchmarkRecoveryResult,
) -> Result<(), OutputError> {
    let mut contents = String::from("r,p_of_r_truth\n");
    for point in recovery.truth_case.pr_truth.points() {
        contents.push_str(&format!("{:.12e},{:.12e}\n", point.r, point.p_of_r));
    }
    fs::write(path, contents)?;
    Ok(())
}

fn write_benchmark_truth_iq_csv(
    path: impl AsRef<Path>,
    recovery: &BenchmarkRecoveryResult,
) -> Result<(), OutputError> {
    let mut contents = String::from("q,i_of_q_truth\n");
    for point in recovery.truth_case.iq_truth.points() {
        contents.push_str(&format!("{:.12e},{:.12e}\n", point.q, point.intensity));
    }
    fs::write(path, contents)?;
    Ok(())
}

fn write_benchmark_observed_iq_csv(
    path: impl AsRef<Path>,
    recovery: &BenchmarkRecoveryResult,
) -> Result<(), OutputError> {
    let mut contents = String::from("q,i_of_q_observed,sigma\n");
    for point in recovery.observed_curve.points() {
        contents.push_str(&format!(
            "{:.12e},{:.12e},{:.12e}\n",
            point.q, point.intensity, point.sigma
        ));
    }
    fs::write(path, contents)?;
    Ok(())
}

fn write_benchmark_recovered_pr_csv(
    path: impl AsRef<Path>,
    recovery: &BenchmarkRecoveryResult,
) -> Result<(), OutputError> {
    let mut contents = String::from("r,p_of_r_recovered\n");
    for point in &recovery.summary.sampled_pr {
        contents.push_str(&format!("{:.12e},{:.12e}\n", point.r, point.p_of_r));
    }
    fs::write(path, contents)?;
    Ok(())
}

fn write_benchmark_recovered_iq_csv(
    path: impl AsRef<Path>,
    recovery: &BenchmarkRecoveryResult,
) -> Result<(), OutputError> {
    let mut contents = String::from("q,i_of_q_truth,i_of_q_recovered,residual\n");
    for (truth_point, recovered) in recovery
        .truth_case
        .iq_truth
        .points()
        .iter()
        .zip(recovery.fit.predicted_intensities.iter())
    {
        contents.push_str(&format!(
            "{:.12e},{:.12e},{:.12e},{:.12e}\n",
            truth_point.q,
            truth_point.intensity,
            recovered,
            recovered - truth_point.intensity
        ));
    }
    fs::write(path, contents)?;
    Ok(())
}

fn write_benchmark_pr_comparison_csv(
    path: impl AsRef<Path>,
    comparison: &BenchmarkRecoveryComparison,
) -> Result<(), OutputError> {
    let mut contents = String::from("r,p_of_r_truth,p_of_r_recovered,residual\n");
    for point in &comparison.pr.residual_curve {
        contents.push_str(&format!(
            "{:.12e},{:.12e},{:.12e},{:.12e}\n",
            point.r, point.true_p_of_r, point.recovered_p_of_r, point.residual
        ));
    }
    fs::write(path, contents)?;
    Ok(())
}

fn write_benchmark_iq_comparison_csv(
    path: impl AsRef<Path>,
    comparison: &BenchmarkRecoveryComparison,
) -> Result<(), OutputError> {
    let mut contents = String::from("q,i_of_q_truth,i_of_q_recovered,residual\n");
    for point in &comparison.iq.residual_curve {
        contents.push_str(&format!(
            "{:.12e},{:.12e},{:.12e},{:.12e}\n",
            point.q, point.true_intensity, point.recovered_intensity, point.residual
        ));
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

fn write_benchmark_case_report_json(
    path: impl AsRef<Path>,
    recovery: &BenchmarkRecoveryResult,
    comparison: &BenchmarkRecoveryComparison,
) -> Result<(), OutputError> {
    let weights_json = join_numeric_array(&recovery.truth_case.metadata.weights);
    let contents = format!(
        concat!(
            "{{\n",
            "  \"case_id\": \"{case_id}\",\n",
            "  \"family\": \"{family}\",\n",
            "  \"seed\": {seed},\n",
            "  \"truth_rg\": {truth_rg:.12e},\n",
            "  \"recovered_rg\": {recovered_rg:.12e},\n",
            "  \"truth_i_zero\": {truth_i_zero:.12e},\n",
            "  \"recovered_i_zero\": {recovered_i_zero:.12e},\n",
            "  \"pr_rmse\": {pr_rmse:.12e},\n",
            "  \"pr_normalized_rmse\": {pr_normalized_rmse:.12e},\n",
            "  \"pr_correlation\": {pr_correlation:.12e},\n",
            "  \"pr_integrated_absolute_error\": {pr_iae:.12e},\n",
            "  \"q_rmse\": {q_rmse:.12e},\n",
            "  \"q_normalized_rmse\": {q_normalized_rmse:.12e},\n",
            "  \"chi_square\": {chi_square:.12e},\n",
            "  \"objective_value\": {objective_value:.12e},\n",
            "  \"weights\": {weights}\n",
            "}}\n"
        ),
        case_id = recovery.truth_case.metadata.candidate_id,
        family = recovery.truth_case.metadata.family,
        seed = recovery.truth_case.metadata.seed,
        truth_rg = recovery.truth_case.metadata.rg,
        recovered_rg = recovery.summary.radius_of_gyration,
        truth_i_zero = recovery.truth_case.metadata.i_zero,
        recovered_i_zero = recovery.summary.i_zero,
        pr_rmse = comparison.pr.rmse,
        pr_normalized_rmse = comparison.pr.normalized_rmse,
        pr_correlation = comparison.pr.correlation,
        pr_iae = comparison.pr.integrated_absolute_error,
        q_rmse = comparison.iq.rmse,
        q_normalized_rmse = comparison.iq.normalized_rmse,
        chi_square = recovery.summary.chi_square,
        objective_value = recovery.summary.objective_value,
        weights = weights_json,
    );

    fs::write(path, contents)?;
    Ok(())
}

fn write_benchmark_suite_summary_csv(
    path: impl AsRef<Path>,
    suite_comparison: &BenchmarkSuiteComparison,
) -> Result<(), OutputError> {
    let mut contents = String::from(
        "case_id,pr_rmse,pr_normalized_rmse,pr_correlation,pr_integrated_absolute_error,rg_error,i_zero_error,q_rmse,q_normalized_rmse\n",
    );

    for case in &suite_comparison.case_comparisons {
        contents.push_str(&format!(
            "{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}\n",
            case.case_id,
            case.pr.rmse,
            case.pr.normalized_rmse,
            case.pr.correlation,
            case.pr.integrated_absolute_error,
            case.pr.radius_of_gyration_error,
            case.pr.i_zero_error,
            case.iq.rmse,
            case.iq.normalized_rmse
        ));
    }

    fs::write(path, contents)?;
    Ok(())
}

fn write_benchmark_suite_report_json(
    path: impl AsRef<Path>,
    suite_recovery: &BenchmarkSuiteRecoveryResult,
    suite_comparison: &BenchmarkSuiteComparison,
) -> Result<(), OutputError> {
    let pr_rmse_values = suite_comparison
        .case_comparisons
        .iter()
        .map(|case| case.pr.rmse)
        .collect::<Vec<_>>();
    let q_rmse_values = suite_comparison
        .case_comparisons
        .iter()
        .map(|case| case.iq.rmse)
        .collect::<Vec<_>>();

    let contents = format!(
        concat!(
            "{{\n",
            "  \"suite_name\": \"{suite_name}\",\n",
            "  \"case_count\": {case_count},\n",
            "  \"accepted_count\": {accepted_count},\n",
            "  \"pr_rmse_range\": {pr_rmse_range},\n",
            "  \"q_rmse_range\": {q_rmse_range}\n",
            "}}\n"
        ),
        suite_name = suite_comparison.suite_name,
        case_count = suite_comparison.case_comparisons.len(),
        accepted_count = suite_recovery.suite.summary.accepted_count,
        pr_rmse_range = tuple_metric_range_json(metric_range(&pr_rmse_values)),
        q_rmse_range = tuple_metric_range_json(metric_range(&q_rmse_values)),
    );

    fs::write(path, contents)?;
    Ok(())
}

fn write_noisy_case_metadata_json(
    path: impl AsRef<Path>,
    noisy_case: &crate::benchmark::NoisyBenchmarkCase,
) -> Result<(), OutputError> {
    let contents = format!(
        concat!(
            "{{\n",
            "  \"case_id\": \"{case_id}\",\n",
            "  \"family\": \"{family}\",\n",
            "  \"noise_level\": {noise_level:.12e},\n",
            "  \"negative_value_count\": {negative_value_count},\n",
            "  \"negative_value_fraction\": {negative_value_fraction:.12e},\n",
            "  \"min_observed_intensity\": {min_observed_intensity:.12e},\n",
            "  \"max_observed_intensity\": {max_observed_intensity:.12e}\n",
            "}}\n"
        ),
        case_id = noisy_case.noise_metadata.case_id,
        family = noisy_case.noise_metadata.family,
        noise_level = noisy_case.noise_metadata.noise_level,
        negative_value_count = noisy_case.noise_metadata.negative_value_count,
        negative_value_fraction = noisy_case.noise_metadata.negative_value_fraction,
        min_observed_intensity = noisy_case.noise_metadata.min_observed_intensity,
        max_observed_intensity = noisy_case.noise_metadata.max_observed_intensity,
    );
    fs::write(path, contents)?;
    Ok(())
}

fn write_noisy_benchmark_suite_summary_csv(
    path: impl AsRef<Path>,
    suite_recovery: &NoisyBenchmarkSuiteRecoveryResult,
    suite_comparison: &BenchmarkSuiteComparison,
) -> Result<(), OutputError> {
    let mut contents = String::from(
        "case_id,noise_level,negative_value_fraction,pr_rmse,pr_normalized_rmse,pr_correlation,q_rmse,q_normalized_rmse\n",
    );

    for (noisy_case, comparison) in suite_recovery
        .suite
        .cases
        .iter()
        .zip(suite_comparison.case_comparisons.iter())
    {
        contents.push_str(&format!(
            "{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}\n",
            noisy_case.truth_case.metadata.candidate_id,
            noisy_case.noise_metadata.noise_level,
            noisy_case.noise_metadata.negative_value_fraction,
            comparison.pr.rmse,
            comparison.pr.normalized_rmse,
            comparison.pr.correlation,
            comparison.iq.rmse,
            comparison.iq.normalized_rmse,
        ));
    }

    fs::write(path, contents)?;
    Ok(())
}

fn write_noisy_benchmark_suite_report_json(
    path: impl AsRef<Path>,
    suite_recovery: &NoisyBenchmarkSuiteRecoveryResult,
    suite_comparison: &BenchmarkSuiteComparison,
) -> Result<(), OutputError> {
    let negative_fraction_values = suite_recovery
        .suite
        .cases
        .iter()
        .map(|case| case.noise_metadata.negative_value_fraction)
        .collect::<Vec<_>>();
    let q_rmse_values = suite_comparison
        .case_comparisons
        .iter()
        .map(|case| case.iq.rmse)
        .collect::<Vec<_>>();

    let contents = format!(
        concat!(
            "{{\n",
            "  \"suite_name\": \"{suite_name}\",\n",
            "  \"variant_count\": {variant_count},\n",
            "  \"noise_levels\": {noise_levels},\n",
            "  \"negative_fraction_range\": {negative_fraction_range},\n",
            "  \"q_rmse_range\": {q_rmse_range}\n",
            "}}\n"
        ),
        suite_name = suite_comparison.suite_name,
        variant_count = suite_recovery.suite.cases.len(),
        noise_levels = join_numeric_array(&suite_recovery.suite.summary.noise_levels),
        negative_fraction_range = tuple_metric_range_json(metric_range(&negative_fraction_values)),
        q_rmse_range = tuple_metric_range_json(metric_range(&q_rmse_values)),
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

fn metric_range(values: &[f64]) -> (f64, f64) {
    let min_value = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max_value = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (min_value, max_value)
}

fn format_noise_level(noise_level: f64) -> String {
    let formatted = format!("{noise_level:.12}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn tuple_metric_range_json(range: (f64, f64)) -> String {
    let (min, max) = range;
    format!(
        "{{\"min\": {:.12e}, \"max\": {:.12e}, \"span\": {:.12e}}}",
        min,
        max,
        max - min
    )
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
    use super::{
        write_benchmark_case_outputs, write_benchmark_suite_outputs, write_dmax_scan_outputs,
        write_fit_outputs, write_truncation_scan_outputs,
    };
    use crate::analysis::{
        DmaxScanConfig, TruncationScanConfig, run_dmax_scan, run_truncation_scan, summarize_fit,
    };
    use crate::basis::CubicBSplineBasis;
    use crate::benchmark::{
        BenchmarkRecoveryConfig, compare_benchmark_recovery, compare_benchmark_suite,
        load_benchmark_suite, recover_benchmark_suite, recover_benchmark_truth_case,
    };
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

    fn synthetic_suite_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("regression")
            .join("clamped_spline")
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

    #[test]
    fn writes_benchmark_case_artifacts() {
        let suite = load_benchmark_suite(synthetic_suite_path()).unwrap();
        let recovery = recover_benchmark_truth_case(
            &suite.truth_cases[0],
            BenchmarkRecoveryConfig {
                dmax: suite.summary.config.dmax,
                basis_size: suite.summary.config.n_weights + 2,
                integration_intervals: suite.summary.config.integration_intervals,
                lambda: 1.0e-2,
                pr_sample_points: suite.summary.config.r_points,
                synthetic_sigma: 0.05,
            },
        )
        .unwrap();
        let comparison = compare_benchmark_recovery(&recovery).unwrap();
        let output_dir = temp_output_dir();

        write_benchmark_case_outputs(&output_dir, &recovery, &comparison).unwrap();

        let truth_pr = fs::read_to_string(output_dir.join("pr_truth.csv")).unwrap();
        let truth_iq = fs::read_to_string(output_dir.join("iq_truth.csv")).unwrap();
        let recovered_pr = fs::read_to_string(output_dir.join("pr_recovered.csv")).unwrap();
        let comparison_pr = fs::read_to_string(output_dir.join("pr_comparison.csv")).unwrap();
        let report = fs::read_to_string(output_dir.join("benchmark_report.json")).unwrap();

        assert!(truth_pr.starts_with("r,p_of_r_truth\n"));
        assert!(truth_iq.starts_with("q,i_of_q_truth\n"));
        assert!(recovered_pr.starts_with("r,p_of_r_recovered\n"));
        assert!(comparison_pr.starts_with("r,p_of_r_truth,p_of_r_recovered,residual\n"));
        assert!(report.contains("\"case_id\""));
        assert!(report.contains("\"pr_rmse\""));
        assert!(report.contains("\"q_rmse\""));

        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn writes_benchmark_suite_artifacts() {
        let suite = load_benchmark_suite(synthetic_suite_path()).unwrap();
        let suite_recovery = recover_benchmark_suite(
            &suite,
            BenchmarkRecoveryConfig {
                dmax: suite.summary.config.dmax,
                basis_size: suite.summary.config.n_weights + 2,
                integration_intervals: suite.summary.config.integration_intervals,
                lambda: 1.0e-2,
                pr_sample_points: suite.summary.config.r_points,
                synthetic_sigma: 0.05,
            },
        )
        .unwrap();
        let suite_comparison = compare_benchmark_suite(&suite_recovery).unwrap();
        let output_dir = temp_output_dir();

        write_benchmark_suite_outputs(&output_dir, &suite_recovery, &suite_comparison).unwrap();

        let suite_summary =
            fs::read_to_string(output_dir.join("benchmark_suite_summary.csv")).unwrap();
        let suite_report =
            fs::read_to_string(output_dir.join("benchmark_suite_report.json")).unwrap();
        let first_case_dir = output_dir.join(
            &suite_recovery.case_results[0]
                .truth_case
                .metadata
                .candidate_id,
        );
        let first_case_report =
            fs::read_to_string(first_case_dir.join("benchmark_report.json")).unwrap();

        assert!(suite_summary.starts_with(
            "case_id,pr_rmse,pr_normalized_rmse,pr_correlation,pr_integrated_absolute_error,rg_error,i_zero_error,q_rmse,q_normalized_rmse\n"
        ));
        assert!(suite_report.contains("\"suite_name\""));
        assert!(suite_report.contains("\"pr_rmse_range\""));
        assert!(first_case_report.contains("\"case_id\""));

        fs::remove_dir_all(output_dir).unwrap();
    }
}
