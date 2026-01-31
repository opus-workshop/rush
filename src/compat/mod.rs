//! Bash script compatibility analyzer
//!
//! This module provides tools to analyze bash scripts and identify syntax features,
//! categorizing them by support status in Rush.
//!
//! The compatibility database maps 50+ bash features to Rush support status:
//! - Supported: Fully implemented
//! - Planned: Will be implemented
//! - Not Supported: Has clear workarounds

pub mod features;
pub mod database;
pub mod analyzer;
pub mod report;
pub mod migrate;

pub use features::RushCompatFeature;
pub use database::CompatDatabase;
pub use analyzer::ScriptAnalyzer;
pub use report::CompatibilityReport;
pub use migrate::MigrationEngine;
