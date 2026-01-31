//! Git branch operations

use std::path::Path;

use crate::{GitError, GitOperations, GitResult};

impl GitOperations {
    /// Creates a new branch from the base branch
    pub fn create_branch(repo_path: &Path, branch_name: &str, base_branch: &str) -> GitResult<()> {
        let repo = Self::open_repo(repo_path)?;

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

    /// Deletes a local branch
    pub fn delete_branch(repo_path: &Path, branch_name: &str) -> GitResult<()> {
        let repo = Self::open_repo(repo_path)?;

        // Find the branch
        let mut branch = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .map_err(|_| GitError::BranchNotFound(branch_name.to_string()))?;

        // Delete the branch
        branch.delete()?;

        Ok(())
    }

    /// Checks if a branch exists
    pub fn branch_exists(repo_path: &Path, branch_name: &str) -> GitResult<bool> {
        let repo = Self::open_repo(repo_path)?;
        let exists = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .is_ok();
        Ok(exists)
    }

    /// Lists all local branches
    pub fn list_branches(repo_path: &Path) -> GitResult<Vec<BranchInfo>> {
        let repo = Self::open_repo(repo_path)?;
        let branches = repo.branches(Some(git2::BranchType::Local))?;

        let mut result = Vec::new();
        for branch in branches.flatten() {
            let (branch, _) = branch;
            if let Some(name) = branch.name().ok().flatten() {
                let is_head = branch.is_head();
                let upstream = branch
                    .upstream()
                    .ok()
                    .and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));

                result.push(BranchInfo {
                    name: name.to_string(),
                    is_head,
                    upstream,
                });
            }
        }

        Ok(result)
    }

    /// Gets the commit hash of a branch
    pub fn get_branch_commit_hash(repo_path: &Path, branch_name: &str) -> GitResult<String> {
        let repo = Self::open_repo(repo_path)?;

        let branch = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .or_else(|_| {
                let remote_name = format!("origin/{}", branch_name);
                repo.find_branch(&remote_name, git2::BranchType::Remote)
            })
            .map_err(|_| GitError::BranchNotFound(branch_name.to_string()))?;

        let commit = branch.get().peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    /// Gets the HEAD commit hash of a worktree
    pub fn get_worktree_head_commit(worktree_path: &Path) -> GitResult<String> {
        let repo = git2::Repository::open(worktree_path)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    /// Commits all changes in a repository/worktree
    pub fn commit_all(repo_path: &Path, message: &str) -> GitResult<git2::Oid> {
        let repo = git2::Repository::open(repo_path)?;

        // Add all changes
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Create tree from index
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // Get parent commit
        let head = repo.head()?;
        let parent = head.peel_to_commit()?;

        // Create signature
        let sig = repo.signature()?;

        // Commit
        let commit_id = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;

        Ok(commit_id)
    }

    /// Pushes a branch to the remote repository
    pub fn push_branch(
        repo_path: &Path,
        branch_name: &str,
        remote_url: &str,
        token: &str,
    ) -> GitResult<()> {
        let repo = Self::open_repo(repo_path)?;

        // Convert SSH URL to HTTPS if needed
        let https_url = convert_to_https_url(remote_url)?;

        // Create a temporary remote for pushing
        let mut remote = repo
            .remote_anonymous(&https_url)
            .map_err(|e| GitError::PushFailed(format!("Failed to create remote: {}", e)))?;

        // Set up credential callbacks
        let token = token.to_string();
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(move |_url, _username, _allowed_types| {
            git2::Cred::userpass_plaintext("x-access-token", &token)
        });

        // Set up push options
        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);

        // Push the branch
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        remote
            .push(&[&refspec], Some(&mut push_options))
            .map_err(|e| {
                let error_msg = e.message().to_string();
                let sanitized = sanitize_error_message(&error_msg);
                GitError::PushFailed(sanitized)
            })?;

        Ok(())
    }

    /// Merges a branch into the current branch
    pub fn merge_branch(repo_path: &Path, branch_name: &str) -> GitResult<git2::Oid> {
        let repo = Self::open_repo(repo_path)?;

        // Get the branch to merge
        let branch = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .map_err(|_| GitError::BranchNotFound(branch_name.to_string()))?;

        let branch_commit = branch.get().peel_to_commit()?;

        // Get the annotated commit for merge
        let annotated_commit = repo.find_annotated_commit(branch_commit.id())?;

        // Perform merge analysis
        let (merge_analysis, _) = repo.merge_analysis(&[&annotated_commit])?;

        if merge_analysis.is_up_to_date() {
            return Ok(branch_commit.id());
        }

        if merge_analysis.is_fast_forward() {
            // Fast-forward merge
            let current_branch = Self::current_branch(repo_path)?;
            let refname = format!("refs/heads/{}", current_branch);
            let mut reference = repo.find_reference(&refname)?;
            reference.set_target(branch_commit.id(), "Fast-forward merge")?;
            repo.set_head(&refname)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            return Ok(branch_commit.id());
        }

        // Normal merge
        repo.merge(&[&annotated_commit], None, None)?;

        // Check for conflicts
        let index = repo.index()?;
        if index.has_conflicts() {
            repo.cleanup_state()?;
            return Err(GitError::MergeConflict(
                "Merge conflict detected. Please resolve manually.".to_string(),
            ));
        }

        // Create the merge commit
        let sig = repo.signature()?;
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;

        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let current_branch = Self::current_branch(repo_path)?;
        let message = format!("Merge branch '{}' into {}", branch_name, current_branch);

        let merge_commit_id = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &message,
            &tree,
            &[&head_commit, &branch_commit],
        )?;

        repo.cleanup_state()?;

        Ok(merge_commit_id)
    }
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

/// Converts SSH URL to HTTPS URL
fn convert_to_https_url(remote_url: &str) -> GitResult<String> {
    if remote_url.starts_with("https://") {
        Ok(remote_url.to_string())
    } else if remote_url.starts_with("git@") {
        // Convert git@github.com:owner/repo.git to https://github.com/owner/repo.git
        let url_without_scheme = remote_url
            .strip_prefix("git@")
            .expect("URL starts with git@ as checked above")
            .replace(':', "/");
        Ok(format!("https://{}", url_without_scheme))
    } else {
        Err(GitError::PushFailed(format!(
            "Unsupported remote URL format: {}",
            remote_url
        )))
    }
}

/// Sanitizes error messages to remove potential token leaks
fn sanitize_error_message(msg: &str) -> String {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"https://[^@]+@").expect("regex pattern is valid"));
    RE.replace_all(msg, "https://[REDACTED]@").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_ssh_to_https() {
        let ssh_url = "git@github.com:owner/repo.git";
        let https_url = convert_to_https_url(ssh_url).unwrap();
        assert_eq!(https_url, "https://github.com/owner/repo.git");
    }

    #[test]
    fn test_convert_https_passthrough() {
        let https_url = "https://github.com/owner/repo.git";
        let result = convert_to_https_url(https_url).unwrap();
        assert_eq!(result, https_url);
    }

    #[test]
    fn test_sanitize_error_message() {
        let msg = "failed to push: https://x-access-token:secret123@github.com/owner/repo.git";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("secret123"));
        assert!(sanitized.contains("[REDACTED]"));
    }
}
