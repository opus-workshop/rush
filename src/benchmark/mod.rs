pub mod runner;
pub mod compare;

pub use runner::BenchmarkMode;
pub use runner::BenchmarkRunner;
pub use compare::ComparisonResult;

/// Run benchmark based on mode
pub fn run_benchmark(mode: BenchmarkMode) -> anyhow::Result<()> {
    let mut runner = BenchmarkRunner::new();
    runner.run(mode)
}
