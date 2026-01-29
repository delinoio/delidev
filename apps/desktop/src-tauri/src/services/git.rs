use std::path::Path;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
    #[error("Worktree already exists: {0}")]
    WorktreeExists(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Push failed: {0}")]
    PushFailed(String),
    #[error("Merge conflict: {0}")]
    MergeConflict(String),
}

pub type GitResult<T> = Result<T, GitError>;

/// Service for Git operations including worktree management
pub struct GitService;

impl GitService {
    /// Creates a new GitService instance
    pub fn new() -> Self {
        Self
    }

    /// Opens a repository at the given path
    pub fn open_repo(&self, repo_path: &Path) -> GitResult<git2::Repository> {
        git2::Repository::open(repo_path).map_err(|e| {
            if e.code() == git2::ErrorCode::NotFound {
                GitError::RepositoryNotFound(repo_path.display().to_string())
            } else {
                GitError::Git(e)
            }
        })
    }

    /// Creates a new branch from the base branch
    pub fn create_branch(
        &self,
        repo_path: &Path,
        branch_name: &str,
        base_branch: &str,
    ) -> GitResult<()> {
        let repo = self.open_repo(repo_path)?;

        // Find the base branch
        let base_ref = repo
            .find_branch(base_branch, git2::BranchType::Local)
            .or_else(|_| {
                // Try to find as remote branch
                let remote_name = format!("origin/{}", base_branch);
                repo.find_branch(&remote_name, git2::BranchType::Remote)
            })
            .map_err(|_| GitError::BranchNotFound(base_branch.to_string()))?;

        let commit = base_ref.get().peel_to_commit()?;

        // Create new branch
        repo.branch(branch_name, &commit, false)?;

        Ok(())
    }

    /// Creates a new worktree for isolated task execution
    pub fn create_worktree(
        &self,
        repo_path: &Path,
        worktree_path: &Path,
        branch_name: &str,
        base_branch: &str,
    ) -> GitResult<()> {
        let repo = self.open_repo(repo_path)?;

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
            self.create_branch(repo_path, branch_name, base_branch)?;
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
    pub fn remove_worktree(&self, repo_path: &Path, worktree_path: &Path) -> GitResult<()> {
        let repo = self.open_repo(repo_path)?;

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

    /// Deletes a local branch
    pub fn delete_branch(&self, repo_path: &Path, branch_name: &str) -> GitResult<()> {
        let repo = self.open_repo(repo_path)?;

        // Find the branch
        let mut branch = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .map_err(|_| GitError::BranchNotFound(branch_name.to_string()))?;

        // Delete the branch
        branch.delete()?;

        Ok(())
    }

    /// Gets the current branch name
    pub fn current_branch(&self, repo_path: &Path) -> GitResult<String> {
        let repo = self.open_repo(repo_path)?;
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
    pub fn default_branch(&self, repo_path: &Path) -> GitResult<String> {
        let repo = self.open_repo(repo_path)?;

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

    /// Commits all changes in a worktree
    pub fn commit_all(&self, worktree_path: &Path, message: &str) -> GitResult<git2::Oid> {
        let repo = git2::Repository::open(worktree_path)?;

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

    /// Gets the diff of changes in a worktree
    pub fn get_diff(&self, worktree_path: &Path) -> GitResult<String> {
        let repo = git2::Repository::open(worktree_path)?;

        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;

        let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), None)?;

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

    /// Pushes a branch to the remote repository using a token for
    /// authentication. Uses git2's credential callbacks to avoid exposing
    /// tokens in process listings or logs.
    pub fn push_branch(
        &self,
        repo_path: &Path,
        branch_name: &str,
        remote_url: &str,
        token: &str,
    ) -> GitResult<()> {
        let repo = self.open_repo(repo_path)?;

        // Convert SSH URL to HTTPS if needed
        let https_url = if remote_url.starts_with("https://") {
            remote_url.to_string()
        } else if remote_url.starts_with("git@") {
            // Convert git@github.com:owner/repo.git to https://github.com/owner/repo.git
            let url_without_scheme = remote_url
                .strip_prefix("git@")
                .expect("URL starts with git@ as checked above")
                .replace(':', "/");
            format!("https://{}", url_without_scheme)
        } else {
            return Err(GitError::PushFailed(format!(
                "Unsupported remote URL format: {}",
                remote_url
            )));
        };

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
                // Sanitize error message to avoid leaking tokens
                let error_msg = e.message().to_string();
                let sanitized = Self::sanitize_error_message(&error_msg);
                GitError::PushFailed(sanitized)
            })?;

        Ok(())
    }

    /// Sanitizes error messages to remove potential token leaks
    fn sanitize_error_message(msg: &str) -> String {
        use std::sync::LazyLock;
        // Remove any URLs that might contain tokens
        static RE: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new(r"https://[^@]+@").expect("regex pattern is valid"));
        RE.replace_all(msg, "https://[REDACTED]@").to_string()
    }

    /// Gets the diff between the current branch and the base branch
    pub fn get_diff_from_base(&self, worktree_path: &Path, base_branch: &str) -> GitResult<String> {
        let repo = git2::Repository::open(worktree_path)?;

        // Get the base branch tree
        let base_tree = Self::find_branch_tree(&repo, base_branch)?;

        // Get current HEAD tree
        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;

        // Diff between base and HEAD
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
        Self::format_diff(&diff)
    }

    /// Gets the diff between two branches
    pub fn get_diff_between_branches(
        &self,
        repo_path: &Path,
        head_branch: &str,
        base_branch: &str,
    ) -> GitResult<String> {
        let repo = git2::Repository::open(repo_path)?;

        // Get the base and head branch trees
        let base_tree = Self::find_branch_tree(&repo, base_branch)?;
        let head_tree = Self::find_branch_tree(&repo, head_branch)?;

        // Diff between base and head
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
        Self::format_diff(&diff)
    }

    /// Gets the commit hash of a branch
    pub fn get_branch_commit_hash(&self, repo_path: &Path, branch_name: &str) -> GitResult<String> {
        let repo = git2::Repository::open(repo_path)?;

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
    pub fn get_worktree_head_commit(&self, worktree_path: &Path) -> GitResult<String> {
        let repo = git2::Repository::open(worktree_path)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    /// Gets the diff between two specific commit hashes
    pub fn get_diff_between_commits(
        &self,
        repo_path: &Path,
        base_commit_hash: &str,
        end_commit_hash: &str,
    ) -> GitResult<String> {
        let repo = git2::Repository::open(repo_path)?;

        // Parse the base commit
        let base_oid = git2::Oid::from_str(base_commit_hash).map_err(GitError::Git)?;
        let base_commit = repo
            .find_commit(base_oid)
            .map_err(|_| GitError::BranchNotFound(format!("commit {}", base_commit_hash)))?;
        let base_tree = base_commit.tree()?;

        // Parse the end commit
        let end_oid = git2::Oid::from_str(end_commit_hash).map_err(GitError::Git)?;
        let end_commit = repo
            .find_commit(end_oid)
            .map_err(|_| GitError::BranchNotFound(format!("commit {}", end_commit_hash)))?;
        let end_tree = end_commit.tree()?;

        // Diff between base and end commits
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&end_tree), None)?;
        Self::format_diff(&diff)
    }

    /// Gets the diff between a specific commit and the current HEAD of a branch
    pub fn get_diff_from_commit(
        &self,
        repo_path: &Path,
        base_commit_hash: &str,
        head_branch: &str,
    ) -> GitResult<String> {
        let repo = git2::Repository::open(repo_path)?;

        // Parse the base commit from hash
        let base_oid = git2::Oid::from_str(base_commit_hash).map_err(GitError::Git)?;
        let base_commit = repo
            .find_commit(base_oid)
            .map_err(|_| GitError::BranchNotFound(format!("commit {}", base_commit_hash)))?;
        let base_tree = base_commit.tree()?;

        // Get the head branch tree
        let head_tree = Self::find_branch_tree(&repo, head_branch)?;

        // Diff between base commit and head branch
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
        Self::format_diff(&diff)
    }

    /// Merges a branch into the current branch of the repository
    pub fn merge_branch(&self, repo_path: &Path, branch_name: &str) -> GitResult<git2::Oid> {
        let repo = self.open_repo(repo_path)?;

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
            // Already up to date, nothing to merge
            return Ok(branch_commit.id());
        }

        if merge_analysis.is_fast_forward() {
            // Fast-forward merge
            let current_branch = self.current_branch(repo_path)?;
            let refname = format!("refs/heads/{}", current_branch);
            let mut reference = repo.find_reference(&refname)?;
            reference.set_target(branch_commit.id(), "Fast-forward merge")?;
            repo.set_head(&refname)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            return Ok(branch_commit.id());
        }

        // Normal merge - create a merge commit
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

        let current_branch = self.current_branch(repo_path)?;
        let message = format!("Merge branch '{}' into {}", branch_name, current_branch);

        let merge_commit_id = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &message,
            &tree,
            &[&head_commit, &branch_commit],
        )?;

        // Cleanup merge state
        repo.cleanup_state()?;

        Ok(merge_commit_id)
    }

    /// Squash merges a branch into the current branch
    /// All commits from the branch are combined into a single new commit
    pub fn squash_merge_branch(&self, repo_path: &Path, branch_name: &str) -> GitResult<git2::Oid> {
        let repo = self.open_repo(repo_path)?;

        // Get the branch to squash merge
        let branch = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .map_err(|_| GitError::BranchNotFound(branch_name.to_string()))?;

        let branch_commit = branch.get().peel_to_commit()?;

        // Get current HEAD
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;

        // Find merge base
        let merge_base = repo.merge_base(head_commit.id(), branch_commit.id())?;

        // Check if already up to date
        if merge_base == branch_commit.id() {
            return Ok(head_commit.id());
        }

        // Perform the merge to get the combined tree
        let annotated_commit = repo.find_annotated_commit(branch_commit.id())?;
        repo.merge(&[&annotated_commit], None, None)?;

        // Check for conflicts
        let index = repo.index()?;
        if index.has_conflicts() {
            repo.cleanup_state()?;
            return Err(GitError::MergeConflict(
                "Merge conflict detected during squash merge. Please resolve manually.".to_string(),
            ));
        }

        // Collect commit messages from the branch for the squash commit message
        let mut commit_messages = Vec::new();
        let mut current = branch_commit.clone();
        while current.id() != merge_base {
            commit_messages.push(format!(
                "* {}",
                current.message().unwrap_or("(no message)").trim()
            ));
            if let Some(parent) = current.parents().next() {
                current = parent;
            } else {
                break;
            }
        }
        commit_messages.reverse();

        // Create the squash commit (single parent - HEAD only)
        let sig = repo.signature()?;
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let current_branch = self.current_branch(repo_path)?;
        let message = format!(
            "Squash merge branch '{}' into {}\n\n{}",
            branch_name,
            current_branch,
            commit_messages.join("\n")
        );

        let squash_commit_id =
            repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&head_commit])?;

        // Cleanup merge state
        repo.cleanup_state()?;

        Ok(squash_commit_id)
    }

    /// Rebases a branch onto the current branch
    /// Replays commits from the branch on top of the current HEAD
    pub fn rebase_merge_branch(&self, repo_path: &Path, branch_name: &str) -> GitResult<git2::Oid> {
        let repo = self.open_repo(repo_path)?;

        // Get the branch to rebase
        let branch = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .map_err(|_| GitError::BranchNotFound(branch_name.to_string()))?;

        let branch_commit = branch.get().peel_to_commit()?;

        // Get current HEAD
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;

        // Find merge base
        let merge_base = repo.merge_base(head_commit.id(), branch_commit.id())?;

        // Check if already up to date
        if merge_base == branch_commit.id() {
            return Ok(head_commit.id());
        }

        // Collect commits to replay (from merge base to branch head)
        let mut commits_to_replay = Vec::new();
        let mut current = branch_commit.clone();
        while current.id() != merge_base {
            commits_to_replay.push(current.clone());
            if let Some(parent) = current.parents().next() {
                current = parent;
            } else {
                break;
            }
        }
        commits_to_replay.reverse();

        // Replay each commit on top of HEAD
        let mut new_head = head_commit;
        for old_commit in commits_to_replay {
            // Cherry-pick the commit by applying its changes
            // Create cherrypick options
            let mut opts = git2::CherrypickOptions::new();
            repo.cherrypick(&old_commit, Some(&mut opts))?;

            // Check for conflicts
            let index = repo.index()?;
            if index.has_conflicts() {
                repo.cleanup_state()?;
                return Err(GitError::MergeConflict(format!(
                    "Conflict while rebasing commit '{}'. Please resolve manually.",
                    old_commit
                        .message()
                        .unwrap_or("(no message)")
                        .lines()
                        .next()
                        .unwrap_or("")
                )));
            }

            // Create the new commit with the same message
            let sig = repo.signature()?;
            let mut index = repo.index()?;
            let tree_id = index.write_tree()?;
            let tree = repo.find_tree(tree_id)?;

            let message = old_commit.message().unwrap_or("(no message)");
            let new_commit_id =
                repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&new_head])?;

            new_head = repo.find_commit(new_commit_id)?;

            // Cleanup cherrypick state
            repo.cleanup_state()?;
        }

        Ok(new_head.id())
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
}

/// Appends a short unique suffix to a branch name to prevent conflicts.
/// Takes the first 8 characters of a UUID to create a unique but readable
/// suffix. Example: "feature/add-login" -> "feature/add-login-a1b2c3d4"
pub fn make_branch_name_unique(branch_name: &str) -> String {
    let suffix = &uuid::Uuid::new_v4().to_string()[..8];
    format!("{}-{}", branch_name, suffix)
}

impl Default for GitService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_error_message_with_token() {
        let msg = "failed to push: https://x-access-token:secret123@github.com/owner/repo.git";
        let sanitized = GitService::sanitize_error_message(msg);
        assert!(!sanitized.contains("secret123"));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn test_sanitize_error_message_without_token() {
        let msg = "failed to push: network error";
        let sanitized = GitService::sanitize_error_message(msg);
        assert_eq!(sanitized, msg);
    }

    #[test]
    fn test_sanitize_error_message_multiple_urls() {
        let msg = "error: https://user:pass1@example.com and https://other:pass2@example.org";
        let sanitized = GitService::sanitize_error_message(msg);
        assert!(!sanitized.contains("pass1"));
        assert!(!sanitized.contains("pass2"));
        assert_eq!(
            sanitized,
            "error: https://[REDACTED]@example.com and https://[REDACTED]@example.org"
        );
    }
}
