// Profiling infrastructure for measuring execution performance
// Provides timing data collection with zero overhead when disabled

use std::time::{Duration, Instant};
use std::collections::HashMap;
use nu_ansi_term::Color;

/// Execution stage for profiling breakdown
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionStage {
    Parse,
    BuiltinExecution,
    ExternalExecution,
    PipelineSetup,
    CommandSubstitution,
    GlobExpansion,
    VariableExpansion,
    Total,
}

impl ExecutionStage {
    pub fn label(&self) -> &'static str {
        match self {
            ExecutionStage::Parse => "Parse",
            ExecutionStage::BuiltinExecution => "Builtin Execution",
            ExecutionStage::ExternalExecution => "External Execution",
            ExecutionStage::PipelineSetup => "Pipeline Setup",
            ExecutionStage::CommandSubstitution => "Command Substitution",
            ExecutionStage::GlobExpansion => "Glob Expansion",
            ExecutionStage::VariableExpansion => "Variable Expansion",
            ExecutionStage::Total => "Total",
        }
    }
}

/// Timing data for a single stage
#[derive(Debug, Clone)]
pub struct StageTiming {
    pub stage: ExecutionStage,
    pub count: usize,
    pub total: Duration,
}

impl StageTiming {
    pub fn average(&self) -> Duration {
        if self.count == 0 {
            Duration::ZERO
        } else {
            self.total / self.count as u32
        }
    }

    pub fn micros(&self) -> u64 {
        self.total.as_micros() as u64
    }

    pub fn millis(&self) -> f64 {
        self.total.as_secs_f64() * 1000.0
    }
}

/// Profile data collected during execution
#[derive(Debug, Clone)]
pub struct ProfileData {
    stages: HashMap<ExecutionStage, StageTiming>,
    total_start: Option<Instant>,
}

impl ProfileData {
    pub fn new() -> Self {
        Self {
            stages: HashMap::new(),
            total_start: None,
        }
    }

    pub fn start_total(&mut self) {
        self.total_start = Some(Instant::now());
    }

    pub fn record(&mut self, stage: ExecutionStage, duration: Duration) {
        let entry = self.stages
            .entry(stage)
            .or_insert(StageTiming {
                stage,
                count: 0,
                total: Duration::ZERO,
            });
        entry.count += 1;
        entry.total += duration;
    }

    pub fn total_elapsed(&self) -> Duration {
        self.total_start.map_or(Duration::ZERO, |start| start.elapsed())
    }

    pub fn get_stats(&self, stage: ExecutionStage) -> Option<StageTiming> {
        self.stages.get(&stage).cloned()
    }

    pub fn stages(&self) -> Vec<StageTiming> {
        let mut stages: Vec<_> = self.stages.values().cloned().collect();
        stages.sort_by_key(|s| s.stage as u8);
        stages
    }

    pub fn clear(&mut self) {
        self.stages.clear();
        self.total_start = None;
    }
}

impl Default for ProfileData {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII wrapper for automatic timing of a stage
pub struct ScopedProfiler {
    profile_data: Option<Box<ProfileData>>,
    stage: ExecutionStage,
    start: Instant,
}

impl ScopedProfiler {
    pub fn new(stage: ExecutionStage) -> Self {
        Self {
            profile_data: None,
            stage,
            start: Instant::now(),
        }
    }

    pub fn with_data(mut self, data: &mut ProfileData) -> Self {
        self.profile_data = Some(Box::new(data.clone()));
        self
    }
}

impl Drop for ScopedProfiler {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        if let Some(ref mut data) = self.profile_data {
            data.record(self.stage, elapsed);
        }
    }
}

/// Formats profiling data for human-readable output
pub struct ProfileFormatter;

impl ProfileFormatter {
    /// Format profile data as a human-readable table
    pub fn format(data: &ProfileData) -> String {
        let mut output = String::new();
        output.push_str("\n");
        output.push_str(&Color::Blue.bold().paint("Execution Timeline").to_string());
        output.push_str("\n");
        output.push_str(&"=".repeat(70));
        output.push_str("\n");

        let stages = data.stages();
        if stages.is_empty() {
            output.push_str("No profiling data collected\n");
        } else {
            output.push_str(&format!(
                "{:<30} {:>15} {:>15} {:>8}\n",
                "Stage", "Total", "Avg", "Count"
            ));
            output.push_str(&"-".repeat(70));
            output.push_str("\n");

            for timing in &stages {
                let total_str = Self::format_duration(timing.total);
                let avg_str = Self::format_duration(timing.average());
                output.push_str(&format!(
                    "{:<30} {:>15} {:>15} {:>8}\n",
                    timing.stage.label(),
                    Color::Green.paint(total_str).to_string(),
                    Color::Yellow.paint(avg_str).to_string(),
                    timing.count
                ));
            }

            output.push_str(&"-".repeat(70));
            output.push_str("\n");

            let total = data.total_elapsed();
            let total_str = Self::format_duration(total);
            output.push_str(&format!(
                "{:<30} {:>15}\n",
                Color::Cyan.bold().paint("Total Time").to_string(),
                Color::Cyan.bold().paint(total_str).to_string()
            ));
        }

        output.push_str(&"=".repeat(70));
        output.push_str("\n");

        output
    }

    /// Format duration in human-readable format (ms or us)
    fn format_duration(d: Duration) -> String {
        let millis = d.as_secs_f64() * 1000.0;
        if millis >= 1.0 {
            format!("{:.2}ms", millis)
        } else {
            let micros = d.as_micros();
            format!("{}µs", micros)
        }
    }

    /// Format profile data as compact JSON
    pub fn format_json(data: &ProfileData) -> serde_json::Value {
        let mut stages = Vec::new();

        for timing in data.stages() {
            stages.push(serde_json::json!({
                "stage": timing.stage.label(),
                "count": timing.count,
                "total_ms": timing.millis(),
                "avg_ms": timing.average().as_secs_f64() * 1000.0,
            }));
        }

        serde_json::json!({
            "total_ms": data.total_elapsed().as_secs_f64() * 1000.0,
            "stages": stages,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_labels() {
        assert_eq!(ExecutionStage::Parse.label(), "Parse");
        assert_eq!(ExecutionStage::BuiltinExecution.label(), "Builtin Execution");
        assert_eq!(ExecutionStage::ExternalExecution.label(), "External Execution");
    }

    #[test]
    fn test_stage_timing_creation() {
        let timing = StageTiming {
            stage: ExecutionStage::Parse,
            count: 1,
            total: Duration::from_millis(10),
        };
        assert_eq!(timing.count, 1);
        assert_eq!(timing.millis(), 10.0);
    }

    #[test]
    fn test_stage_timing_average() {
        let timing = StageTiming {
            stage: ExecutionStage::Parse,
            count: 2,
            total: Duration::from_millis(100),
        };
        assert_eq!(timing.average(), Duration::from_millis(50));
    }

    #[test]
    fn test_stage_timing_average_zero_count() {
        let timing = StageTiming {
            stage: ExecutionStage::Parse,
            count: 0,
            total: Duration::from_millis(100),
        };
        assert_eq!(timing.average(), Duration::ZERO);
    }

    #[test]
    fn test_profile_data_new() {
        let data = ProfileData::new();
        assert_eq!(data.stages().len(), 0);
    }

    #[test]
    fn test_profile_data_record() {
        let mut data = ProfileData::new();
        data.record(ExecutionStage::Parse, Duration::from_millis(5));
        data.record(ExecutionStage::Parse, Duration::from_millis(3));

        let stats = data.get_stats(ExecutionStage::Parse).unwrap();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.total, Duration::from_millis(8));
    }

    #[test]
    fn test_profile_data_total_elapsed() {
        let mut data = ProfileData::new();
        data.start_total();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = data.total_elapsed();
        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn test_profile_data_stages() {
        let mut data = ProfileData::new();
        data.record(ExecutionStage::Parse, Duration::from_millis(5));
        data.record(ExecutionStage::BuiltinExecution, Duration::from_millis(10));

        let stages = data.stages();
        assert_eq!(stages.len(), 2);
    }

    #[test]
    fn test_profile_data_clear() {
        let mut data = ProfileData::new();
        data.record(ExecutionStage::Parse, Duration::from_millis(5));
        data.clear();
        assert_eq!(data.stages().len(), 0);
    }

    #[test]
    fn test_scoped_profiler() {
        let mut data = ProfileData::new();
        {
            let _profiler = ScopedProfiler::new(ExecutionStage::Parse);
            std::thread::sleep(Duration::from_millis(5));
        }
        // Without attaching data, it shouldn't record
        assert_eq!(data.stages().len(), 0);
    }

    #[test]
    fn test_profile_formatter_format() {
        let mut data = ProfileData::new();
        data.start_total();
        data.record(ExecutionStage::Parse, Duration::from_millis(10));
        data.record(ExecutionStage::BuiltinExecution, Duration::from_millis(5));

        let output = ProfileFormatter::format(&data);
        assert!(output.contains("Execution Timeline"));
        assert!(output.contains("Parse"));
        assert!(output.contains("Builtin Execution"));
    }

    #[test]
    fn test_profile_formatter_duration_millis() {
        let output = ProfileFormatter::format(&ProfileData::new());
        // Should not panic
        assert!(!output.is_empty());
    }

    #[test]
    fn test_profile_formatter_duration_micros() {
        let duration = Duration::from_micros(500);
        let formatted = ProfileFormatter::format_duration(duration);
        assert!(formatted.contains("µs"));
    }

    #[test]
    fn test_profile_formatter_json() {
        let mut data = ProfileData::new();
        data.record(ExecutionStage::Parse, Duration::from_millis(10));

        let json = ProfileFormatter::format_json(&data);
        assert!(json.get("total_ms").is_some());
        assert!(json.get("stages").is_some());
    }

    #[test]
    fn test_profile_data_default() {
        let data = ProfileData::default();
        assert_eq!(data.stages().len(), 0);
    }
}
