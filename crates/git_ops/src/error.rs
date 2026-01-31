//! Git operations error types.

use thiserror::Error;

/// Errors that can occur during git operations.
#[derive(Debug, Error)]
pub enum GitError {
    /// Repository not found.
    #[error("Repository not found: {0}")]
    NotFound(String),

    /// Clone failed.
    #[error("Clone failed: {0}")]
    CloneFailed(String),

    /// Fetch failed.
    #[error("Fetch failed: {0}")]
    FetchFailed(String),

    /// Branch not found.
    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    /// Branch already exists.
    #[error("Branch already exists: {0}")]
    BranchExists(String),

    /// Worktree error.
    #[error("Worktree error: {0}")]
    Worktree(String),

    /// Commit error.
    #[error("Commit error: {0}")]
    Commit(String),

    /// Invalid URL.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Authentication error.
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Underlying git2 error.
    #[error("Git error: {0}")]
    Git2(#[from] git2::Error),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

/// Result type for git operations.
pub type GitResult<T> = Result<T, GitError>;
