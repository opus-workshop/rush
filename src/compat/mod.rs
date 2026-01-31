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

pub use features::{RushCompatFeature, RushSupportStatus, rush_compat_features};
pub use database::{CompatDatabase, CompatSummary, MigrationStep};
pub use analyzer::{ScriptAnalyzer, AnalysisResult, FeatureOccurrence};
pub use report::{CompatibilityReport, CompatibilityIssue, FeatureGroup};
