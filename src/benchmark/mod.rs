//! Typed loading of synthetic benchmark assets exported outside the Rust crate.
//!
//! Version 0.3 uses Python-side tooling to generate deterministic truth assets.
//! Reviewed benchmark suites can then be committed under `data/regression/`.
//! This module gives Rust a strongly typed, validated
//! way to load those assets so the recovery and comparison pipeline can build on
//! reviewed benchmark suites rather than ad hoc fixture parsing.

mod comparison;
mod noisy;
mod recovery;
mod suite;

pub use comparison::{
    BenchmarkComparisonError, BenchmarkIqComparison, BenchmarkIqResidualPoint,
    BenchmarkPrComparison, BenchmarkPrResidualPoint, BenchmarkRecoveryComparison,
    BenchmarkSuiteComparison, compare_benchmark_recovery, compare_benchmark_suite,
};
pub use noisy::{
    NoisyBenchmarkCase, NoisyBenchmarkCaseMetadata, NoisyBenchmarkIqCurve, NoisyBenchmarkIqPoint,
    NoisyBenchmarkSuite, NoisyBenchmarkSuiteRecoveryResult, NoisyBenchmarkSuiteSummary,
    compare_noisy_benchmark_suite, load_noisy_benchmark_suite, recover_noisy_benchmark_case,
    recover_noisy_benchmark_suite,
};
pub use recovery::{
    BenchmarkRecoveryConfig, BenchmarkRecoveryError, BenchmarkRecoveryResult,
    BenchmarkSuiteRecoveryResult, recover_benchmark_suite, recover_benchmark_truth_case,
};
pub use suite::{
    BenchmarkCaseMetadata, BenchmarkIqCurve, BenchmarkIqPoint, BenchmarkPrCurve, BenchmarkPrPoint,
    BenchmarkSuite, BenchmarkSuiteConfig, BenchmarkSuiteSummary, BenchmarkTruthCase,
    LoadBenchmarkError, load_benchmark_suite, load_benchmark_truth_case,
};
