//! Git worktree management

use std::path::Path;

use crate::{GitError, GitOperations, GitResult};

impl GitOperations {
    /// Creates a new worktree for isolated task execution
    ///
    /// This creates a new worktree at `worktree_path` based on `base_branch`,
    /// with a new branch named `branch_name`.
    pub fn create_worktree(
        repo_path: &Path,
        worktree_path: &Path,
        branch_name: &str,
        base_branch: &str,
    ) -> GitResult<()> {
        let repo = Self::open_repo(repo_path)?;

        // Check if worktree path already exists
        if worktree_path.exists() {
            return Err(GitError::WorktreeExists(
                worktree_path.display().to_string(),
            ));
        }

        // Create parent directories
        if let Some(parent) = worktree_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create the branch first if it doesn't exist
        if repo
            .find_branch(branch_name, git2::BranchType::Local)
            .is_err()
        {
            Self::create_branch_internal(&repo, branch_name, base_branch)?;
        }

        // Get the branch reference
        let branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
        let reference = branch.into_reference();

        // Create the worktree
        let worktree_name = worktree_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("worktree");

        repo.worktree(
            worktree_name,
            worktree_path,
            Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
        )?;

        Ok(())
    }

    /// Removes a worktree and cleans up
    pub fn remove_worktree(repo_path: &Path, worktree_path: &Path) -> GitResult<()> {
        let repo = Self::open_repo(repo_path)?;

        // Find the worktree by path
        let worktree_name = worktree_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("worktree");

        // Try to find and prune the worktree
        if let Ok(worktree) = repo.find_worktree(worktree_name) {
            // Prune if locked or valid
            let _ = worktree.prune(Some(
                git2::WorktreePruneOptions::new()
                    .working_tree(true)
                    .valid(true)
                    .locked(true),
            ));
        }

        // Remove the directory
        if worktree_path.exists() {
            std::fs::remove_dir_all(worktree_path)?;
        }

        Ok(())
    }

    /// Lists all worktrees for a repository
    pub fn list_worktrees(repo_path: &Path) -> GitResult<Vec<WorktreeInfo>> {
        let repo = Self::open_repo(repo_path)?;
        let worktrees = repo.worktrees()?;

        let mut result = Vec::new();
        for name in worktrees.iter().flatten() {
            if let Ok(worktree) = repo.find_worktree(name) {
                let path = worktree.path().to_path_buf();
                let is_locked = worktree.is_locked().is_ok();
                let is_prunable = worktree.validate().is_err();

                result.push(WorktreeInfo {
                    name: name.to_string(),
                    path,
                    is_locked,
                    is_prunable,
                });
            }
        }

        Ok(result)
    }

    /// Checks if a worktree exists
    pub fn worktree_exists(repo_path: &Path, worktree_name: &str) -> GitResult<bool> {
        let repo = Self::open_repo(repo_path)?;
        Ok(repo.find_worktree(worktree_name).is_ok())
    }

    /// Internal helper to create a branch
    fn create_branch_internal(
        repo: &git2::Repository,
        branch_name: &str,
        base_branch: &str,
    ) -> GitResult<()> {
        // Find the base branch
        let base_ref = repo
            .find_branch(base_branch, git2::BranchType::Local)
            .or_else(|_| {
                let remote_name = format!("origin/{}", base_branch);
                repo.find_branch(&remote_name, git2::BranchType::Remote)
            })
            .map_err(|_| GitError::BranchNotFound(base_branch.to_string()))?;

        let commit = base_ref.get().peel_to_commit()?;

        // Create new branch
        repo.branch(branch_name, &commit, false)?;

        Ok(())
    }
}

/// Information about a worktree
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Worktree name
    pub name: String,
    /// Worktree path
    pub path: std::path::PathBuf,
    /// Whether the worktree is locked
    pub is_locked: bool,
    /// Whether the worktree is prunable
    pub is_prunable: bool,
}
