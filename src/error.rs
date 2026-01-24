//! Structured error types for Rush shell
//!
//! This module provides typed error representations that can be formatted
//! as either human-readable text or structured JSON.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Structured error type for Rush shell operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RushError {
    /// Error code category
    pub error_code: String,
    /// Human-readable error message
    pub message: String,
    /// Exit code for the shell
    pub exit_code: i32,
    /// Additional context information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

impl RushError {
    /// Create a new error with the given code, message, and exit code
    pub fn new(error_code: impl Into<String>, message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            error_code: error_code.into(),
            message: message.into(),
            exit_code,
            context: None,
        }
    }

    /// Add context information to the error
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    /// File not found error
    pub fn file_not_found(path: &Path) -> Self {
        Self::new(
            "FILE_NOT_FOUND",
            format!("{}: No such file or directory", path.display()),
            1,
        )
    }

    /// Is a directory error (when a file was expected)
    pub fn is_a_directory(path: &Path) -> Self {
        Self::new(
            "IS_A_DIRECTORY",
            format!("{}: Is a directory", path.display()),
            1,
        )
    }

    /// Format error as JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                r#"{{"error_code":"{}","message":"{}","exit_code":{}}}"#,
                self.error_code, self.message, self.exit_code
            )
        })
    }

    /// Format error as human-readable text
    pub fn to_text(&self) -> String {
        self.message.clone()
    }
}

/// Check if errors should be output in JSON format
///
/// Currently checks the RUSH_ERROR_FORMAT environment variable.
/// Returns true if it's set to "json", false otherwise.
pub fn should_output_json_errors() -> bool {
    std::env::var("RUSH_ERROR_FORMAT")
        .map(|v| v.to_lowercase() == "json")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_not_found() {
        let err = RushError::file_not_found(Path::new("/tmp/nonexistent"));
        assert_eq!(err.error_code, "FILE_NOT_FOUND");
        assert_eq!(err.exit_code, 1);
        assert!(err.message.contains("No such file or directory"));
    }

    #[test]
    fn test_is_a_directory() {
        let err = RushError::is_a_directory(Path::new("/tmp"));
        assert_eq!(err.error_code, "IS_A_DIRECTORY");
        assert_eq!(err.exit_code, 1);
        assert!(err.message.contains("Is a directory"));
    }

    #[test]
    fn test_to_json() {
        let err = RushError::new("TEST_ERROR", "Test message", 42);
        let json = err.to_json();
        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_to_text() {
        let err = RushError::new("TEST_ERROR", "Test message", 42);
        assert_eq!(err.to_text(), "Test message");
    }

    #[test]
    fn test_with_context() {
        let err = RushError::new("TEST_ERROR", "Test message", 1)
            .with_context(serde_json::json!({"file": "/tmp/test.txt"}));
        assert!(err.context.is_some());
    }

    #[test]
    fn test_should_output_json_errors() {
        // Default should be false
        std::env::remove_var("RUSH_ERROR_FORMAT");
        assert!(!should_output_json_errors());

        // Set to json
        std::env::set_var("RUSH_ERROR_FORMAT", "json");
        assert!(should_output_json_errors());

        // Set to JSON (uppercase)
        std::env::set_var("RUSH_ERROR_FORMAT", "JSON");
        assert!(should_output_json_errors());

        // Set to something else
        std::env::set_var("RUSH_ERROR_FORMAT", "text");
        assert!(!should_output_json_errors());

        // Clean up
        std::env::remove_var("RUSH_ERROR_FORMAT");
    }
}
