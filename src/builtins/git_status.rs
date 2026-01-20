use crate::executor::ExecutionResult;
use crate::git::GitContext;
use crate::runtime::Runtime;
use anyhow::Result;
use nu_ansi_term::Color;
use std::env;

pub fn builtin_git_status(_args: &[String], runtime: &mut Runtime) -> Result<ExecutionResult> {
    let cwd = runtime.get_cwd();
    let git_ctx = GitContext::new(cwd);

    if !git_ctx.is_git_repo() {
        return Ok(ExecutionResult::error(
            "fatal: not a git repository\n".to_string(),
        ));
    }

    let mut output = String::new();

    // Branch info
    if let Some(branch) = git_ctx.current_branch() {
        output.push_str(&format!("On branch {}\n", Color::Cyan.paint(&branch)));

        // Ahead/behind info
        if let Some((ahead, behind)) = git_ctx.ahead_behind() {
            if ahead > 0 && behind > 0 {
                output.push_str(&format!(
                    "Your branch and 'origin/{}' have diverged,\n",
                    branch
                ));
                output.push_str(&format!(
                    "and have {} and {} different commits each, respectively.\n",
                    ahead, behind
                ));
            } else if ahead > 0 {
                output.push_str(&format!(
                    "Your branch is ahead of 'origin/{}' by {} commit{}.\n",
                    branch,
                    ahead,
                    if ahead == 1 { "" } else { "s" }
                ));
                output.push_str("  (use \"git push\" to publish your local commits)\n");
            } else if behind > 0 {
                output.push_str(&format!(
                    "Your branch is behind 'origin/{}' by {} commit{}.\n",
                    branch,
                    behind,
                    if behind == 1 { "" } else { "s" }
                ));
                output.push_str("  (use \"git pull\" to update your local branch)\n");
            } else {
                output.push_str(&format!(
                    "Your branch is up to date with 'origin/{}'.\n",
                    branch
                ));
            }
        }
    }

    output.push('\n');

    // Status info
    if git_ctx.is_dirty() {
        output.push_str("Changes not staged for commit:\n");
        output.push_str("  (use \"git add <file>...\" to update what will be committed)\n");
        output.push_str("  (use \"git restore <file>...\" to discard changes in working directory)\n");
        // TODO: List actual changed files
    } else {
        output.push_str("nothing to commit, working tree clean\n");
    }

    Ok(ExecutionResult::success(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_status_not_a_repo() {
        let mut runtime = Runtime::new();
        // Set to a non-git directory
        runtime.set_cwd(std::path::PathBuf::from("/tmp"));
        let result = builtin_git_status(&[], &mut runtime).unwrap();
        assert_ne!(result.exit_code, 0);
        assert!(result.stderr.contains("not a git repository"));
    }
}
