//! Mobile stub implementations for git operations
//!
//! On mobile platforms (Android/iOS), git2 is not available.
//! This module provides stub types and error handling.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during git operations
#[derive(Error, Debug)]
pub enum GitError {
    /// Repository not found
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),

    /// Branch not found
    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    /// Worktree already exists
    #[error("Worktree already exists: {0}")]
    WorktreeExists(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Push failed
    #[error("Push failed: {0}")]
    PushFailed(String),

    /// Merge conflict
    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    /// Invalid reference
    #[error("Invalid reference: {0}")]
    InvalidReference(String),

    /// Commit not found
    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    /// Not supported on mobile
    #[error("Git operations are not supported on mobile platforms")]
    NotSupportedOnMobile,
}

/// Result type for git operations
pub type GitResult<T> = Result<T, GitError>;

/// Main service for git operations (stub for mobile)
pub struct GitOperations;

impl GitOperations {
    /// Gets the current branch name (stub)
    pub fn current_branch(_repo_path: &Path) -> GitResult<String> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets the default branch name (stub)
    pub fn default_branch(_repo_path: &Path) -> GitResult<String> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets repository information (stub)
    pub fn get_repo_info(_repo_path: &Path) -> GitResult<RepoInfo> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Creates a new worktree (stub)
    pub fn create_worktree(
        _repo_path: &Path,
        _worktree_path: &Path,
        _branch_name: &str,
        _base_branch: &str,
    ) -> GitResult<()> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Removes a worktree (stub)
    pub fn remove_worktree(_repo_path: &Path, _worktree_path: &Path) -> GitResult<()> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Lists all worktrees (stub)
    pub fn list_worktrees(_repo_path: &Path) -> GitResult<Vec<WorktreeInfo>> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Checks if a worktree exists (stub)
    pub fn worktree_exists(_repo_path: &Path, _worktree_name: &str) -> GitResult<bool> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Creates a new branch (stub)
    pub fn create_branch(
        _repo_path: &Path,
        _branch_name: &str,
        _base_branch: &str,
    ) -> GitResult<()> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Deletes a local branch (stub)
    pub fn delete_branch(_repo_path: &Path, _branch_name: &str) -> GitResult<()> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Checks if a branch exists (stub)
    pub fn branch_exists(_repo_path: &Path, _branch_name: &str) -> GitResult<bool> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Lists all local branches (stub)
    pub fn list_branches(_repo_path: &Path) -> GitResult<Vec<BranchInfo>> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets the commit hash of a branch (stub)
    pub fn get_branch_commit_hash(_repo_path: &Path, _branch_name: &str) -> GitResult<String> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets the HEAD commit hash of a worktree (stub)
    pub fn get_worktree_head_commit(_worktree_path: &Path) -> GitResult<String> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Commits all changes (stub)
    pub fn commit_all(_repo_path: &Path, _message: &str) -> GitResult<CommitId> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Pushes a branch to the remote repository (stub)
    pub fn push_branch(
        _repo_path: &Path,
        _branch_name: &str,
        _remote_url: &str,
        _token: &str,
    ) -> GitResult<()> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Merges a branch into the current branch (stub)
    pub fn merge_branch(_repo_path: &Path, _branch_name: &str) -> GitResult<CommitId> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets the diff of uncommitted changes (stub)
    pub fn get_diff(_repo_path: &Path) -> GitResult<String> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets the diff between the current branch and the base branch (stub)
    pub fn get_diff_from_base(_worktree_path: &Path, _base_branch: &str) -> GitResult<String> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets the diff between two branches (stub)
    pub fn get_diff_between_branches(
        _repo_path: &Path,
        _head_branch: &str,
        _base_branch: &str,
    ) -> GitResult<String> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets the diff between two specific commit hashes (stub)
    pub fn get_diff_between_commits(
        _repo_path: &Path,
        _base_commit_hash: &str,
        _end_commit_hash: &str,
    ) -> GitResult<String> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets the diff between a specific commit and the current HEAD of a branch (stub)
    pub fn get_diff_from_commit(
        _repo_path: &Path,
        _base_commit_hash: &str,
        _head_branch: &str,
    ) -> GitResult<String> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets diff statistics (stub)
    pub fn get_diff_stats(_repo_path: &Path, _base_branch: &str) -> GitResult<DiffStats> {
        Err(GitError::NotSupportedOnMobile)
    }

    /// Gets a list of changed files (stub)
    pub fn get_changed_files(_repo_path: &Path, _base_branch: &str) -> GitResult<Vec<ChangedFile>> {
        Err(GitError::NotSupportedOnMobile)
    }
}

/// Repository information
#[derive(Debug, Clone)]
pub struct RepoInfo {
    /// Repository name
    pub name: String,
    /// Local path
    pub path: PathBuf,
    /// Current branch
    pub current_branch: String,
    /// Default branch
    pub default_branch: String,
    /// Remote URL (if any)
    pub remote_url: Option<String>,
}

/// Information about a worktree
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Worktree name
    pub name: String,
    /// Worktree path
    pub path: PathBuf,
    /// Whether the worktree is locked
    pub is_locked: bool,
    /// Whether the worktree is prunable
    pub is_prunable: bool,
}

/// Information about a branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this is the current HEAD
    pub is_head: bool,
    /// Upstream tracking branch (if any)
    pub upstream: Option<String>,
}

/// Diff statistics
#[derive(Debug, Clone)]
pub struct DiffStats {
    /// Number of files changed
    pub files_changed: usize,
    /// Number of insertions
    pub insertions: usize,
    /// Number of deletions
    pub deletions: usize,
}

/// File change status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    Unknown,
}

/// Information about a changed file
#[derive(Debug, Clone)]
pub struct ChangedFile {
    /// Old path (for renames/deletes)
    pub old_path: Option<String>,
    /// New path (for adds/renames)
    pub new_path: Option<String>,
    /// Change status
    pub status: FileStatus,
}

impl ChangedFile {
    /// Returns the primary path (new_path if available, otherwise old_path)
    pub fn path(&self) -> Option<&str> {
        self.new_path.as_deref().or(self.old_path.as_deref())
    }
}

/// Commit ID (stub for mobile - uses String instead of git2::Oid)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitId(String);

impl CommitId {
    /// Creates a new commit ID
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    /// Returns the commit ID as a string
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl std::fmt::Display for CommitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
