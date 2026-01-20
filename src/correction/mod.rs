use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use strsim::jaro_winkler;
use std::path::{Path, PathBuf};
use std::env;
use std::fs;

/// Suggests corrections for mistyped commands or paths
pub struct Corrector {
    matcher: SkimMatcherV2,
}

impl Clone for Corrector {
    fn clone(&self) -> Self {
        // SkimMatcherV2 doesn't implement Clone, so create a new instance
        Self::new()
    }
}

impl Corrector {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Calculate a combined score using both edit distance and fuzzy matching
    fn calculate_score(&self, target: &str, input: &str) -> i64 {
        // Use Jaro-Winkler distance (0.0 to 1.0, higher is better)
        let edit_score = jaro_winkler(target, input);
        
        // Convert to a score similar to fuzzy matcher (0-100+ range)
        let normalized_score = (edit_score * 100.0) as i64;
        
        // Also try fuzzy matching for subsequence matches
        let fuzzy_score = self.matcher.fuzzy_match(target, input).unwrap_or(0);
        
        // Take the maximum of both approaches
        normalized_score.max(fuzzy_score)
    }

    /// Suggest corrections for a command that wasn't found
    /// Returns suggestions sorted by similarity score (best first)
    pub fn suggest_command(&self, input: &str, builtins: &[String]) -> Vec<Suggestion> {
        let mut builtin_suggestions = Vec::new();
        let mut path_suggestions = Vec::new();

        // Check builtins first
        for builtin in builtins {
            let score = self.calculate_score(builtin, input);
            if score > 30 {  // Minimum threshold
                builtin_suggestions.push(Suggestion {
                    text: builtin.clone(),
                    score,
                    kind: SuggestionKind::Builtin,
                });
            }
        }

        // Check PATH executables only if we don't have good builtin matches
        if builtin_suggestions.is_empty() || builtin_suggestions[0].score < 80 {
            if let Ok(path_var) = env::var("PATH") {
                for path_dir in path_var.split(':') {
                    if let Ok(entries) = fs::read_dir(path_dir) {
                        for entry in entries.flatten() {
                            if let Ok(file_name) = entry.file_name().into_string() {
                                let score = self.calculate_score(&file_name, input);
                                if score > 50 {  // Higher threshold for PATH commands
                                    path_suggestions.push(Suggestion {
                                        text: file_name,
                                        score,
                                        kind: SuggestionKind::Executable,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort both lists by score
        builtin_suggestions.sort_by(|a, b| b.score.cmp(&a.score));
        path_suggestions.sort_by(|a, b| b.score.cmp(&a.score));

        // Combine: take top 3 builtins first, then fill with PATH commands
        let mut suggestions = Vec::new();
        suggestions.extend(builtin_suggestions.into_iter().take(3));
        
        // Add path suggestions to fill up to 5 total
        let remaining = 5 - suggestions.len();
        suggestions.extend(path_suggestions.into_iter().take(remaining));

        suggestions
    }

    /// Suggest corrections for a path that doesn't exist
    /// Looks in the parent directory and current directory for similar names
    pub fn suggest_path(&self, input: &Path, current_dir: &Path) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Get the parent directory and file name
        let (search_dir, target_name) = if let Some(parent) = input.parent() {
            let name = input.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            (parent, name)
        } else {
            (Path::new("."), input.to_str().unwrap_or(""))
        };

        // Make search_dir absolute if it's relative
        let search_path = if search_dir.is_absolute() {
            search_dir.to_path_buf()
        } else {
            current_dir.join(search_dir)
        };

        // Search in the parent/current directory
        if let Ok(entries) = fs::read_dir(&search_path) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    let score = self.calculate_score(&file_name, target_name);
                    if score > 30 {
                        let full_path = if search_dir.as_os_str().is_empty() {
                            PathBuf::from(&file_name)
                        } else {
                            search_dir.join(&file_name)
                        };

                        suggestions.push(Suggestion {
                            text: full_path.to_string_lossy().to_string(),
                            score,
                            kind: if entry.path().is_dir() {
                                SuggestionKind::Directory
                            } else {
                                SuggestionKind::File
                            },
                        });
                    }
                }
            }
        }

        // Also check current directory if input was absolute
        if input.is_absolute() && &search_path != current_dir {
            if let Ok(entries) = fs::read_dir(current_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        let score = self.calculate_score(&file_name, target_name);
                        if score > 30 {
                            suggestions.push(Suggestion {
                                text: current_dir.join(&file_name).to_string_lossy().to_string(),
                                score,
                                kind: SuggestionKind::Recent,
                            });
                        }
                    }
                }
            }
        }

        // Sort by score (descending) and take top 3
        suggestions.sort_by(|a, b| b.score.cmp(&a.score));
        suggestions.truncate(3);
        suggestions
    }

    /// Calculate similarity percentage for display
    pub fn similarity_percent(score: i64, text: &str) -> u8 {
        // Rough heuristic: score relative to string length
        let max_score = text.len() as i64 * 10; // Approximate max score
        let percent = ((score as f64 / max_score as f64) * 100.0).min(100.0).max(0.0);
        percent as u8
    }
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub text: String,
    pub score: i64,
    pub kind: SuggestionKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SuggestionKind {
    Builtin,
    Executable,
    Directory,
    File,
    Recent,
}

impl SuggestionKind {
    pub fn label(&self) -> &str {
        match self {
            SuggestionKind::Builtin => "builtin",
            SuggestionKind::Executable => "command",
            SuggestionKind::Directory => "directory",
            SuggestionKind::File => "file",
            SuggestionKind::Recent => "recent",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_suggestion() {
        let corrector = Corrector::new();
        let builtins = vec!["echo".to_string(), "grep".to_string(), "find".to_string()];

        let suggestions = corrector.suggest_command("ehco", &builtins);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].text, "echo");
    }

    #[test]
    fn test_command_suggestion_multiple() {
        let corrector = Corrector::new();
        let builtins = vec![
            "grep".to_string(),
            "git".to_string(),
            "get".to_string(),
        ];

        let suggestions = corrector.suggest_command("gre", &builtins);
        assert!(!suggestions.is_empty());
        // Should match both "grep" and "get" with decent scores
        assert!(suggestions.iter().any(|s| s.text == "grep"));
    }

    #[test]
    fn test_similarity_percent() {
        let score = 50;
        let text = "example";
        let percent = Corrector::similarity_percent(score, text);
        assert!(percent > 0 && percent <= 100);
    }

    #[test]
    fn test_path_suggestion_current_dir() {
        let corrector = Corrector::new();
        let current_dir = std::env::current_dir().unwrap();

        // Try to find suggestions for a likely misspelling
        let suggestions = corrector.suggest_path(Path::new("srcc"), &current_dir);

        // If src exists, we should find it. Otherwise just verify we get some suggestions.
        if current_dir.join("src").exists() {
            assert!(suggestions.iter().any(|s| s.text.contains("src")), 
                "Expected to find 'src' in suggestions: {:?}", suggestions);
        } else {
            // Just verify the function runs without error
            // Suggestions may or may not be empty depending on directory contents
        }
    }
}
