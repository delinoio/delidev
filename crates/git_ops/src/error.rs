//! Git error types

use thiserror::Error;

/// Errors that can occur during git operations
#[derive(Error, Debug)]
pub enum GitError {
    /// Underlying git2 error
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

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
}

/// Result type for git operations
pub type GitResult<T> = Result<T, GitError>;
