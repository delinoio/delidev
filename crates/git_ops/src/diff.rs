//! Git diff operations

use std::path::Path;

use crate::{GitError, GitOperations, GitResult};

impl GitOperations {
    /// Gets the diff of uncommitted changes in a repository/worktree
    pub fn get_diff(repo_path: &Path) -> GitResult<String> {
        let repo = git2::Repository::open(repo_path)?;

        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;

        let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), None)?;

        format_diff(&diff)
    }

    /// Gets the diff between the current branch and the base branch
    pub fn get_diff_from_base(worktree_path: &Path, base_branch: &str) -> GitResult<String> {
        let repo = git2::Repository::open(worktree_path)?;

        // Get the base branch tree
        let base_tree = find_branch_tree(&repo, base_branch)?;

        // Get current HEAD tree
        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;

        // Diff between base and HEAD
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
        format_diff(&diff)
    }

    /// Gets the diff between two branches
    pub fn get_diff_between_branches(
        repo_path: &Path,
        head_branch: &str,
        base_branch: &str,
    ) -> GitResult<String> {
        let repo = git2::Repository::open(repo_path)?;

        // Get the base and head branch trees
        let base_tree = find_branch_tree(&repo, base_branch)?;
        let head_tree = find_branch_tree(&repo, head_branch)?;

        // Diff between base and head
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
        format_diff(&diff)
    }

    /// Gets the diff between two specific commit hashes
    pub fn get_diff_between_commits(
        repo_path: &Path,
        base_commit_hash: &str,
        end_commit_hash: &str,
    ) -> GitResult<String> {
        let repo = git2::Repository::open(repo_path)?;

        // Parse the base commit
        let base_oid = git2::Oid::from_str(base_commit_hash).map_err(GitError::Git)?;
        let base_commit = repo
            .find_commit(base_oid)
            .map_err(|_| GitError::CommitNotFound(base_commit_hash.to_string()))?;
        let base_tree = base_commit.tree()?;

        // Parse the end commit
        let end_oid = git2::Oid::from_str(end_commit_hash).map_err(GitError::Git)?;
        let end_commit = repo
            .find_commit(end_oid)
            .map_err(|_| GitError::CommitNotFound(end_commit_hash.to_string()))?;
        let end_tree = end_commit.tree()?;

        // Diff between base and end commits
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&end_tree), None)?;
        format_diff(&diff)
    }

    /// Gets the diff between a specific commit and the current HEAD of a branch
    pub fn get_diff_from_commit(
        repo_path: &Path,
        base_commit_hash: &str,
        head_branch: &str,
    ) -> GitResult<String> {
        let repo = git2::Repository::open(repo_path)?;

        // Parse the base commit from hash
        let base_oid = git2::Oid::from_str(base_commit_hash).map_err(GitError::Git)?;
        let base_commit = repo
            .find_commit(base_oid)
            .map_err(|_| GitError::CommitNotFound(base_commit_hash.to_string()))?;
        let base_tree = base_commit.tree()?;

        // Get the head branch tree
        let head_tree = find_branch_tree(&repo, head_branch)?;

        // Diff between base commit and head branch
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
        format_diff(&diff)
    }

    /// Gets diff statistics (files changed, insertions, deletions)
    pub fn get_diff_stats(repo_path: &Path, base_branch: &str) -> GitResult<DiffStats> {
        let repo = git2::Repository::open(repo_path)?;

        let base_tree = find_branch_tree(&repo, base_branch)?;
        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;

        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
        let stats = diff.stats()?;

        Ok(DiffStats {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        })
    }

    /// Gets a list of changed files
    pub fn get_changed_files(repo_path: &Path, base_branch: &str) -> GitResult<Vec<ChangedFile>> {
        let repo = git2::Repository::open(repo_path)?;

        let base_tree = find_branch_tree(&repo, base_branch)?;
        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;

        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;

        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                let status = match delta.status() {
                    git2::Delta::Added => FileStatus::Added,
                    git2::Delta::Deleted => FileStatus::Deleted,
                    git2::Delta::Modified => FileStatus::Modified,
                    git2::Delta::Renamed => FileStatus::Renamed,
                    git2::Delta::Copied => FileStatus::Copied,
                    _ => FileStatus::Unknown,
                };

                let old_path = delta
                    .old_file()
                    .path()
                    .map(|p| p.to_string_lossy().to_string());
                let new_path = delta
                    .new_file()
                    .path()
                    .map(|p| p.to_string_lossy().to_string());

                files.push(ChangedFile {
                    old_path,
                    new_path,
                    status,
                });

                true
            },
            None,
            None,
            None,
        )?;

        Ok(files)
    }
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

/// Finds a branch (local or remote) and returns its tree
fn find_branch_tree<'a>(
    repo: &'a git2::Repository,
    branch_name: &str,
) -> GitResult<git2::Tree<'a>> {
    let branch_ref = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .or_else(|_| {
            let remote_name = format!("origin/{}", branch_name);
            repo.find_branch(&remote_name, git2::BranchType::Remote)
        })
        .map_err(|_| GitError::BranchNotFound(branch_name.to_string()))?;

    branch_ref.get().peel_to_tree().map_err(GitError::Git)
}

/// Formats a diff as a patch string
fn format_diff(diff: &git2::Diff) -> GitResult<String> {
    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let content = std::str::from_utf8(line.content()).unwrap_or("");
        let prefix = match line.origin() {
            '+' => "+",
            '-' => "-",
            ' ' => " ",
            _ => "",
        };
        diff_text.push_str(prefix);
        diff_text.push_str(content);
        true
    })?;

    Ok(diff_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_changed_file_path() {
        let file = ChangedFile {
            old_path: Some("old/path.rs".to_string()),
            new_path: Some("new/path.rs".to_string()),
            status: FileStatus::Renamed,
        };
        assert_eq!(file.path(), Some("new/path.rs"));

        let deleted = ChangedFile {
            old_path: Some("deleted.rs".to_string()),
            new_path: None,
            status: FileStatus::Deleted,
        };
        assert_eq!(deleted.path(), Some("deleted.rs"));
    }
}
