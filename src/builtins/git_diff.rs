use crate::executor::ExecutionResult;
use crate::git::GitContext;
use crate::runtime::Runtime;
use anyhow::{anyhow, Result};
use git2::{Diff, DiffDelta, DiffFormat, DiffHunk, DiffLine, DiffOptions, Repository};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct DiffOutput {
    files: Vec<FileDiff>,
    summary: DiffSummary,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileDiff {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_path: Option<String>,
    status: FileStatus,
    additions: usize,
    deletions: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    hunks: Vec<Hunk>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Binary,
}

#[derive(Debug, Serialize, Deserialize)]
struct Hunk {
    old_start: u32,
    old_lines: u32,
    new_start: u32,
    new_lines: u32,
    header: String,
    changes: Vec<LineChange>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LineChange {
    #[serde(rename = "type")]
    change_type: ChangeType,
    line: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ChangeType {
    Context,
    Add,
    Delete,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiffSummary {
    files_changed: usize,
    insertions: usize,
    deletions: usize,
}

pub fn builtin_git_diff(args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    let cwd = runtime.get_cwd();
    let git_ctx = GitContext::new(cwd);

    if !git_ctx.is_git_repo() {
        return Ok(ExecutionResult::error(
            "fatal: not a git repository\n".to_string(),
        ));
    }

    // Parse arguments
    let mut json_output = false;
    let mut staged = false;
    let mut stat_only = false;
    let mut name_only = false;
    let mut paths = Vec::new();
    let mut commit_range: Option<String> = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--json" => json_output = true,
            "--staged" | "--cached" => staged = true,
            "--stat" => stat_only = true,
            "--name-only" => name_only = true,
            arg if arg.contains("..") => commit_range = Some(arg.to_string()),
            arg if !arg.starts_with('-') => paths.push(arg.to_string()),
            _ => {}
        }
        i += 1;
    }

    // Open repository
    let repo = Repository::discover(cwd)
        .map_err(|e| anyhow!("Failed to open git repository: {}", e))?;

    // Get diff
    let diff = get_diff(&repo, staged, commit_range.as_deref(), &paths)?;

    if json_output {
        output_json(&diff, stat_only, name_only)
    } else {
        output_unified(&diff, stat_only, name_only)
    }
}

fn get_diff<'a>(
    repo: &'a Repository,
    staged: bool,
    commit_range: Option<&str>,
    paths: &[String],
) -> Result<Diff<'a>> {
    let mut opts = DiffOptions::new();

    // Set path filters if provided
    for path in paths {
        opts.pathspec(path);
    }

    opts.context_lines(3);

    if let Some(range) = commit_range {
        // Parse commit range (e.g., "HEAD~1..HEAD" or "abc123..def456")
        let parts: Vec<&str> = range.split("..").collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid commit range format. Use format: commit1..commit2"));
        }

        let old_tree = repo
            .revparse_single(parts[0])?
            .peel_to_tree()?;
        let new_tree = repo
            .revparse_single(parts[1])?
            .peel_to_tree()?;

        repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut opts))
            .map_err(|e| anyhow!("Failed to get diff: {}", e))
    } else if staged {
        // Diff between HEAD and index (staged changes)
        let head = repo.head()?.peel_to_tree()?;
        repo.diff_tree_to_index(Some(&head), None, Some(&mut opts))
            .map_err(|e| anyhow!("Failed to get staged diff: {}", e))
    } else {
        // Diff between index and working tree (unstaged changes)
        repo.diff_index_to_workdir(None, Some(&mut opts))
            .map_err(|e| anyhow!("Failed to get working tree diff: {}", e))
    }
}

fn output_json(diff: &Diff, stat_only: bool, name_only: bool) -> Result<ExecutionResult> {
    let mut file_diffs = Vec::new();
    let mut total_insertions = 0;
    let mut total_deletions = 0;

    diff.foreach(
        &mut |delta, _progress| {
            let file_diff = process_delta(&delta, !stat_only && !name_only);
            total_insertions += file_diff.additions;
            total_deletions += file_diff.deletions;
            file_diffs.push(file_diff);
            true
        },
        None,
        if stat_only || name_only {
            None
        } else {
            Some(&mut |delta, hunk| {
                if let Some(file_diff) = file_diffs.last_mut() {
                    if file_diff.path == delta.new_file().path().unwrap().to_string_lossy() {
                        file_diff.hunks.push(process_hunk(&hunk));
                    }
                }
                true
            })
        },
        if stat_only || name_only {
            None
        } else {
            Some(&mut |delta, _hunk, line| {
                if let Some(file_diff) = file_diffs.last_mut() {
                    if file_diff.path == delta.new_file().path().unwrap().to_string_lossy() {
                        if let Some(current_hunk) = file_diff.hunks.last_mut() {
                            current_hunk.changes.push(process_line(&line));
                        }
                    }
                }
                true
            })
        },
    )
    .map_err(|e| anyhow!("Failed to iterate diff: {}", e))?;

    let output = if name_only {
        // Just output file paths
        let paths: Vec<String> = file_diffs.iter().map(|f| f.path.clone()).collect();
        serde_json::to_string_pretty(&paths)?
    } else if stat_only {
        // Just output summary
        let summary = DiffSummary {
            files_changed: file_diffs.len(),
            insertions: total_insertions,
            deletions: total_deletions,
        };
        serde_json::to_string_pretty(&summary)?
    } else {
        // Full output
        let files_changed = file_diffs.len();
        let diff_output = DiffOutput {
            files: file_diffs,
            summary: DiffSummary {
                files_changed,
                insertions: total_insertions,
                deletions: total_deletions,
            },
        };
        serde_json::to_string_pretty(&diff_output)?
    };

    Ok(ExecutionResult::success(output + "\n"))
}

fn output_unified(diff: &Diff, stat_only: bool, name_only: bool) -> Result<ExecutionResult> {
    let mut output = String::new();

    if name_only {
        // Just output file names
        diff.foreach(
            &mut |delta, _progress| {
                if let Some(path) = delta.new_file().path() {
                    output.push_str(&path.to_string_lossy());
                    output.push('\n');
                }
                true
            },
            None,
            None,
            None,
        )
        .map_err(|e| anyhow!("Failed to iterate diff: {}", e))?;
    } else if stat_only {
        // Output stat summary
        let stats = diff.stats()?;
        output.push_str(&format!(
            " {} file{} changed, {} insertion{}(+), {} deletion{}(-)\n",
            stats.files_changed(),
            if stats.files_changed() == 1 { "" } else { "s" },
            stats.insertions(),
            if stats.insertions() == 1 { "" } else { "s" },
            stats.deletions(),
            if stats.deletions() == 1 { "" } else { "s" },
        ));
    } else {
        // Full unified diff output
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            let origin = line.origin();
            let content = std::str::from_utf8(line.content()).unwrap_or("");

            match origin {
                '+' | '-' | ' ' => {
                    output.push(origin);
                    output.push_str(content);
                }
                'F' => {
                    // File header
                    output.push_str("diff --git ");
                    output.push_str(content);
                }
                'H' => {
                    // Hunk header
                    output.push_str("@@ ");
                    output.push_str(content);
                }
                _ => {
                    output.push_str(content);
                }
            }
            true
        })
        .map_err(|e| anyhow!("Failed to print diff: {}", e))?;
    }

    Ok(ExecutionResult::success(output))
}

fn process_delta(delta: &DiffDelta, _include_hunks: bool) -> FileDiff {
    let new_file = delta.new_file();
    let old_file = delta.old_file();

    let path = new_file
        .path()
        .unwrap_or_else(|| std::path::Path::new(""))
        .to_string_lossy()
        .to_string();

    let old_path = if delta.status() == git2::Delta::Renamed {
        old_file
            .path()
            .map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    let status = match delta.status() {
        git2::Delta::Added => FileStatus::Added,
        git2::Delta::Deleted => FileStatus::Deleted,
        git2::Delta::Modified => FileStatus::Modified,
        git2::Delta::Renamed => FileStatus::Renamed,
        git2::Delta::Copied => FileStatus::Copied,
        git2::Delta::Untracked => FileStatus::Untracked,
        _ => FileStatus::Modified,
    };

    // Check if binary
    let is_binary = new_file.is_binary() || old_file.is_binary();

    FileDiff {
        path,
        old_path,
        status: if is_binary { FileStatus::Binary } else { status },
        additions: 0,
        deletions: 0,
        hunks: Vec::new(),
    }
}

fn process_hunk(hunk: &DiffHunk) -> Hunk {
    Hunk {
        old_start: hunk.old_start(),
        old_lines: hunk.old_lines(),
        new_start: hunk.new_start(),
        new_lines: hunk.new_lines(),
        header: String::from_utf8_lossy(hunk.header()).trim().to_string(),
        changes: Vec::new(),
    }
}

fn process_line(line: &DiffLine) -> LineChange {
    let origin = line.origin();
    let content = String::from_utf8_lossy(line.content());
    let content_str = content.trim_end_matches('\n').to_string();

    let change_type = match origin {
        '+' => ChangeType::Add,
        '-' => ChangeType::Delete,
        _ => ChangeType::Context,
    };

    LineChange {
        change_type,
        line: content_str,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> Result<(TempDir, Runtime)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .args(&["init"])
            .current_dir(repo_path)
            .output()?;

        // Configure git
        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()?;

        // Create initial file
        fs::write(repo_path.join("file.txt"), "line1\nline2\nline3\n")?;

        // Add and commit
        Command::new("git")
            .args(&["add", "."])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
            .args(&["commit", "-m", "initial commit"])
            .current_dir(repo_path)
            .output()?;

        let mut runtime = Runtime::new();
        runtime.set_cwd(repo_path.to_path_buf());

        Ok((temp_dir, runtime))
    }

    #[test]
    fn test_git_diff_not_a_repo() {
        let mut runtime = Runtime::new();
        runtime.set_cwd(std::path::PathBuf::from("/tmp"));
        let result = builtin_git_diff(&[], &mut runtime).unwrap();
        assert_ne!(result.exit_code, 0);
        assert!(result.stderr.contains("not a git repository"));
    }

    #[test]
    fn test_git_diff_no_changes() -> Result<()> {
        let (_temp_dir, mut runtime) = setup_test_repo()?;
        let result = builtin_git_diff(&[], &mut runtime)?;
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout().trim(), "");
        Ok(())
    }

    #[test]
    fn test_git_diff_modified_file() -> Result<()> {
        let (temp_dir, mut runtime) = setup_test_repo()?;
        let repo_path = temp_dir.path();

        // Modify file
        fs::write(repo_path.join("file.txt"), "line1\nmodified line2\nline3\n")?;

        let result = builtin_git_diff(&[], &mut runtime)?;
        assert_eq!(result.exit_code, 0);
        let output = result.stdout();
        assert!(output.contains("file.txt"));
        assert!(output.contains("-line2"));
        assert!(output.contains("+modified line2"));
        Ok(())
    }

    #[test]
    fn test_git_diff_json_output() -> Result<()> {
        let (temp_dir, mut runtime) = setup_test_repo()?;
        let repo_path = temp_dir.path();

        // Modify file
        fs::write(repo_path.join("file.txt"), "line1\nmodified line2\nline3\n")?;

        let result = builtin_git_diff(&["--json".to_string()], &mut runtime)?;
        assert_eq!(result.exit_code, 0);

        let output = result.stdout();
        let diff_output: DiffOutput = serde_json::from_str(output.trim())?;

        assert_eq!(diff_output.summary.files_changed, 1);
        assert_eq!(diff_output.files[0].path, "file.txt");
        assert!(matches!(diff_output.files[0].status, FileStatus::Modified));
        Ok(())
    }

    #[test]
    fn test_git_diff_staged() -> Result<()> {
        let (temp_dir, mut runtime) = setup_test_repo()?;
        let repo_path = temp_dir.path();

        // Modify and stage file
        fs::write(repo_path.join("file.txt"), "line1\nmodified line2\nline3\n")?;
        Command::new("git")
            .args(&["add", "file.txt"])
            .current_dir(repo_path)
            .output()?;

        let result = builtin_git_diff(&["--staged".to_string()], &mut runtime)?;
        assert_eq!(result.exit_code, 0);
        let output = result.stdout();
        assert!(output.contains("file.txt"));
        assert!(output.contains("-line2"));
        assert!(output.contains("+modified line2"));
        Ok(())
    }

    #[test]
    fn test_git_diff_stat_only() -> Result<()> {
        let (temp_dir, mut runtime) = setup_test_repo()?;
        let repo_path = temp_dir.path();

        // Modify file
        fs::write(repo_path.join("file.txt"), "line1\nmodified line2\nline3\nline4\n")?;

        let result = builtin_git_diff(&["--stat".to_string()], &mut runtime)?;
        assert_eq!(result.exit_code, 0);
        let output = result.stdout();
        assert!(output.contains("1 file"));
        assert!(output.contains("insertion"));
        Ok(())
    }

    #[test]
    fn test_git_diff_json_stat() -> Result<()> {
        let (temp_dir, mut runtime) = setup_test_repo()?;
        let repo_path = temp_dir.path();

        // Modify file
        fs::write(repo_path.join("file.txt"), "line1\nmodified line2\nline3\nline4\n")?;

        let result = builtin_git_diff(&["--json".to_string(), "--stat".to_string()], &mut runtime)?;
        assert_eq!(result.exit_code, 0);

        let output = result.stdout();
        let summary: DiffSummary = serde_json::from_str(output.trim())?;
        assert_eq!(summary.files_changed, 1);
        Ok(())
    }

    #[test]
    fn test_git_diff_name_only() -> Result<()> {
        let (temp_dir, mut runtime) = setup_test_repo()?;
        let repo_path = temp_dir.path();

        // Modify file
        fs::write(repo_path.join("file.txt"), "line1\nmodified\nline3\n")?;
        fs::write(repo_path.join("file2.txt"), "new file\n")?;

        let result = builtin_git_diff(&["--name-only".to_string()], &mut runtime)?;
        assert_eq!(result.exit_code, 0);
        let output = result.stdout();
        assert!(output.contains("file.txt"));
        Ok(())
    }

    #[test]
    fn test_git_diff_specific_file() -> Result<()> {
        let (temp_dir, mut runtime) = setup_test_repo()?;
        let repo_path = temp_dir.path();

        // Modify multiple files
        fs::write(repo_path.join("file.txt"), "modified\n")?;
        fs::write(repo_path.join("other.txt"), "other\n")?;

        let result = builtin_git_diff(&["file.txt".to_string()], &mut runtime)?;
        assert_eq!(result.exit_code, 0);
        let output = result.stdout();
        assert!(output.contains("file.txt"));
        assert!(!output.contains("other.txt"));
        Ok(())
    }
}
