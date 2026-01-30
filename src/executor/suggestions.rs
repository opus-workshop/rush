//! Suggestion engine for command and flag typos
//!
//! This module provides utilities for detecting and suggesting corrections for:
//! - Command typos (e.g., 'lss' → 'ls')
//! - Flag typos (e.g., '--hlep' → '--help')
//! - Variable/path suggestions
//!
//! It integrates with the Corrector module which uses Levenshtein distance
//! and fuzzy matching to find similar commands and flags.

use crate::correction::{Corrector, Suggestion, SuggestionKind};
use std::path::Path;

/// Configuration for suggestion behavior
#[derive(Debug, Clone)]
pub struct SuggestionConfig {
    /// Whether to show suggestions at all
    pub enabled: bool,
    /// Maximum number of suggestions to display
    pub max_display: usize,
    /// Minimum score threshold for suggestions (0-100)
    pub min_score: i64,
}

impl Default for SuggestionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_display: 3,
            min_score: 50,
        }
    }
}

/// Suggestion engine for command and flag typos
#[derive(Clone)]
pub struct SuggestionEngine {
    corrector: Corrector,
    config: SuggestionConfig,
}

impl SuggestionEngine {
    /// Create a new suggestion engine with default configuration
    pub fn new() -> Self {
        Self {
            corrector: Corrector::new(),
            config: SuggestionConfig::default(),
        }
    }

    /// Create a suggestion engine with custom configuration
    pub fn with_config(config: SuggestionConfig) -> Self {
        Self {
            corrector: Corrector::new(),
            config,
        }
    }

    /// Get the underlying corrector
    pub fn corrector(&self) -> &Corrector {
        &self.corrector
    }

    /// Get the underlying corrector mutably
    pub fn corrector_mut(&mut self) -> &mut Corrector {
        &mut self.corrector
    }

    /// Suggest corrections for a mistyped command
    ///
    /// Uses Levenshtein distance and fuzzy matching to find similar commands
    /// from builtins, aliases, and PATH.
    ///
    /// # Arguments
    /// - `typo`: The mistyped command name
    /// - `builtins`: Available builtin command names
    /// - `aliases`: Defined alias names
    /// - `history`: Recent commands from history
    /// - `current_dir`: Current working directory for context
    ///
    /// # Returns
    /// A vector of suggestions, sorted by similarity score (best first)
    pub fn suggest_command(
        &self,
        typo: &str,
        builtins: &[String],
        aliases: &[String],
        history: &[String],
        current_dir: &Path,
    ) -> Vec<Suggestion> {
        if !self.config.enabled {
            return Vec::new();
        }

        // Get suggestions from the corrector with full context
        let mut suggestions = self.corrector.suggest_command_with_context(
            typo,
            builtins,
            aliases,
            history,
            current_dir,
        );

        // Filter by minimum score and limit to max_display
        suggestions.retain(|s| s.score >= self.config.min_score);
        suggestions.truncate(self.config.max_display);

        suggestions
    }

    /// Suggest corrections for a mistyped flag
    ///
    /// Compares the typo against valid flags, ignoring leading dashes.
    /// Uses Levenshtein distance to measure similarity.
    ///
    /// # Arguments
    /// - `typo`: The mistyped flag (e.g., '--hlep')
    /// - `valid_flags`: Valid flags for the command (e.g., ['--help', '--verbose'])
    ///
    /// # Returns
    /// A vector of flag suggestions, sorted by similarity score (best first)
    pub fn suggest_flag(&self, typo: &str, valid_flags: &[&str]) -> Vec<Suggestion> {
        if !self.config.enabled {
            return Vec::new();
        }

        let mut suggestions = self.corrector.suggest_flag(typo, valid_flags);

        // Filter by minimum score and limit to max_display
        suggestions.retain(|s| s.score >= self.config.min_score);
        suggestions.truncate(self.config.max_display);

        suggestions
    }

    /// Format suggestions as a user-friendly string
    ///
    /// Example output:
    /// ```
    /// Did you mean?
    ///   ls (95%, builtin)
    ///   lst (85%, command)
    /// ```
    pub fn format_suggestions(&self, suggestions: &[Suggestion]) -> String {
        if suggestions.is_empty() {
            return String::new();
        }

        let mut output = String::from("Did you mean?\n");

        for suggestion in suggestions.iter().take(self.config.max_display) {
            let similarity = Corrector::similarity_percent(suggestion.score, &suggestion.text);
            let kind_label = suggestion.kind.label();

            output.push_str(&format!(
                "  {} ({}%, {})\n",
                suggestion.text, similarity, kind_label
            ));
        }

        output
    }

    /// Check if a command typo is likely (similarity score above threshold)
    pub fn is_likely_typo(&self, typo: &str, builtins: &[String]) -> bool {
        if !self.config.enabled {
            return false;
        }

        let suggestions = self.corrector.suggest_command(typo, builtins);
        if let Some(best) = suggestions.first() {
            best.score >= self.config.min_score
        } else {
            false
        }
    }

    /// Check if a flag typo is likely (similarity score above threshold)
    pub fn is_likely_flag_typo(&self, typo: &str, valid_flags: &[&str]) -> bool {
        if !self.config.enabled {
            return false;
        }

        let suggestions = self.corrector.suggest_flag(typo, valid_flags);
        if let Some(best) = suggestions.first() {
            best.score >= self.config.min_score
        } else {
            false
        }
    }

    /// Set whether suggestions are enabled
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        self.corrector.set_enabled(enabled);
    }

    /// Check if suggestions are enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_command_typo_lss() {
        let engine = SuggestionEngine::new();
        let builtins = vec!["ls".to_string(), "cat".to_string(), "echo".to_string()];
        let aliases = vec![];
        let history = vec![];
        let cwd = std::path::Path::new(".");

        let suggestions = engine.suggest_command("lss", &builtins, &aliases, &history, cwd);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].text, "ls");
    }

    #[test]
    fn test_suggest_command_typo_eho() {
        let engine = SuggestionEngine::new();
        let builtins = vec!["echo".to_string(), "cat".to_string(), "grep".to_string()];
        let aliases = vec![];
        let history = vec![];
        let cwd = std::path::Path::new(".");

        let suggestions = engine.suggest_command("eho", &builtins, &aliases, &history, cwd);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].text, "echo");
    }

    #[test]
    fn test_suggest_flag_typo_hlep() {
        let engine = SuggestionEngine::new();
        let valid_flags = &["--help", "--version", "--verbose"];

        let suggestions = engine.suggest_flag("--hlep", valid_flags);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].text, "--help");
    }

    #[test]
    fn test_suggest_flag_typo_verbo() {
        let engine = SuggestionEngine::new();
        let valid_flags = &["--help", "--version", "--verbose", "-v"];

        let suggestions = engine.suggest_flag("--verbo", valid_flags);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.text == "--verbose"));
    }

    #[test]
    fn test_format_suggestions() {
        let engine = SuggestionEngine::new();
        let suggestions = vec![
            Suggestion {
                text: "ls".to_string(),
                score: 95,
                kind: SuggestionKind::Builtin,
            },
            Suggestion {
                text: "list".to_string(),
                score: 80,
                kind: SuggestionKind::Executable,
            },
        ];

        let formatted = engine.format_suggestions(&suggestions);
        assert!(formatted.contains("Did you mean?"));
        assert!(formatted.contains("ls"));
        assert!(formatted.contains("builtin"));
    }

    #[test]
    fn test_is_likely_typo() {
        let engine = SuggestionEngine::new();
        let builtins = vec!["echo".to_string(), "cat".to_string()];

        // "ehco" is close to "echo"
        assert!(engine.is_likely_typo("ehco", &builtins));

        // "xyz" is not close to anything
        let suggestions = engine.corrector().suggest_command("xyz", &builtins);
        println!("Suggestions for 'xyz': {:?}", suggestions);
        for suggestion in &suggestions {
            println!("  {} (score: {})", suggestion.text, suggestion.score);
        }
        assert!(!engine.is_likely_typo("xyz", &builtins));
    }

    #[test]
    fn test_is_likely_flag_typo() {
        let engine = SuggestionEngine::new();
        let valid_flags = &["--help", "--version"];

        assert!(engine.is_likely_flag_typo("--hlep", valid_flags));
        assert!(!engine.is_likely_flag_typo("--xyz", valid_flags));
    }

    #[test]
    fn test_disabled_suggestions() {
        let mut engine = SuggestionEngine::new();
        engine.set_enabled(false);

        let builtins = vec!["echo".to_string()];
        let suggestions = engine.suggest_command("ehco", &builtins, &[], &[], Path::new("."));
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_suggestion_score_filtering() {
        let config = SuggestionConfig {
            enabled: true,
            max_display: 3,
            min_score: 90, // Very high threshold
        };
        let engine = SuggestionEngine::with_config(config);

        let builtins = vec!["ls".to_string()];
        // "xyz" won't meet the high threshold
        let suggestions = engine.suggest_command("xyz", &builtins, &[], &[], Path::new("."));
        assert!(suggestions.is_empty());
    }
}
