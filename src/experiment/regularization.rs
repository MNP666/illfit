use std::error::Error;
use std::fmt;

use crate::analysis::{AnalysisError, summarize_fit};
use crate::basis::{BasisError, CubicBSplineBasis};
use crate::benchmark::{
    BenchmarkComparisonError, BenchmarkRecoveryComparison, BenchmarkRecoveryResult,
    BenchmarkTruthCase, LoadBenchmarkError, NoisyBenchmarkCase, compare_benchmark_recovery,
    load_benchmark_suite, load_noisy_benchmark_suite,
};
use crate::data::{ParseCurveError, SaxsCurve, SaxsPoint};
use crate::experiment::{ExperimentConfig, ExperimentSuiteKind, LambdaSelectorMethod};
use crate::solver::{FitResult, LeastSquaresObservation, SolverError, solve_design_matrix};
use crate::transform::{ForwardTransform, TransformError};
use crate::weighting::{WeightingError, WeightingStrategy};

#[derive(Debug, Clone)]
pub struct ExperimentCaseResult {
    pub weighting_strategy: WeightingStrategy,
    pub lambda: f64,
    pub case_id: String,
    pub family: String,
    pub comparison: BenchmarkRecoveryComparison,
    pub data_misfit: f64,
    pub regularization_penalty: f64,
    pub effective_degrees_of_freedom: f64,
    pub gcv_score: f64,
    pub negative_value_fraction: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentStrategySummary {
    pub weighting_strategy: WeightingStrategy,
    pub lambda: f64,
    pub case_count: usize,
    pub mean_pr_rmse: f64,
    pub mean_pr_correlation: f64,
    pub mean_q_rmse: f64,
    pub mean_data_misfit: f64,
    pub mean_regularization_penalty: f64,
    pub mean_effective_degrees_of_freedom: f64,
    pub mean_gcv_score: f64,
    pub l_curve_curvature: Option<f64>,
    pub mean_negative_value_fraction: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExperimentSelectorResult {
    pub weighting_strategy: WeightingStrategy,
    pub method: LambdaSelectorMethod,
    pub selected_lambda: f64,
    pub selected_mean_data_misfit: f64,
    pub selected_mean_regularization_penalty: f64,
    pub selected_mean_gcv_score: f64,
    pub l_curve_curvature: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ExperimentRunResult {
    pub config: ExperimentConfig,
    pub case_results: Vec<ExperimentCaseResult>,
    pub summaries: Vec<ExperimentStrategySummary>,
    pub selector_results: Vec<ExperimentSelectorResult>,
}

#[derive(Debug, Clone)]
pub struct SelectedExperimentRecovery {
    pub weighting_strategy: WeightingStrategy,
    pub method: LambdaSelectorMethod,
    pub selected_lambda: f64,
    pub case_results: Vec<ExperimentSelectedCaseRecovery>,
}

#[derive(Debug, Clone)]
pub struct ExperimentSelectedCaseRecovery {
    pub case_id: String,
    pub family: String,
    pub negative_value_fraction: Option<f64>,
    pub recovery: BenchmarkRecoveryResult,
    pub comparison: BenchmarkRecoveryComparison,
}

#[derive(Debug)]
pub enum ExperimentRegularizationError {
    Config(crate::experiment::ExperimentConfigError),
    LoadBenchmark(LoadBenchmarkError),
    Basis(BasisError),
    Curve(ParseCurveError),
    Transform(TransformError),
    Solver(SolverError),
    Analysis(AnalysisError),
    Comparison(BenchmarkComparisonError),
    Weighting(WeightingError),
}

impl fmt::Display for ExperimentRegularizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(f, "{error}"),
            Self::LoadBenchmark(error) => write!(f, "{error}"),
            Self::Basis(error) => write!(f, "{error}"),
            Self::Curve(error) => write!(f, "{error}"),
            Self::Transform(error) => write!(f, "{error}"),
            Self::Solver(error) => write!(f, "{error}"),
            Self::Analysis(error) => write!(f, "{error}"),
            Self::Comparison(error) => write!(f, "{error}"),
            Self::Weighting(error) => write!(f, "{error}"),
        }
    }
}

impl Error for ExperimentRegularizationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::LoadBenchmark(error) => Some(error),
            Self::Basis(error) => Some(error),
            Self::Curve(error) => Some(error),
            Self::Transform(error) => Some(error),
            Self::Solver(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::Comparison(error) => Some(error),
            Self::Weighting(error) => Some(error),
        }
    }
}

impl From<crate::experiment::ExperimentConfigError> for ExperimentRegularizationError {
    fn from(value: crate::experiment::ExperimentConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<LoadBenchmarkError> for ExperimentRegularizationError {
    fn from(value: LoadBenchmarkError) -> Self {
        Self::LoadBenchmark(value)
    }
}

impl From<BasisError> for ExperimentRegularizationError {
    fn from(value: BasisError) -> Self {
        Self::Basis(value)
    }
}

impl From<ParseCurveError> for ExperimentRegularizationError {
    fn from(value: ParseCurveError) -> Self {
        Self::Curve(value)
    }
}

impl From<TransformError> for ExperimentRegularizationError {
    fn from(value: TransformError) -> Self {
        Self::Transform(value)
    }
}

impl From<SolverError> for ExperimentRegularizationError {
    fn from(value: SolverError) -> Self {
        Self::Solver(value)
    }
}

impl From<AnalysisError> for ExperimentRegularizationError {
    fn from(value: AnalysisError) -> Self {
        Self::Analysis(value)
    }
}

impl From<BenchmarkComparisonError> for ExperimentRegularizationError {
    fn from(value: BenchmarkComparisonError) -> Self {
        Self::Comparison(value)
    }
}

impl From<WeightingError> for ExperimentRegularizationError {
    fn from(value: WeightingError) -> Self {
        Self::Weighting(value)
    }
}

pub fn run_regularization_experiment(
    config: &ExperimentConfig,
) -> Result<ExperimentRunResult, ExperimentRegularizationError> {
    let mut case_results = Vec::new();

    match config.suite.kind {
        ExperimentSuiteKind::Benchmark => {
            let suite = load_benchmark_suite(&config.suite.path)?;
            for &strategy in &config.weighting_strategies {
                for &lambda in &config.lambda_grid.values {
                    for truth_case in &suite.truth_cases {
                        let observed_curve =
                            synthetic_observed_curve(truth_case, config.recovery.synthetic_sigma)?;
                        let recovery = recover_with_weighting(
                            truth_case,
                            observed_curve,
                            strategy,
                            lambda,
                            config,
                        )?;
                        case_results.push(case_result_from_recovery(
                            strategy,
                            lambda,
                            truth_case.metadata.candidate_id.clone(),
                            truth_case.metadata.family.clone(),
                            &recovery,
                            None,
                        )?);
                    }
                }
            }
        }
        ExperimentSuiteKind::NoisyBenchmark => {
            let suite = load_noisy_benchmark_suite(&config.suite.path)?;
            for &strategy in &config.weighting_strategies {
                for &lambda in &config.lambda_grid.values {
                    for noisy_case in &suite.cases {
                        let recovery =
                            recover_noisy_with_weighting(noisy_case, strategy, lambda, config)?;
                        case_results.push(case_result_from_recovery(
                            strategy,
                            lambda,
                            noisy_case.truth_case.metadata.candidate_id.clone(),
                            noisy_case.truth_case.metadata.family.clone(),
                            &recovery,
                            Some(noisy_case.noise_metadata.negative_value_fraction),
                        )?);
                    }
                }
            }
        }
    }

    let summaries = summarize_case_results(&case_results);
    let selector_results = select_lambdas(&config.selectors, &summaries);

    Ok(ExperimentRunResult {
        config: config.clone(),
        case_results,
        summaries,
        selector_results,
    })
}

pub fn recover_selected_experiment_cases(
    config: &ExperimentConfig,
    selector_results: &[ExperimentSelectorResult],
) -> Result<Vec<SelectedExperimentRecovery>, ExperimentRegularizationError> {
    let mut selected = Vec::new();

    match config.suite.kind {
        ExperimentSuiteKind::Benchmark => {
            let suite = load_benchmark_suite(&config.suite.path)?;
            for selector in selector_results {
                let mut case_results = Vec::with_capacity(suite.truth_cases.len());
                for truth_case in &suite.truth_cases {
                    let observed_curve =
                        synthetic_observed_curve(truth_case, config.recovery.synthetic_sigma)?;
                    let recovery = recover_with_weighting(
                        truth_case,
                        observed_curve,
                        selector.weighting_strategy,
                        selector.selected_lambda,
                        config,
                    )?;
                    let comparison = compare_benchmark_recovery(&recovery)?;
                    case_results.push(ExperimentSelectedCaseRecovery {
                        case_id: truth_case.metadata.candidate_id.clone(),
                        family: truth_case.metadata.family.clone(),
                        negative_value_fraction: None,
                        recovery,
                        comparison,
                    });
                }
                selected.push(SelectedExperimentRecovery {
                    weighting_strategy: selector.weighting_strategy,
                    method: selector.method,
                    selected_lambda: selector.selected_lambda,
                    case_results,
                });
            }
        }
        ExperimentSuiteKind::NoisyBenchmark => {
            let suite = load_noisy_benchmark_suite(&config.suite.path)?;
            for selector in selector_results {
                let mut case_results = Vec::with_capacity(suite.cases.len());
                for noisy_case in &suite.cases {
                    let recovery = recover_noisy_with_weighting(
                        noisy_case,
                        selector.weighting_strategy,
                        selector.selected_lambda,
                        config,
                    )?;
                    let comparison = compare_benchmark_recovery(&recovery)?;
                    case_results.push(ExperimentSelectedCaseRecovery {
                        case_id: noisy_case.truth_case.metadata.candidate_id.clone(),
                        family: noisy_case.truth_case.metadata.family.clone(),
                        negative_value_fraction: Some(
                            noisy_case.noise_metadata.negative_value_fraction,
                        ),
                        recovery,
                        comparison,
                    });
                }
                selected.push(SelectedExperimentRecovery {
                    weighting_strategy: selector.weighting_strategy,
                    method: selector.method,
                    selected_lambda: selector.selected_lambda,
                    case_results,
                });
            }
        }
    }

    Ok(selected)
}

fn case_result_from_recovery(
    weighting_strategy: WeightingStrategy,
    lambda: f64,
    case_id: String,
    family: String,
    recovery: &BenchmarkRecoveryResult,
    negative_value_fraction: Option<f64>,
) -> Result<ExperimentCaseResult, BenchmarkComparisonError> {
    let comparison = compare_benchmark_recovery(recovery)?;
    let point_count = recovery.observed_curve.points().len();

    Ok(ExperimentCaseResult {
        weighting_strategy,
        lambda,
        case_id,
        family,
        comparison,
        data_misfit: recovery.fit.weighted_residual_sum_squares,
        regularization_penalty: recovery.fit.regularization_penalty,
        effective_degrees_of_freedom: recovery.fit.effective_degrees_of_freedom,
        gcv_score: generalized_cross_validation_score(
            recovery.fit.weighted_residual_sum_squares,
            point_count,
            recovery.fit.effective_degrees_of_freedom,
        ),
        negative_value_fraction,
    })
}

fn recover_noisy_with_weighting(
    noisy_case: &NoisyBenchmarkCase,
    strategy: WeightingStrategy,
    lambda: f64,
    config: &ExperimentConfig,
) -> Result<BenchmarkRecoveryResult, ExperimentRegularizationError> {
    let observed_curve = SaxsCurve::new(
        noisy_case
            .observed_iq
            .points()
            .iter()
            .map(|point| SaxsPoint {
                q: point.q,
                intensity: point.observed_intensity,
                sigma: point.sigma,
            })
            .collect(),
    )?;

    recover_with_weighting(
        &noisy_case.truth_case,
        observed_curve,
        strategy,
        lambda,
        config,
    )
}

fn recover_with_weighting(
    truth_case: &BenchmarkTruthCase,
    observed_curve: SaxsCurve,
    strategy: WeightingStrategy,
    lambda: f64,
    config: &ExperimentConfig,
) -> Result<BenchmarkRecoveryResult, ExperimentRegularizationError> {
    let basis = CubicBSplineBasis::new(config.recovery.dmax, config.recovery.basis_size)?;
    let transform = ForwardTransform::new(basis, config.recovery.integration_intervals)?;
    let raw_design_matrix = transform.design_matrix_for_curve(&observed_curve)?;
    let transformed_observations = observed_curve
        .points()
        .iter()
        .map(|point| strategy.transform_observation(point.q, point.intensity, point.sigma))
        .collect::<Result<Vec<_>, _>>()?;
    let weighted_design_matrix = raw_design_matrix
        .iter()
        .zip(transformed_observations.iter())
        .map(|(row, observation)| {
            row.iter()
                .map(|value| observation.scale * value)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let weighted_observations = transformed_observations
        .iter()
        .map(|observation| LeastSquaresObservation {
            intensity: observation.intensity,
            sigma: observation.sigma,
        })
        .collect::<Vec<_>>();
    let solution = solve_design_matrix(&weighted_design_matrix, &weighted_observations, lambda)?;
    let coefficients = solution.coefficients;
    let predicted_intensities = transform.predict_for_curve(&observed_curve, &coefficients)?;
    let residuals = observed_curve
        .points()
        .iter()
        .zip(predicted_intensities.iter())
        .map(|(point, predicted)| predicted - point.intensity)
        .collect::<Vec<_>>();
    let fit = FitResult {
        coefficients,
        predicted_intensities,
        residuals,
        weighted_residual_sum_squares: solution.weighted_residual_sum_squares,
        regularization_penalty: solution.regularization_penalty,
        objective_value: solution.objective_value,
        effective_degrees_of_freedom: solution.effective_degrees_of_freedom,
        lambda: solution.lambda,
    };
    let summary = summarize_fit(
        &observed_curve,
        &transform,
        &fit,
        config.recovery.pr_sample_points,
    )?;

    Ok(BenchmarkRecoveryResult {
        truth_case: truth_case.clone(),
        observed_curve,
        transform,
        fit,
        summary,
    })
}

fn synthetic_observed_curve(
    truth_case: &BenchmarkTruthCase,
    synthetic_sigma: f64,
) -> Result<SaxsCurve, ParseCurveError> {
    SaxsCurve::new(
        truth_case
            .iq_truth
            .points()
            .iter()
            .map(|point| SaxsPoint {
                q: point.q,
                intensity: point.intensity,
                sigma: synthetic_sigma,
            })
            .collect(),
    )
}

fn summarize_case_results(case_results: &[ExperimentCaseResult]) -> Vec<ExperimentStrategySummary> {
    let mut summaries = Vec::new();

    for strategy in case_results
        .iter()
        .map(|result| result.weighting_strategy)
        .collect::<Vec<_>>()
    {
        for lambda in case_results
            .iter()
            .filter(|result| result.weighting_strategy == strategy)
            .map(|result| result.lambda)
            .collect::<Vec<_>>()
        {
            if summaries.iter().any(|summary: &ExperimentStrategySummary| {
                summary.weighting_strategy == strategy && summary.lambda == lambda
            }) {
                continue;
            }

            let matching = case_results
                .iter()
                .filter(|result| result.weighting_strategy == strategy && result.lambda == lambda)
                .collect::<Vec<_>>();

            let case_count = matching.len();
            let mean_pr_rmse = matching
                .iter()
                .map(|result| result.comparison.pr.rmse)
                .sum::<f64>()
                / case_count as f64;
            let mean_pr_correlation = matching
                .iter()
                .map(|result| result.comparison.pr.correlation)
                .sum::<f64>()
                / case_count as f64;
            let mean_q_rmse = matching
                .iter()
                .map(|result| result.comparison.iq.rmse)
                .sum::<f64>()
                / case_count as f64;
            let mean_data_misfit = matching
                .iter()
                .map(|result| result.data_misfit)
                .sum::<f64>()
                / case_count as f64;
            let mean_regularization_penalty = matching
                .iter()
                .map(|result| result.regularization_penalty)
                .sum::<f64>()
                / case_count as f64;
            let mean_effective_degrees_of_freedom = matching
                .iter()
                .map(|result| result.effective_degrees_of_freedom)
                .sum::<f64>()
                / case_count as f64;
            let mean_gcv_score =
                matching.iter().map(|result| result.gcv_score).sum::<f64>() / case_count as f64;

            let negative_values = matching
                .iter()
                .filter_map(|result| result.negative_value_fraction)
                .collect::<Vec<_>>();
            let mean_negative_value_fraction = if negative_values.is_empty() {
                None
            } else {
                Some(negative_values.iter().sum::<f64>() / negative_values.len() as f64)
            };

            summaries.push(ExperimentStrategySummary {
                weighting_strategy: strategy,
                lambda,
                case_count,
                mean_pr_rmse,
                mean_pr_correlation,
                mean_q_rmse,
                mean_data_misfit,
                mean_regularization_penalty,
                mean_effective_degrees_of_freedom,
                mean_gcv_score,
                l_curve_curvature: None,
                mean_negative_value_fraction,
            });
        }
    }

    summaries.sort_by(|left, right| {
        left.as_sort_key()
            .cmp(&right.as_sort_key())
            .then_with(|| left.lambda.total_cmp(&right.lambda))
    });
    annotate_l_curve_curvature(&mut summaries);
    summaries
}

fn generalized_cross_validation_score(
    weighted_residual_sum_squares: f64,
    observation_count: usize,
    effective_degrees_of_freedom: f64,
) -> f64 {
    let denominator = observation_count as f64 - effective_degrees_of_freedom;
    if denominator <= 0.0 || !denominator.is_finite() {
        return f64::INFINITY;
    }

    weighted_residual_sum_squares / (denominator * denominator)
}

fn annotate_l_curve_curvature(summaries: &mut [ExperimentStrategySummary]) {
    let strategies = summaries
        .iter()
        .map(|summary| summary.weighting_strategy)
        .collect::<Vec<_>>();

    for strategy in strategies {
        let indices = summaries
            .iter()
            .enumerate()
            .filter_map(|(index, summary)| {
                (summary.weighting_strategy == strategy).then_some(index)
            })
            .collect::<Vec<_>>();
        if indices.len() < 3 {
            continue;
        }

        for window in indices.windows(3) {
            let curvature = l_curve_curvature(
                &summaries[window[0]],
                &summaries[window[1]],
                &summaries[window[2]],
            );
            summaries[window[1]].l_curve_curvature = Some(curvature);
        }
    }
}

fn l_curve_curvature(
    left: &ExperimentStrategySummary,
    center: &ExperimentStrategySummary,
    right: &ExperimentStrategySummary,
) -> f64 {
    if left.mean_data_misfit <= 0.0
        || center.mean_data_misfit <= 0.0
        || right.mean_data_misfit <= 0.0
        || left.mean_regularization_penalty <= 0.0
        || center.mean_regularization_penalty <= 0.0
        || right.mean_regularization_penalty <= 0.0
    {
        return 0.0;
    }

    let x1 = left.mean_data_misfit.ln();
    let y1 = left.mean_regularization_penalty.ln();
    let x2 = center.mean_data_misfit.ln();
    let y2 = center.mean_regularization_penalty.ln();
    let x3 = right.mean_data_misfit.ln();
    let y3 = right.mean_regularization_penalty.ln();
    let t1 = left.lambda.ln();
    let t2 = center.lambda.ln();
    let t3 = right.lambda.ln();
    if !(t1 < t2 && t2 < t3) {
        return 0.0;
    }

    let dx_dt = (x3 - x1) / (t3 - t1);
    let dy_dt = (y3 - y1) / (t3 - t1);
    let d2x_dt2 = 2.0 * (((x3 - x2) / (t3 - t2)) - ((x2 - x1) / (t2 - t1))) / (t3 - t1);
    let d2y_dt2 = 2.0 * (((y3 - y2) / (t3 - t2)) - ((y2 - y1) / (t2 - t1))) / (t3 - t1);
    let numerator = (dx_dt * d2y_dt2 - dy_dt * d2x_dt2).abs();
    let denominator = (dx_dt * dx_dt + dy_dt * dy_dt).powf(1.5);

    if denominator == 0.0 || !denominator.is_finite() {
        0.0
    } else {
        numerator / denominator
    }
}

fn select_lambdas(
    methods: &[LambdaSelectorMethod],
    summaries: &[ExperimentStrategySummary],
) -> Vec<ExperimentSelectorResult> {
    let mut results = Vec::new();

    for strategy in summaries
        .iter()
        .map(|summary| summary.weighting_strategy)
        .collect::<Vec<_>>()
    {
        let strategy_points = summaries
            .iter()
            .filter(|summary| summary.weighting_strategy == strategy)
            .collect::<Vec<_>>();
        if strategy_points.is_empty() {
            continue;
        }

        for &method in methods {
            if results.iter().any(|result: &ExperimentSelectorResult| {
                result.weighting_strategy == strategy && result.method == method
            }) {
                continue;
            }

            let selected = match method {
                LambdaSelectorMethod::LCurve => strategy_points
                    .iter()
                    .max_by(|left, right| {
                        left.l_curve_curvature
                            .unwrap_or(0.0)
                            .total_cmp(&right.l_curve_curvature.unwrap_or(0.0))
                    })
                    .copied()
                    .unwrap_or(strategy_points[0]),
                LambdaSelectorMethod::Gcv => strategy_points
                    .iter()
                    .min_by(|left, right| left.mean_gcv_score.total_cmp(&right.mean_gcv_score))
                    .copied()
                    .unwrap_or(strategy_points[0]),
            };

            results.push(ExperimentSelectorResult {
                weighting_strategy: strategy,
                method,
                selected_lambda: selected.lambda,
                selected_mean_data_misfit: selected.mean_data_misfit,
                selected_mean_regularization_penalty: selected.mean_regularization_penalty,
                selected_mean_gcv_score: selected.mean_gcv_score,
                l_curve_curvature: selected.l_curve_curvature,
            });
        }
    }

    results.sort_by(|left, right| {
        left.weighting_strategy
            .as_config_string()
            .cmp(&right.weighting_strategy.as_config_string())
            .then_with(|| method_sort_key(left.method).cmp(method_sort_key(right.method)))
    });
    results
}

fn method_sort_key(method: LambdaSelectorMethod) -> &'static str {
    match method {
        LambdaSelectorMethod::LCurve => "l_curve",
        LambdaSelectorMethod::Gcv => "gcv",
    }
}

impl ExperimentStrategySummary {
    fn as_sort_key(&self) -> String {
        self.weighting_strategy.as_config_string()
    }
}

#[cfg(test)]
mod tests {
    use super::run_regularization_experiment;
    use crate::experiment::parse_experiment_config_str;
    use crate::weighting::WeightingStrategy;
    use std::path::PathBuf;

    fn benchmark_config() -> String {
        format!(
            r#"
[suite]
kind = "benchmark"
path = "{}"

[recovery]
dmax = 120.0
basis_size = 7
integration_intervals = 800
pr_sample_points = 120
synthetic_sigma = 0.05

[weighting]
strategies = ["none", "q"]

[lambda]
values = [1e-4, 1e-3, 1e-2]

[selectors]
methods = ["l_curve", "gcv"]

[output]
run_name = "test_run"
root_dir = "profiling/output"
"#,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data")
                .join("regression")
                .join("clamped_spline")
                .display()
        )
    }

    fn noisy_config() -> String {
        format!(
            r#"
[suite]
kind = "noisy_benchmark"
path = "{}"

[recovery]
dmax = 120.0
basis_size = 7
integration_intervals = 800
pr_sample_points = 120
synthetic_sigma = 0.05

[weighting]
strategies = ["none"]

[lambda]
values = [1e-4, 1e-3, 1e-2]

[selectors]
methods = ["l_curve", "gcv"]

[output]
run_name = "test_run"
root_dir = "profiling/output"
"#,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data")
                .join("synthetic")
                .join("noisy_clamped_spline_signal_scaled")
                .display()
        )
    }

    #[test]
    fn runs_regularization_experiment_for_benchmark_suite() {
        let config = parse_experiment_config_str(&benchmark_config()).unwrap();
        let result = run_regularization_experiment(&config).unwrap();

        assert!(!result.case_results.is_empty());
        assert_eq!(result.summaries.len(), 6);
        assert_eq!(
            result.summaries[0].weighting_strategy,
            WeightingStrategy::None
        );
        assert_eq!(result.selector_results.len(), 4);
        assert!(
            result
                .selector_results
                .iter()
                .any(|result| result.method == crate::experiment::LambdaSelectorMethod::LCurve)
        );
    }

    #[test]
    fn runs_regularization_experiment_for_noisy_suite() {
        let config = parse_experiment_config_str(&noisy_config()).unwrap();
        let result = run_regularization_experiment(&config).unwrap();

        assert!(!result.case_results.is_empty());
        assert!(
            result
                .case_results
                .iter()
                .all(|result| result.negative_value_fraction.is_some())
        );
        assert!(
            result
                .case_results
                .iter()
                .all(|result| result.gcv_score.is_finite())
        );
    }

    #[test]
    fn weighting_strategies_change_summary_metrics() {
        let config = parse_experiment_config_str(&benchmark_config()).unwrap();
        let result = run_regularization_experiment(&config).unwrap();
        let none_summary = result
            .summaries
            .iter()
            .find(|summary| {
                summary.weighting_strategy == WeightingStrategy::None && summary.lambda == 1e-3
            })
            .unwrap();
        let q_summary = result
            .summaries
            .iter()
            .find(|summary| {
                summary.weighting_strategy == WeightingStrategy::Q && summary.lambda == 1e-3
            })
            .unwrap();

        assert_ne!(none_summary.mean_data_misfit, q_summary.mean_data_misfit);
    }
}
