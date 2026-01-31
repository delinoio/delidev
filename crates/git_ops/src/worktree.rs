//! Git worktree operations.

use std::path::{Path, PathBuf};

use tracing::info;

use crate::{GitError, GitRepository, GitResult};

/// Options for creating a worktree.
#[derive(Debug, Clone, Default)]
pub struct WorktreeOptions {
    /// Branch name for the worktree.
    pub branch: Option<String>,
    /// Whether to create the branch if it doesn't exist.
    pub create_branch: bool,
    /// Base commit/branch for the new branch.
    pub base: Option<String>,
}

/// A git worktree.
pub struct Worktree {
    /// Path to the worktree.
    pub path: PathBuf,
    /// Name of the worktree.
    pub name: String,
    /// Branch checked out in the worktree.
    pub branch: Option<String>,
}

impl Worktree {
    /// Opens an existing worktree at the given path.
    pub fn open(path: impl AsRef<Path>) -> GitResult<GitRepository> {
        GitRepository::open(path)
    }
}

/// Extension trait for worktree operations on GitRepository.
pub trait WorktreeExt {
    /// Lists all worktrees.
    fn list_worktrees(&self) -> GitResult<Vec<String>>;

    /// Creates a new worktree.
    fn create_worktree(
        &self,
        name: &str,
        path: impl AsRef<Path>,
        options: WorktreeOptions,
    ) -> GitResult<Worktree>;

    /// Removes a worktree.
    fn remove_worktree(&self, name: &str, force: bool) -> GitResult<()>;

    /// Prunes stale worktrees.
    fn prune_worktrees(&self) -> GitResult<()>;
}

impl WorktreeExt for GitRepository {
    fn list_worktrees(&self) -> GitResult<Vec<String>> {
        let worktrees = self.inner().worktrees()?;
        Ok(worktrees
            .iter()
            .filter_map(|s| s.map(String::from))
            .collect())
    }

    fn create_worktree(
        &self,
        name: &str,
        path: impl AsRef<Path>,
        options: WorktreeOptions,
    ) -> GitResult<Worktree> {
        let path = path.as_ref();
        let repo = self.inner();

        info!("Creating worktree '{}' at {:?}", name, path);

        // Determine the branch
        let branch_name = options.branch.unwrap_or_else(|| name.to_string());

        // Create branch if needed
        if options.create_branch {
            let base = options.base.as_deref().unwrap_or("HEAD");
            let commit = repo.revparse_single(base)?.peel_to_commit()?;

            // Check if branch exists
            if repo
                .find_branch(&branch_name, git2::BranchType::Local)
                .is_err()
            {
                repo.branch(&branch_name, &commit, false)?;
            }
        }

        // Find the reference for the branch
        let reference = repo.find_branch(&branch_name, git2::BranchType::Local)?;
        let reference_name = reference
            .get()
            .name()
            .ok_or_else(|| GitError::Other("Invalid branch reference".to_string()))?;

        // Create the worktree
        repo.worktree(
            name,
            path,
            Some(
                git2::WorktreeAddOptions::new()
                    .reference(Some(&repo.find_reference(reference_name)?)),
            ),
        )?;

        Ok(Worktree {
            path: path.to_path_buf(),
            name: name.to_string(),
            branch: Some(branch_name),
        })
    }

    fn remove_worktree(&self, name: &str, force: bool) -> GitResult<()> {
        let worktree = self.inner().find_worktree(name)?;

        if force {
            worktree.prune(Some(
                git2::WorktreePruneOptions::new()
                    .valid(true)
                    .locked(true)
                    .working_tree(true),
            ))?;
        } else {
            worktree.prune(None)?;
        }

        Ok(())
    }

    fn prune_worktrees(&self) -> GitResult<()> {
        for name in self.list_worktrees()? {
            if let Ok(worktree) = self.inner().find_worktree(&name) {
                // Only prune if not valid
                if worktree.validate().is_err() {
                    let _ = worktree.prune(None);
                }
            }
        }
        Ok(())
    }
}

/// Creates a unique worktree path for a task.
pub fn worktree_path_for_task(base_dir: impl AsRef<Path>, task_id: &str) -> PathBuf {
    base_dir.as_ref().join("worktrees").join(task_id)
}

/// Generates a branch name from a task.
pub fn branch_name_for_task(task_id: &str, slug: Option<&str>, template: Option<&str>) -> String {
    let template = template.unwrap_or("delidev/${taskId}");

    let mut result = template.replace("${taskId}", task_id);
    if let Some(slug) = slug {
        result = result.replace("${slug}", slug);
    } else {
        result = result.replace("-${slug}", "").replace("${slug}", "");
    }

    // Clean up the branch name
    result
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '/' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_name_generation() {
        assert_eq!(branch_name_for_task("abc123", None, None), "delidev/abc123");

        assert_eq!(
            branch_name_for_task("abc123", Some("fix-bug"), Some("feature/${taskId}-${slug}")),
            "feature/abc123-fix-bug"
        );

        // When slug is None, the "-${slug}" pattern is removed entirely
        assert_eq!(
            branch_name_for_task("abc123", None, Some("feature/${taskId}-${slug}")),
            "feature/abc123"
        );
    }

    #[test]
    fn test_worktree_path_for_task() {
        let path = worktree_path_for_task("/home/user/.delidev", "task-123");
        assert_eq!(
            path,
            PathBuf::from("/home/user/.delidev/worktrees/task-123")
        );
    }
}
