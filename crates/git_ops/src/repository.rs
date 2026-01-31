//! Repository operations.

use std::path::Path;

use git2::{BranchType, Cred, FetchOptions, RemoteCallbacks, Repository as Git2Repository};
use tracing::info;

use crate::{GitError, GitResult};

/// Credentials for git operations.
#[derive(Debug, Clone)]
pub enum GitCredentials {
    /// Username and password/token.
    UserPass { username: String, password: String },
    /// SSH key.
    SshKey {
        username: String,
        public_key: Option<String>,
        private_key: String,
        passphrase: Option<String>,
    },
    /// Default credentials (SSH agent or git config).
    Default,
}

/// Options for cloning a repository.
#[derive(Debug, Clone)]
pub struct CloneOptions {
    /// Branch to checkout.
    pub branch: Option<String>,
    /// Credentials for authentication.
    pub credentials: Option<GitCredentials>,
    /// Whether to do a bare clone.
    pub bare: bool,
    /// Clone depth (None for full clone).
    pub depth: Option<u32>,
}

impl Default for CloneOptions {
    fn default() -> Self {
        Self {
            branch: None,
            credentials: None,
            bare: false,
            depth: None,
        }
    }
}

/// Options for fetching from a remote.
#[derive(Debug, Clone, Default)]
pub struct FetchOpts {
    /// Credentials for authentication.
    pub credentials: Option<GitCredentials>,
    /// Prune deleted branches.
    pub prune: bool,
}

/// A wrapper around git2::Repository with additional functionality.
pub struct GitRepository {
    repo: Git2Repository,
}

impl GitRepository {
    /// Opens an existing repository.
    pub fn open(path: impl AsRef<Path>) -> GitResult<Self> {
        let repo = Git2Repository::open(path)?;
        Ok(Self { repo })
    }

    /// Opens a repository, searching upward from the given path.
    pub fn discover(path: impl AsRef<Path>) -> GitResult<Self> {
        let repo = Git2Repository::discover(path)?;
        Ok(Self { repo })
    }

    /// Initializes a new repository.
    pub fn init(path: impl AsRef<Path>) -> GitResult<Self> {
        let repo = Git2Repository::init(path)?;
        Ok(Self { repo })
    }

    /// Clones a repository.
    pub fn clone(url: &str, path: impl AsRef<Path>, options: CloneOptions) -> GitResult<Self> {
        let path = path.as_ref();
        info!("Cloning {} to {:?}", url, path);

        let mut builder = git2::build::RepoBuilder::new();

        if let Some(branch) = &options.branch {
            builder.branch(branch);
        }

        if options.bare {
            builder.bare(true);
        }

        let mut fetch_options = FetchOptions::new();

        if let Some(creds) = &options.credentials {
            let callbacks = create_callbacks(creds.clone());
            fetch_options.remote_callbacks(callbacks);
        }

        builder.fetch_options(fetch_options);

        let repo = builder
            .clone(url, path)
            .map_err(|e| GitError::CloneFailed(e.to_string()))?;

        Ok(Self { repo })
    }

    /// Returns the path to the repository.
    pub fn path(&self) -> &Path {
        self.repo.path()
    }

    /// Returns the workdir path (None for bare repos).
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Fetches from a remote.
    pub fn fetch(&self, remote_name: &str, options: FetchOpts) -> GitResult<()> {
        let mut remote = self.repo.find_remote(remote_name)?;

        let mut fetch_options = FetchOptions::new();

        if let Some(creds) = options.credentials {
            let callbacks = create_callbacks(creds);
            fetch_options.remote_callbacks(callbacks);
        }

        if options.prune {
            fetch_options.prune(git2::FetchPrune::On);
        }

        remote
            .fetch(&[] as &[&str], Some(&mut fetch_options), None)
            .map_err(|e| GitError::FetchFailed(e.to_string()))?;

        Ok(())
    }

    /// Creates a new branch.
    pub fn create_branch(&self, name: &str, target: Option<&str>) -> GitResult<()> {
        let commit = if let Some(target) = target {
            self.repo.revparse_single(target)?.peel_to_commit()?
        } else {
            self.repo.head()?.peel_to_commit()?
        };

        self.repo.branch(name, &commit, false).map_err(|e| {
            if e.code() == git2::ErrorCode::Exists {
                GitError::BranchExists(name.to_string())
            } else {
                GitError::Git2(e)
            }
        })?;

        Ok(())
    }

    /// Checks out a branch.
    pub fn checkout_branch(&self, name: &str) -> GitResult<()> {
        let (object, reference) = self.repo.revparse_ext(name)?;

        self.repo.checkout_tree(&object, None)?;

        match reference {
            Some(gref) => self.repo.set_head(gref.name().unwrap())?,
            None => self.repo.set_head_detached(object.id())?,
        }

        Ok(())
    }

    /// Deletes a branch.
    pub fn delete_branch(&self, name: &str, force: bool) -> GitResult<()> {
        let mut branch = self.repo.find_branch(name, BranchType::Local)?;

        if force {
            branch.delete()?;
        } else {
            // Check if branch is merged before deleting
            let head = self.repo.head()?;
            let head_commit = head.peel_to_commit()?;
            let branch_commit = branch.get().peel_to_commit()?;

            if self.repo.merge_base(head_commit.id(), branch_commit.id())? == branch_commit.id() {
                branch.delete()?;
            } else {
                return Err(GitError::Other(format!(
                    "Branch '{}' is not fully merged. Use force to delete.",
                    name
                )));
            }
        }

        Ok(())
    }

    /// Lists all local branches.
    pub fn list_branches(&self) -> GitResult<Vec<String>> {
        let mut branches = Vec::new();

        for branch in self.repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                branches.push(name.to_string());
            }
        }

        Ok(branches)
    }

    /// Returns the current branch name.
    pub fn current_branch(&self) -> GitResult<Option<String>> {
        let head = self.repo.head()?;
        Ok(head.shorthand().map(|s| s.to_string()))
    }

    /// Returns the default branch name (usually main or master).
    pub fn default_branch(&self) -> GitResult<String> {
        // Try to get from origin/HEAD
        if let Ok(reference) = self.repo.find_reference("refs/remotes/origin/HEAD")
            && let Ok(resolved) = reference.resolve()
            && let Some(name) = resolved.shorthand()
        {
            return Ok(name.trim_start_matches("origin/").to_string());
        }

        // Fallback: try main, then master
        if self.repo.find_branch("main", BranchType::Local).is_ok() {
            return Ok("main".to_string());
        }

        if self.repo.find_branch("master", BranchType::Local).is_ok() {
            return Ok("master".to_string());
        }

        // Last resort: return current branch
        self.current_branch()?
            .ok_or_else(|| GitError::Other("Could not determine default branch".to_string()))
    }

    /// Returns the underlying git2 repository.
    pub fn inner(&self) -> &Git2Repository {
        &self.repo
    }
}

/// Creates remote callbacks with the given credentials.
fn create_callbacks(credentials: GitCredentials) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(move |_url, username_from_url, allowed_types| {
        match &credentials {
            GitCredentials::UserPass { username, password } => {
                Cred::userpass_plaintext(username, password)
            }
            GitCredentials::SshKey {
                username,
                public_key,
                private_key,
                passphrase,
            } => {
                let public_key_path = public_key.as_ref().map(Path::new);
                Cred::ssh_key(
                    username,
                    public_key_path,
                    Path::new(private_key),
                    passphrase.as_deref(),
                )
            }
            GitCredentials::Default => {
                // Try SSH agent first
                if allowed_types.contains(git2::CredentialType::SSH_KEY)
                    && let Some(username) = username_from_url
                {
                    return Cred::ssh_key_from_agent(username);
                }

                // Try default credentials
                Cred::default()
            }
        }
    });

    callbacks
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_init_and_open() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test-repo");

        // Initialize
        let repo = GitRepository::init(&path).unwrap();
        assert!(repo.workdir().is_some());

        // Open
        let repo2 = GitRepository::open(&path).unwrap();
        assert!(repo2.workdir().is_some());
    }

    #[test]
    fn test_branch_operations() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test-repo");

        let repo = GitRepository::init(&path).unwrap();

        // Create initial commit
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.inner().index().unwrap();
            fs::write(path.join("README.md"), "# Test").unwrap();
            index.add_path(Path::new("README.md")).unwrap();
            index.write().unwrap(); // Write the index to disk
            index.write_tree().unwrap()
        };
        let tree = repo.inner().find_tree(tree_id).unwrap();
        repo.inner()
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Checkout HEAD to reset the working directory state
        repo.inner()
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();

        // Test branch creation
        repo.create_branch("feature", None).unwrap();
        let branches = repo.list_branches().unwrap();
        assert!(branches.contains(&"feature".to_string()));

        // Test checkout
        repo.checkout_branch("feature").unwrap();
        assert_eq!(repo.current_branch().unwrap(), Some("feature".to_string()));
    }
}
