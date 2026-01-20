use anyhow::{anyhow, Result};
use glob::{glob_with, MatchOptions};
use std::path::PathBuf;

/// Expand glob patterns into a list of matching file paths
///
/// Features:
/// - `*` matches any characters
/// - `?` matches a single character
/// - `[...]` character classes
/// - `**` recursive glob (matches directories recursively)
/// - Dotfiles are not matched by default (unless explicitly specified)
/// - Empty glob results return an error (not the literal)
pub fn expand_globs(pattern: &str, cwd: &std::path::Path) -> Result<Vec<String>> {
    // Check if the pattern contains glob metacharacters
    if !contains_glob_pattern(pattern) {
        // Not a glob pattern, return as-is
        return Ok(vec![pattern.to_string()]);
    }

    // Configure match options
    // If pattern explicitly starts with a dot, allow * to match dotfiles
    // Otherwise, require a literal dot to match files starting with .
    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: !pattern.starts_with('.'), // Allow .* to match dotfiles
    };

    // Resolve the pattern relative to cwd
    let absolute_pattern = if pattern.starts_with('/') {
        pattern.to_string()
    } else {
        cwd.join(pattern).to_string_lossy().to_string()
    };

    // Perform glob expansion
    let mut matches: Vec<PathBuf> = glob_with(&absolute_pattern, options)
        .map_err(|e| anyhow!("Invalid glob pattern '{}': {}", pattern, e))?
        .filter_map(Result::ok)
        .collect();

    // Sort matches for consistent output
    matches.sort();

    if matches.is_empty() {
        // No matches - return error per requirements
        return Err(anyhow!("No matches found for pattern: {}", pattern));
    }

    // Convert PathBuf to String, making paths relative to cwd if they're under cwd
    let results: Vec<String> = matches
        .into_iter()
        .map(|path| {
            // Try to make the path relative to cwd for cleaner output
            if let Ok(relative) = path.strip_prefix(cwd) {
                relative.to_string_lossy().to_string()
            } else {
                path.to_string_lossy().to_string()
            }
        })
        .collect();

    Ok(results)
}

/// Check if a string contains glob metacharacters
fn contains_glob_pattern(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

/// Expand multiple glob patterns
///
/// Each pattern is expanded independently, and all results are combined.
/// If any pattern fails to match, an error is returned.
pub fn expand_multiple_globs(patterns: &[String], cwd: &std::path::Path) -> Result<Vec<String>> {
    let mut all_matches = Vec::new();

    for pattern in patterns {
        let matches = expand_globs(pattern, cwd)?;
        all_matches.extend(matches);
    }

    Ok(all_matches)
}

/// Check if a pattern should be expanded as a glob
pub fn should_expand_glob(arg: &str) -> bool {
    contains_glob_pattern(arg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_contains_glob_pattern() {
        assert!(contains_glob_pattern("*.txt"));
        assert!(contains_glob_pattern("file?.txt"));
        assert!(contains_glob_pattern("file[123].txt"));
        assert!(contains_glob_pattern("**/*.rs"));
        assert!(!contains_glob_pattern("file.txt"));
        assert!(!contains_glob_pattern("some/path/to/file"));
    }

    #[test]
    fn test_asterisk_glob() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create test files
        fs::write(temp_path.join("file1.txt"), "").unwrap();
        fs::write(temp_path.join("file2.txt"), "").unwrap();
        fs::write(temp_path.join("other.md"), "").unwrap();

        let matches = expand_globs("*.txt", temp_path).unwrap();
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&"file1.txt".to_string()));
        assert!(matches.contains(&"file2.txt".to_string()));
    }

    #[test]
    fn test_question_mark_glob() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create test files
        fs::write(temp_path.join("file1.txt"), "").unwrap();
        fs::write(temp_path.join("file2.txt"), "").unwrap();
        fs::write(temp_path.join("file10.txt"), "").unwrap();

        let matches = expand_globs("file?.txt", temp_path).unwrap();
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&"file1.txt".to_string()));
        assert!(matches.contains(&"file2.txt".to_string()));
        assert!(!matches.contains(&"file10.txt".to_string()));
    }

    #[test]
    fn test_character_class_glob() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create test files
        fs::write(temp_path.join("file1.txt"), "").unwrap();
        fs::write(temp_path.join("file2.txt"), "").unwrap();
        fs::write(temp_path.join("file3.txt"), "").unwrap();
        fs::write(temp_path.join("file4.txt"), "").unwrap();

        let matches = expand_globs("file[12].txt", temp_path).unwrap();
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&"file1.txt".to_string()));
        assert!(matches.contains(&"file2.txt".to_string()));
        assert!(!matches.contains(&"file3.txt".to_string()));
    }

    #[test]
    fn test_recursive_glob() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create nested directories
        fs::create_dir(temp_path.join("dir1")).unwrap();
        fs::create_dir(temp_path.join("dir1/subdir")).unwrap();
        fs::write(temp_path.join("file.rs"), "").unwrap();
        fs::write(temp_path.join("dir1/file.rs"), "").unwrap();
        fs::write(temp_path.join("dir1/subdir/file.rs"), "").unwrap();

        let matches = expand_globs("**/*.rs", temp_path).unwrap();
        assert!(matches.len() >= 3);
        assert!(matches.iter().any(|s| s.contains("file.rs")));
        assert!(matches.iter().any(|s| s.contains("dir1")));
    }

    #[test]
    fn test_dotfiles_not_matched_by_default() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create files including dotfiles
        fs::write(temp_path.join("file.txt"), "").unwrap();
        fs::write(temp_path.join(".hidden"), "").unwrap();

        let matches = expand_globs("*", temp_path).unwrap();
        assert!(matches.contains(&"file.txt".to_string()));
        assert!(!matches.contains(&".hidden".to_string()));
    }

    #[test]
    fn test_dotfiles_matched_explicitly() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create hidden files
        fs::write(temp_path.join(".gitignore"), "").unwrap();
        fs::write(temp_path.join(".gitattributes"), "").unwrap();
        fs::write(temp_path.join("regular.txt"), "").unwrap();

        // Pattern with explicit leading dot and specific name should match dotfiles
        let matches = expand_globs(".git*", temp_path).unwrap();
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&".gitignore".to_string()));
        assert!(matches.contains(&".gitattributes".to_string()));
        assert!(!matches.contains(&"regular.txt".to_string()));
    }

    #[test]
    fn test_empty_glob_returns_error() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // No files created, glob should fail
        let result = expand_globs("*.nonexistent", temp_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No matches found"));
    }

    #[test]
    fn test_non_glob_pattern_returns_as_is() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        let matches = expand_globs("literal_file.txt", temp_path).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], "literal_file.txt");
    }

    #[test]
    fn test_expand_multiple_globs() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create test files
        fs::write(temp_path.join("file1.txt"), "").unwrap();
        fs::write(temp_path.join("file2.txt"), "").unwrap();
        fs::write(temp_path.join("doc.md"), "").unwrap();

        let patterns = vec!["*.txt".to_string(), "*.md".to_string()];
        let matches = expand_multiple_globs(&patterns, temp_path).unwrap();

        assert_eq!(matches.len(), 3);
        assert!(matches.contains(&"file1.txt".to_string()));
        assert!(matches.contains(&"file2.txt".to_string()));
        assert!(matches.contains(&"doc.md".to_string()));
    }

    #[test]
    fn test_sorted_output() {
        let temp = TempDir::new().unwrap();
        let temp_path = temp.path();

        // Create files in non-alphabetical order
        fs::write(temp_path.join("zebra.txt"), "").unwrap();
        fs::write(temp_path.join("apple.txt"), "").unwrap();
        fs::write(temp_path.join("mango.txt"), "").unwrap();

        let matches = expand_globs("*.txt", temp_path).unwrap();
        assert_eq!(matches, vec!["apple.txt", "mango.txt", "zebra.txt"]);
    }
}
