use serde::Deserialize;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::weighting::{WeightingParseError, WeightingStrategy};

#[derive(Debug, Clone, PartialEq)]
pub enum ExperimentSuiteKind {
    Benchmark,
    NoisyBenchmark,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentSuiteConfig {
    pub kind: ExperimentSuiteKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentRecoveryConfig {
    pub dmax: f64,
    pub basis_size: usize,
    pub integration_intervals: usize,
    pub pr_sample_points: usize,
    pub synthetic_sigma: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaGridConfig {
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LambdaSelectorMethod {
    LCurve,
    Gcv,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentOutputConfig {
    pub run_name: String,
    pub root_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentConfig {
    pub suite: ExperimentSuiteConfig,
    pub recovery: ExperimentRecoveryConfig,
    pub weighting_strategies: Vec<WeightingStrategy>,
    pub lambda_grid: LambdaGridConfig,
    pub selectors: Vec<LambdaSelectorMethod>,
    pub output: ExperimentOutputConfig,
}

impl ExperimentConfig {
    pub fn to_toml_string(&self) -> String {
        let weighting = self
            .weighting_strategies
            .iter()
            .map(|strategy| format!("\"{}\"", strategy.as_config_string()))
            .collect::<Vec<_>>()
            .join(", ");
        let lambda_values = self
            .lambda_grid
            .values
            .iter()
            .map(|value| format!("{value:.12e}"))
            .collect::<Vec<_>>()
            .join(", ");
        let selectors = self
            .selectors
            .iter()
            .map(|selector| match selector {
                LambdaSelectorMethod::LCurve => "\"l_curve\"".to_string(),
                LambdaSelectorMethod::Gcv => "\"gcv\"".to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let suite_kind = match self.suite.kind {
            ExperimentSuiteKind::Benchmark => "benchmark",
            ExperimentSuiteKind::NoisyBenchmark => "noisy_benchmark",
        };

        format!(
            concat!(
                "[suite]\n",
                "kind = \"{suite_kind}\"\n",
                "path = \"{suite_path}\"\n\n",
                "[recovery]\n",
                "dmax = {dmax:.12e}\n",
                "basis_size = {basis_size}\n",
                "integration_intervals = {integration_intervals}\n",
                "pr_sample_points = {pr_sample_points}\n",
                "synthetic_sigma = {synthetic_sigma:.12e}\n\n",
                "[weighting]\n",
                "strategies = [{weighting}]\n\n",
                "[lambda]\n",
                "values = [{lambda_values}]\n\n",
                "[selectors]\n",
                "methods = [{selectors}]\n\n",
                "[output]\n",
                "run_name = \"{run_name}\"\n",
                "root_dir = \"{root_dir}\"\n"
            ),
            suite_kind = suite_kind,
            suite_path = self.suite.path.display(),
            dmax = self.recovery.dmax,
            basis_size = self.recovery.basis_size,
            integration_intervals = self.recovery.integration_intervals,
            pr_sample_points = self.recovery.pr_sample_points,
            synthetic_sigma = self.recovery.synthetic_sigma,
            weighting = weighting,
            lambda_values = lambda_values,
            selectors = selectors,
            run_name = self.output.run_name,
            root_dir = self.output.root_dir.display(),
        )
    }
}

#[derive(Debug)]
pub enum ExperimentConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    InvalidSuiteKind { value: String },
    InvalidLambdaSelector { value: String },
    InvalidWeightingStrategy(WeightingParseError),
    EmptyWeightingStrategies,
    EmptyLambdaGrid,
    EmptySelectors,
    InvalidLambdaValue { lambda: f64 },
    InvalidDmax { dmax: f64 },
    InvalidBasisSize { basis_size: usize },
    InvalidIntegrationIntervals { integration_intervals: usize },
    InvalidPrSamplePoints { pr_sample_points: usize },
    InvalidSyntheticSigma { synthetic_sigma: f64 },
    EmptyRunName,
}

impl fmt::Display for ExperimentConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read experiment config: {error}"),
            Self::Toml(error) => write!(f, "failed to parse experiment TOML: {error}"),
            Self::InvalidSuiteKind { value } => write!(
                f,
                "unsupported suite kind `{value}`; expected `benchmark` or `noisy_benchmark`"
            ),
            Self::InvalidLambdaSelector { value } => write!(
                f,
                "unsupported lambda selector `{value}`; expected `l_curve` or `gcv`"
            ),
            Self::InvalidWeightingStrategy(error) => write!(f, "{error}"),
            Self::EmptyWeightingStrategies => {
                write!(
                    f,
                    "experiment config must include at least one weighting strategy"
                )
            }
            Self::EmptyLambdaGrid => {
                write!(
                    f,
                    "experiment config must include at least one lambda value"
                )
            }
            Self::EmptySelectors => {
                write!(
                    f,
                    "experiment config must include at least one lambda selector"
                )
            }
            Self::InvalidLambdaValue { lambda } => write!(
                f,
                "lambda values must be finite and non-negative, but found {lambda}"
            ),
            Self::InvalidDmax { dmax } => {
                write!(f, "dmax must be finite and positive, but was {dmax}")
            }
            Self::InvalidBasisSize { basis_size } => {
                write!(f, "basis_size must be at least 3, but was {basis_size}")
            }
            Self::InvalidIntegrationIntervals {
                integration_intervals,
            } => write!(
                f,
                "integration_intervals must be positive, but was {integration_intervals}"
            ),
            Self::InvalidPrSamplePoints { pr_sample_points } => write!(
                f,
                "pr_sample_points must be at least 2, but was {pr_sample_points}"
            ),
            Self::InvalidSyntheticSigma { synthetic_sigma } => write!(
                f,
                "synthetic_sigma must be finite and positive, but was {synthetic_sigma}"
            ),
            Self::EmptyRunName => write!(f, "output.run_name must not be empty"),
        }
    }
}

impl Error for ExperimentConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Toml(error) => Some(error),
            Self::InvalidWeightingStrategy(error) => Some(error),
            Self::InvalidSuiteKind { .. }
            | Self::InvalidLambdaSelector { .. }
            | Self::EmptyWeightingStrategies
            | Self::EmptyLambdaGrid
            | Self::EmptySelectors
            | Self::InvalidLambdaValue { .. }
            | Self::InvalidDmax { .. }
            | Self::InvalidBasisSize { .. }
            | Self::InvalidIntegrationIntervals { .. }
            | Self::InvalidPrSamplePoints { .. }
            | Self::InvalidSyntheticSigma { .. }
            | Self::EmptyRunName => None,
        }
    }
}

impl From<std::io::Error> for ExperimentConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<toml::de::Error> for ExperimentConfigError {
    fn from(value: toml::de::Error) -> Self {
        Self::Toml(value)
    }
}

impl From<WeightingParseError> for ExperimentConfigError {
    fn from(value: WeightingParseError) -> Self {
        Self::InvalidWeightingStrategy(value)
    }
}

#[derive(Debug, Deserialize)]
struct RawExperimentConfig {
    suite: RawSuiteConfig,
    recovery: RawRecoveryConfig,
    weighting: RawWeightingConfig,
    lambda: RawLambdaConfig,
    selectors: RawSelectorsConfig,
    output: RawOutputConfig,
}

#[derive(Debug, Deserialize)]
struct RawSuiteConfig {
    kind: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct RawRecoveryConfig {
    dmax: f64,
    basis_size: usize,
    integration_intervals: usize,
    pr_sample_points: usize,
    synthetic_sigma: f64,
}

#[derive(Debug, Deserialize)]
struct RawWeightingConfig {
    strategies: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawLambdaConfig {
    values: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct RawSelectorsConfig {
    methods: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawOutputConfig {
    run_name: String,
    root_dir: String,
}

pub fn parse_experiment_config(
    path: impl AsRef<Path>,
) -> Result<ExperimentConfig, ExperimentConfigError> {
    let contents = fs::read_to_string(path)?;
    parse_experiment_config_str(&contents)
}

pub fn parse_experiment_config_str(
    contents: &str,
) -> Result<ExperimentConfig, ExperimentConfigError> {
    let raw: RawExperimentConfig = toml::from_str(contents)?;
    let suite_kind = match raw.suite.kind.as_str() {
        "benchmark" => ExperimentSuiteKind::Benchmark,
        "noisy_benchmark" => ExperimentSuiteKind::NoisyBenchmark,
        other => {
            return Err(ExperimentConfigError::InvalidSuiteKind {
                value: other.to_string(),
            });
        }
    };

    let weighting_strategies = raw
        .weighting
        .strategies
        .into_iter()
        .map(|value| value.parse::<WeightingStrategy>())
        .collect::<Result<Vec<_>, _>>()?;
    if weighting_strategies.is_empty() {
        return Err(ExperimentConfigError::EmptyWeightingStrategies);
    }

    if raw.lambda.values.is_empty() {
        return Err(ExperimentConfigError::EmptyLambdaGrid);
    }
    for &lambda in &raw.lambda.values {
        if !lambda.is_finite() || lambda < 0.0 {
            return Err(ExperimentConfigError::InvalidLambdaValue { lambda });
        }
    }

    let selectors = raw
        .selectors
        .methods
        .into_iter()
        .map(|value| match value.as_str() {
            "l_curve" => Ok(LambdaSelectorMethod::LCurve),
            "gcv" => Ok(LambdaSelectorMethod::Gcv),
            other => Err(ExperimentConfigError::InvalidLambdaSelector {
                value: other.to_string(),
            }),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if selectors.is_empty() {
        return Err(ExperimentConfigError::EmptySelectors);
    }

    if !raw.recovery.dmax.is_finite() || raw.recovery.dmax <= 0.0 {
        return Err(ExperimentConfigError::InvalidDmax {
            dmax: raw.recovery.dmax,
        });
    }
    if raw.recovery.basis_size < 3 {
        return Err(ExperimentConfigError::InvalidBasisSize {
            basis_size: raw.recovery.basis_size,
        });
    }
    if raw.recovery.integration_intervals == 0 {
        return Err(ExperimentConfigError::InvalidIntegrationIntervals {
            integration_intervals: raw.recovery.integration_intervals,
        });
    }
    if raw.recovery.pr_sample_points < 2 {
        return Err(ExperimentConfigError::InvalidPrSamplePoints {
            pr_sample_points: raw.recovery.pr_sample_points,
        });
    }
    if !raw.recovery.synthetic_sigma.is_finite() || raw.recovery.synthetic_sigma <= 0.0 {
        return Err(ExperimentConfigError::InvalidSyntheticSigma {
            synthetic_sigma: raw.recovery.synthetic_sigma,
        });
    }
    if raw.output.run_name.trim().is_empty() {
        return Err(ExperimentConfigError::EmptyRunName);
    }

    Ok(ExperimentConfig {
        suite: ExperimentSuiteConfig {
            kind: suite_kind,
            path: PathBuf::from(raw.suite.path),
        },
        recovery: ExperimentRecoveryConfig {
            dmax: raw.recovery.dmax,
            basis_size: raw.recovery.basis_size,
            integration_intervals: raw.recovery.integration_intervals,
            pr_sample_points: raw.recovery.pr_sample_points,
            synthetic_sigma: raw.recovery.synthetic_sigma,
        },
        weighting_strategies,
        lambda_grid: LambdaGridConfig {
            values: raw.lambda.values,
        },
        selectors,
        output: ExperimentOutputConfig {
            run_name: raw.output.run_name,
            root_dir: PathBuf::from(raw.output.root_dir),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ExperimentConfigError, ExperimentSuiteKind, LambdaSelectorMethod,
        parse_experiment_config_str,
    };
    use crate::weighting::WeightingStrategy;

    fn example_config() -> &'static str {
        r#"
[suite]
kind = "benchmark"
path = "data/regression/clamped_spline"

[recovery]
dmax = 120.0
basis_size = 7
integration_intervals = 1600
pr_sample_points = 400
synthetic_sigma = 0.05

[weighting]
strategies = ["none", "q", "q2"]

[lambda]
values = [1e-4, 1e-3, 1e-2]

[selectors]
methods = ["l_curve", "gcv"]

[output]
run_name = "baseline_weighting_scan"
root_dir = "profiling/output"
"#
    }

    #[test]
    fn parses_valid_experiment_config() {
        let config = parse_experiment_config_str(example_config()).unwrap();

        assert_eq!(config.suite.kind, ExperimentSuiteKind::Benchmark);
        assert_eq!(config.weighting_strategies.len(), 3);
        assert_eq!(config.weighting_strategies[1], WeightingStrategy::Q);
        assert_eq!(
            config.selectors,
            vec![LambdaSelectorMethod::LCurve, LambdaSelectorMethod::Gcv]
        );
        assert_eq!(config.lambda_grid.values.len(), 3);
    }

    #[test]
    fn rejects_empty_weighting_list() {
        let error = parse_experiment_config_str(
            &example_config().replace("strategies = [\"none\", \"q\", \"q2\"]", "strategies = []"),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ExperimentConfigError::EmptyWeightingStrategies
        ));
    }

    #[test]
    fn rejects_invalid_lambda_values() {
        let error = parse_experiment_config_str(
            &example_config().replace("values = [1e-4, 1e-3, 1e-2]", "values = [1e-4, -1.0]"),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ExperimentConfigError::InvalidLambdaValue { .. }
        ));
    }

    #[test]
    fn rejects_unknown_selector() {
        let error = parse_experiment_config_str(
            &example_config().replace("methods = [\"l_curve\", \"gcv\"]", "methods = [\"banana\"]"),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ExperimentConfigError::InvalidLambdaSelector { .. }
        ));
    }
}
