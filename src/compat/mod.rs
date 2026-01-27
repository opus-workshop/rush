//! Bash script compatibility analyzer
//!
//! This module provides tools to analyze bash scripts and identify syntax features,
//! categorizing them as POSIX-compliant, bash-specific, or zsh-specific.

pub mod analyzer;
pub mod features;

pub use analyzer::{ScriptAnalyzer, AnalysisResult, FeatureOccurrence};
pub use features::{BashFeature, FeatureCategory};
