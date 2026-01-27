//! Bash script compatibility analyzer
//!
//! This module provides tools to analyze bash scripts and identify syntax features,
//! categorizing them by support status in Rush.

pub mod features;
pub mod database;

pub use features::{Feature, FeatureCategory, SupportStatus};
pub use database::{CompatDatabase, CompatSummary, MigrationStep};
