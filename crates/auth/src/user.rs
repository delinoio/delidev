//! User types for authentication

use serde::{Deserialize, Serialize};

/// An authenticated user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    /// User ID
    pub id: String,

    /// Email address (if available)
    pub email: Option<String>,

    /// Display name (if available)
    pub name: Option<String>,
}

impl AuthenticatedUser {
    /// Creates a new authenticated user
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            email: None,
            name: None,
        }
    }

    /// Sets the email
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Sets the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Returns the display name, falling back to email or ID
    pub fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .or(self.email.as_deref())
            .unwrap_or(&self.id)
    }
}

/// Role-based access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Regular user
    User,
    /// Administrator
    Admin,
    /// Worker server (internal)
    Worker,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::User
    }
}

impl UserRole {
    /// Checks if this role has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }

    /// Checks if this role is a worker
    pub fn is_worker(&self) -> bool {
        matches!(self, Self::Worker)
    }
}

/// User with role information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWithRole {
    /// The authenticated user
    #[serde(flatten)]
    pub user: AuthenticatedUser,

    /// User's role
    pub role: UserRole,
}

impl UserWithRole {
    /// Creates a new user with the default role
    pub fn new(user: AuthenticatedUser) -> Self {
        Self {
            user,
            role: UserRole::default(),
        }
    }

    /// Sets the role
    pub fn with_role(mut self, role: UserRole) -> Self {
        self.role = role;
        self
    }

    /// Checks if the user can perform admin actions
    pub fn can_admin(&self) -> bool {
        self.role.is_admin()
    }

    /// Checks if the user can execute tasks
    pub fn can_execute(&self) -> bool {
        !self.role.is_worker() // Workers can only report, not initiate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticated_user_display_name() {
        let user = AuthenticatedUser::new("user-123");
        assert_eq!(user.display_name(), "user-123");

        let user_with_email =
            AuthenticatedUser::new("user-123").with_email("test@example.com");
        assert_eq!(user_with_email.display_name(), "test@example.com");

        let user_with_name = AuthenticatedUser::new("user-123")
            .with_email("test@example.com")
            .with_name("Test User");
        assert_eq!(user_with_name.display_name(), "Test User");
    }

    #[test]
    fn test_user_role() {
        assert!(UserRole::Admin.is_admin());
        assert!(!UserRole::User.is_admin());
        assert!(UserRole::Worker.is_worker());
    }

    #[test]
    fn test_user_with_role() {
        let user = AuthenticatedUser::new("admin-1").with_name("Admin");
        let admin = UserWithRole::new(user).with_role(UserRole::Admin);

        assert!(admin.can_admin());
        assert!(admin.can_execute());

        let worker_user = AuthenticatedUser::new("worker-1");
        let worker = UserWithRole::new(worker_user).with_role(UserRole::Worker);

        assert!(!worker.can_admin());
        assert!(!worker.can_execute());
    }
}
