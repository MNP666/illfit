use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use illfit::analysis::{
    DmaxScanConfig, TruncationScanConfig, run_dmax_scan, run_truncation_scan, summarize_fit,
};
use illfit::basis::CubicBSplineBasis;
use illfit::data::parse_ascii_curve_file;
use illfit::io::{write_dmax_scan_outputs, write_fit_outputs, write_truncation_scan_outputs};
use illfit::solver::solve_curve;
use illfit::transform::ForwardTransform;

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), CliError> {
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Err(CliError::Usage(usage_text()));
    };

    let rest = args.collect::<Vec<_>>();
    match command.as_str() {
        "fit" => run_fit(rest),
        "scan-dmax" => run_scan_dmax(rest),
        "scan-truncation" => run_scan_truncation(rest),
        "--help" | "-h" | "help" => Err(CliError::Usage(usage_text())),
        other => Err(CliError::Message(format!(
            "unknown command `{other}`\n\n{}",
            usage_text()
        ))),
    }
}

#[derive(Debug)]
pub enum CliError {
    Message(String),
    Usage(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) | Self::Usage(message) => write!(f, "{message}"),
        }
    }
}

impl Error for CliError {}

fn run_fit(args: Vec<String>) -> Result<(), CliError> {
    let parsed = ParsedArgs::parse(args)?;

    if parsed.flag_present("help") {
        return Err(CliError::Usage(fit_usage()));
    }

    let data_path = parsed.require_path("data")?;
    let output_dir = parsed.require_path("output-dir")?;
    let dmax = parsed.require_f64("dmax")?;
    let basis_size = parsed.require_usize("basis-size")?;
    let integration_intervals = parsed.require_usize("integration-intervals")?;
    let lambda = parsed.require_f64("lambda")?;
    let pr_sample_points = parsed.require_usize("pr-sample-points")?;
    let drop_first = parsed.optional_usize("drop-first")?.unwrap_or(0);

    let curve = parse_ascii_curve_file(&data_path)
        .map_err(|error| CliError::Message(format!("failed to load data: {error}")))?;
    let curve = curve
        .truncate_front(drop_first)
        .map_err(|error| CliError::Message(format!("failed to truncate input curve: {error}")))?;
    let basis = CubicBSplineBasis::new(dmax, basis_size)
        .map_err(|error| CliError::Message(format!("failed to build basis: {error}")))?;
    let transform = ForwardTransform::new(basis, integration_intervals)
        .map_err(|error| CliError::Message(format!("failed to build transform: {error}")))?;
    let fit = solve_curve(&curve, &transform, lambda)
        .map_err(|error| CliError::Message(format!("fit failed: {error}")))?;
    let summary = summarize_fit(&curve, &transform, &fit, pr_sample_points)
        .map_err(|error| CliError::Message(format!("failed to summarize fit: {error}")))?;

    write_fit_outputs(output_dir, &curve, &transform, &fit, &summary)
        .map_err(|error| CliError::Message(format!("failed to write outputs: {error}")))?;

    Ok(())
}

fn run_scan_dmax(args: Vec<String>) -> Result<(), CliError> {
    let parsed = ParsedArgs::parse(args)?;

    if parsed.flag_present("help") {
        return Err(CliError::Usage(scan_dmax_usage()));
    }

    let data_path = parsed.require_path("data")?;
    let output_dir = parsed.require_path("output-dir")?;
    let drop_first = parsed.optional_usize("drop-first")?.unwrap_or(0);
    let curve = parse_ascii_curve_file(&data_path)
        .map_err(|error| CliError::Message(format!("failed to load data: {error}")))?;
    let curve = curve
        .truncate_front(drop_first)
        .map_err(|error| CliError::Message(format!("failed to truncate input curve: {error}")))?;

    let config = DmaxScanConfig {
        center_dmax: parsed.require_f64("center-dmax")?,
        half_width: parsed.require_f64("half-width")?,
        point_count: parsed.require_usize("point-count")?,
        basis_size: parsed.require_usize("basis-size")?,
        integration_intervals: parsed.require_usize("integration-intervals")?,
        lambda: parsed.require_f64("lambda")?,
        pr_sample_point_count: parsed.require_usize("pr-sample-points")?,
    };

    let scan = run_dmax_scan(&curve, &config)
        .map_err(|error| CliError::Message(format!("Dmax scan failed: {error}")))?;
    write_dmax_scan_outputs(output_dir, &scan)
        .map_err(|error| CliError::Message(format!("failed to write scan outputs: {error}")))?;

    Ok(())
}

fn run_scan_truncation(args: Vec<String>) -> Result<(), CliError> {
    let parsed = ParsedArgs::parse(args)?;

    if parsed.flag_present("help") {
        return Err(CliError::Usage(scan_truncation_usage()));
    }

    let data_path = parsed.require_path("data")?;
    let output_dir = parsed.require_path("output-dir")?;
    let curve = parse_ascii_curve_file(&data_path)
        .map_err(|error| CliError::Message(format!("failed to load data: {error}")))?;

    let config = TruncationScanConfig {
        dmax: parsed.require_f64("dmax")?,
        baseline_drop_count: parsed.require_usize("baseline-drop-count")?,
        step_size: parsed.require_usize("step-size")?,
        point_count: parsed.require_usize("point-count")?,
        basis_size: parsed.require_usize("basis-size")?,
        integration_intervals: parsed.require_usize("integration-intervals")?,
        lambda: parsed.require_f64("lambda")?,
        pr_sample_point_count: parsed.require_usize("pr-sample-points")?,
    };

    let scan = run_truncation_scan(&curve, &config)
        .map_err(|error| CliError::Message(format!("truncation scan failed: {error}")))?;
    write_truncation_scan_outputs(output_dir, &scan)
        .map_err(|error| CliError::Message(format!("failed to write scan outputs: {error}")))?;

    Ok(())
}

#[derive(Debug, Default)]
struct ParsedArgs {
    entries: Vec<(String, String)>,
}

impl ParsedArgs {
    fn parse(args: Vec<String>) -> Result<Self, CliError> {
        let mut entries = Vec::new();
        let mut index = 0;

        while index < args.len() {
            let current = &args[index];
            if !current.starts_with("--") {
                return Err(CliError::Message(format!(
                    "expected a `--flag value` pair, found `{current}`"
                )));
            }

            let key = current.trim_start_matches("--").to_string();
            if key == "help" || key == "h" {
                entries.push(("help".to_string(), "true".to_string()));
                index += 1;
                continue;
            }

            let Some(value) = args.get(index + 1) else {
                return Err(CliError::Message(format!(
                    "flag `--{key}` requires a value"
                )));
            };

            if value.starts_with("--") {
                return Err(CliError::Message(format!(
                    "flag `--{key}` requires a value"
                )));
            }

            entries.push((key, value.clone()));
            index += 2;
        }

        Ok(Self { entries })
    }

    fn flag_present(&self, key: &str) -> bool {
        self.entries.iter().any(|(entry_key, _)| entry_key == key)
    }

    fn require_string(&self, key: &str) -> Result<String, CliError> {
        self.entries
            .iter()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| CliError::Message(format!("missing required flag `--{key}`")))
    }

    fn require_path(&self, key: &str) -> Result<PathBuf, CliError> {
        Ok(PathBuf::from(self.require_string(key)?))
    }

    fn require_f64(&self, key: &str) -> Result<f64, CliError> {
        let raw = self.require_string(key)?;
        raw.parse::<f64>().map_err(|_| {
            CliError::Message(format!(
                "flag `--{key}` must be a floating-point value, found `{raw}`"
            ))
        })
    }

    fn require_usize(&self, key: &str) -> Result<usize, CliError> {
        let raw = self.require_string(key)?;
        raw.parse::<usize>().map_err(|_| {
            CliError::Message(format!(
                "flag `--{key}` must be a non-negative integer, found `{raw}`"
            ))
        })
    }

    fn optional_usize(&self, key: &str) -> Result<Option<usize>, CliError> {
        let maybe = self
            .entries
            .iter()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value.clone());

        maybe
            .map(|raw| {
                raw.parse::<usize>().map_err(|_| {
                    CliError::Message(format!(
                        "flag `--{key}` must be a non-negative integer, found `{raw}`"
                    ))
                })
            })
            .transpose()
    }
}

fn usage_text() -> String {
    format!(
        "{}\n\n{}\n\n{}\n\n{}",
        "Usage:",
        fit_usage(),
        scan_dmax_usage(),
        scan_truncation_usage()
    )
}

fn fit_usage() -> String {
    String::from(
        "illfit fit --data <path> --dmax <float> --basis-size <usize> --integration-intervals <usize> --lambda <float> --pr-sample-points <usize> --output-dir <path> [--drop-first <usize>]",
    )
}

fn scan_dmax_usage() -> String {
    String::from(
        "illfit scan-dmax --data <path> --center-dmax <float> --half-width <float> --point-count <usize> --basis-size <usize> --integration-intervals <usize> --lambda <float> --pr-sample-points <usize> --output-dir <path> [--drop-first <usize>]",
    )
}

fn scan_truncation_usage() -> String {
    String::from(
        "illfit scan-truncation --data <path> --dmax <float> --baseline-drop-count <usize> --step-size <usize> --point-count <usize> --basis-size <usize> --integration-intervals <usize> --lambda <float> --pr-sample-points <usize> --output-dir <path>",
    )
}

#[cfg(test)]
mod tests {
    use super::{ParsedArgs, fit_usage, run};

    #[test]
    fn parses_flag_value_pairs() {
        let parsed = ParsedArgs::parse(vec![
            "--data".to_string(),
            "curve.dat".to_string(),
            "--dmax".to_string(),
            "80.0".to_string(),
        ])
        .unwrap();

        assert_eq!(parsed.require_string("data").unwrap(), "curve.dat");
        assert_eq!(parsed.require_f64("dmax").unwrap(), 80.0);
    }

    #[test]
    fn rejects_missing_flag_value() {
        let error = ParsedArgs::parse(vec!["--data".to_string()]).unwrap_err();
        assert!(error.to_string().contains("requires a value"));
    }

    #[test]
    fn returns_usage_when_no_command_is_given() {
        let error = run(vec!["illfit".to_string()]).unwrap_err();
        assert!(error.to_string().contains("Usage:"));
    }

    #[test]
    fn returns_command_specific_help() {
        let error = run(vec![
            "illfit".to_string(),
            "fit".to_string(),
            "--help".to_string(),
        ])
        .unwrap_err();
        assert_eq!(error.to_string(), fit_usage());
    }
}
