//! Git operations for DeliDev including worktree management
//!
//! This crate provides git operations extracted from the desktop app,
//! making them available for use in the worker server and other components.
//!
//! Note: On mobile platforms (Android/iOS), git operations are not available
//! and stub implementations are provided.

// Desktop implementation with git2
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod branch;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod diff;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod error;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod worktree;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::path::Path;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use branch::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use diff::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use error::*;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use worktree::*;

// Mobile stub implementation
#[cfg(any(target_os = "android", target_os = "ios"))]
mod mobile_stub;
#[cfg(any(target_os = "android", target_os = "ios"))]
pub use mobile_stub::*;

/// Main service for git operations
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub struct GitOperations;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl GitOperations {
    /// Opens a repository at the given path
    pub fn open_repo(repo_path: &Path) -> GitResult<git2::Repository> {
        git2::Repository::open(repo_path).map_err(|e| {
            if e.code() == git2::ErrorCode::NotFound {
                GitError::RepositoryNotFound(repo_path.display().to_string())
            } else {
                GitError::Git(e)
            }
        })
    }

    /// Gets the current branch name
    pub fn current_branch(repo_path: &Path) -> GitResult<String> {
        let repo = Self::open_repo(repo_path)?;
        let head = repo.head()?;

        if head.is_branch() {
            if let Some(name) = head.shorthand() {
                return Ok(name.to_string());
            }
        }

        // If detached HEAD, return the commit hash
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string()[..7].to_string())
    }

    /// Gets the default branch name (main or master)
    pub fn default_branch(repo_path: &Path) -> GitResult<String> {
        let repo = Self::open_repo(repo_path)?;

        // Try common default branch names
        for branch_name in &["main", "master"] {
            if repo
                .find_branch(branch_name, git2::BranchType::Local)
                .is_ok()
            {
                return Ok(branch_name.to_string());
            }
        }

        // Try to get from origin/HEAD
        if let Ok(reference) = repo.find_reference("refs/remotes/origin/HEAD") {
            if let Some(target) = reference.symbolic_target() {
                if let Some(branch) = target.strip_prefix("refs/remotes/origin/") {
                    return Ok(branch.to_string());
                }
            }
        }

        // Default to main
        Ok("main".to_string())
    }

    /// Gets repository information
    pub fn get_repo_info(repo_path: &Path) -> GitResult<RepoInfo> {
        let repo = Self::open_repo(repo_path)?;

        let name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let current_branch = Self::current_branch(repo_path)?;
        let default_branch = Self::default_branch(repo_path)?;

        // Try to get remote URL
        let remote_url = repo
            .find_remote("origin")
            .ok()
            .and_then(|remote| remote.url().map(|s| s.to_string()));

        Ok(RepoInfo {
            name,
            path: repo_path.to_path_buf(),
            current_branch,
            default_branch,
            remote_url,
        })
    }
}

/// Repository information
#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[derive(Debug, Clone)]
pub struct RepoInfo {
    /// Repository name
    pub name: String,
    /// Local path
    pub path: std::path::PathBuf,
    /// Current branch
    pub current_branch: String,
    /// Default branch
    pub default_branch: String,
    /// Remote URL (if any)
    pub remote_url: Option<String>,
}

/// Appends a short unique suffix to a branch name to prevent conflicts.
pub fn make_branch_name_unique(branch_name: &str) -> String {
    let suffix = &uuid::Uuid::new_v4().to_string()[..8];
    format!("{}-{}", branch_name, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_branch_name_unique() {
        let branch1 = make_branch_name_unique("feature/test");
        let branch2 = make_branch_name_unique("feature/test");

        assert!(branch1.starts_with("feature/test-"));
        assert!(branch2.starts_with("feature/test-"));
        assert_ne!(branch1, branch2);
    }
}
