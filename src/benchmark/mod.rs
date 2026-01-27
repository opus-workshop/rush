pub mod runner;

pub use runner::{BenchmarkMode, BenchmarkRunner, BenchmarkResult};

/// Run benchmark based on mode
pub fn run_benchmark(mode: BenchmarkMode) -> anyhow::Result<()> {
    let mut runner = BenchmarkRunner::new();
    runner.run(mode)
}
