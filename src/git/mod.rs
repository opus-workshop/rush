use git2::{Repository, StatusOptions};
use anyhow::{anyhow, Result};
use std::path::Path;

pub struct GitContext {
    repo: Option<Repository>,
}

impl GitContext {
    pub fn new(path: &Path) -> Self {
        let repo = Repository::discover(path).ok();
        Self { repo }
    }

    pub fn is_git_repo(&self) -> bool {
        self.repo.is_some()
    }

    pub fn current_branch(&self) -> Option<String> {
        let repo = self.repo.as_ref()?;
        let head = repo.head().ok()?;
        let branch_name = head.shorthand()?;
        Some(branch_name.to_string())
    }

    pub fn is_dirty(&self) -> bool {
        if let Some(repo) = &self.repo {
            if let Ok(statuses) = repo.statuses(Some(StatusOptions::new().include_untracked(true))) {
                return !statuses.is_empty();
            }
        }
        false
    }

    pub fn ahead_behind(&self) -> Option<(usize, usize)> {
        let repo = self.repo.as_ref()?;
        let head = repo.head().ok()?;
        let local_oid = head.target()?;

        // Get upstream branch
        let branch = repo.find_branch(head.shorthand()?, git2::BranchType::Local).ok()?;
        let upstream = branch.upstream().ok()?;
        let upstream_oid = upstream.get().target()?;

        // Calculate ahead/behind
        let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid).ok()?;
        Some((ahead, behind))
    }

    pub fn status_summary(&self) -> GitStatus {
        if !self.is_git_repo() {
            return GitStatus::NotGit;
        }

        let branch = self.current_branch();
        let dirty = self.is_dirty();
        let ahead_behind = self.ahead_behind();

        GitStatus::InRepo {
            branch,
            dirty,
            ahead_behind,
        }
    }
}

#[derive(Debug, Clone)]
pub enum GitStatus {
    NotGit,
    InRepo {
        branch: Option<String>,
        dirty: bool,
        ahead_behind: Option<(usize, usize)>,
    },
}

impl GitStatus {
    pub fn prompt_string(&self) -> String {
        match self {
            GitStatus::NotGit => String::new(),
            GitStatus::InRepo {
                branch,
                dirty,
                ahead_behind,
            } => {
                let mut parts = Vec::new();

                if let Some(branch_name) = branch {
                    parts.push(branch_name.clone());
                }

                if *dirty {
                    parts.push("✗".to_string());
                } else {
                    parts.push("✓".to_string());
                }

                if let Some((ahead, behind)) = ahead_behind {
                    if *ahead > 0 {
                        parts.push(format!("↑{}", ahead));
                    }
                    if *behind > 0 {
                        parts.push(format!("↓{}", behind));
                    }
                }

                format!("({})", parts.join(" "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_context_not_a_repo() {
        let ctx = GitContext::new(Path::new("/tmp"));
        // /tmp might or might not be a git repo, just test it doesn't crash
        let _ = ctx.is_git_repo();
    }

    #[test]
    fn test_status_prompt_not_git() {
        let status = GitStatus::NotGit;
        assert_eq!(status.prompt_string(), "");
    }

    #[test]
    fn test_status_prompt_clean() {
        let status = GitStatus::InRepo {
            branch: Some("main".to_string()),
            dirty: false,
            ahead_behind: None,
        };
        assert_eq!(status.prompt_string(), "(main ✓)");
    }

    #[test]
    fn test_status_prompt_dirty_with_ahead() {
        let status = GitStatus::InRepo {
            branch: Some("feature".to_string()),
            dirty: true,
            ahead_behind: Some((2, 0)),
        };
        assert_eq!(status.prompt_string(), "(feature ✗ ↑2)");
    }
}
