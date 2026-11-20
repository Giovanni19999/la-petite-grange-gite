//! Admin account management use case (F001-S08): creating an owner account.

use std::fmt;
use std::sync::Arc;

use crate::application::auth::{CreateUserError, PasswordHasher, UserRepository};
use crate::domain::auth::{Email, Password, UserId, WeakPassword, check_new_password};

/// Why an admin account could not be created.
#[derive(Debug, thiserror::Error)]
pub enum CreateAdminError {
    #[error("adresse email invalide")]
    InvalidEmail,
    #[error(transparent)]
    WeakPassword(#[from] WeakPassword),
    #[error("un compte existe déjà pour cet email")]
    AlreadyExists,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Creates owner accounts. Used only by the command-line adapter: there is no sign-up.
#[derive(Clone)]
pub struct AdminAccounts {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl fmt::Debug for AdminAccounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdminAccounts").finish_non_exhaustive()
    }
}

impl AdminAccounts {
    #[must_use]
    pub fn new(users: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { users, hasher }
    }

    /// Validates the email and password, hashes the password and stores the account.
    ///
    /// # Errors
    ///
    /// See [`CreateAdminError`].
    pub async fn create(
        &self,
        raw_email: &str,
        password: &Password,
    ) -> Result<UserId, CreateAdminError> {
        let email = Email::parse(raw_email).map_err(|_| CreateAdminError::InvalidEmail)?;
        check_new_password(password)?;
        let hash = self.hasher.hash(password).await?;
        self.users
            .create(&email, &hash)
            .await
            .map_err(|error| match error {
                CreateUserError::AlreadyExists => CreateAdminError::AlreadyExists,
                CreateUserError::Other(error) => CreateAdminError::Internal(error),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::auth::fakes::{InMemoryUsers, PlainHasher};

    fn accounts() -> (AdminAccounts, Arc<InMemoryUsers>) {
        let users = Arc::new(InMemoryUsers::default());
        (
            AdminAccounts::new(users.clone(), Arc::new(PlainHasher)),
            users,
        )
    }

    #[tokio::test]
    async fn f001_s08_create_admin_stores_only_the_hash() {
        let (accounts, users) = accounts();

        accounts
            .create(
                " Proprio@Example.test",
                &Password::new("correct horse battery"),
            )
            .await
            .unwrap();

        let email = Email::parse("proprio@example.test").unwrap();
        let stored = users.find_by_email(&email).await.unwrap().unwrap();
        assert_eq!(stored.password_hash, "plain:correct horse battery");
    }

    #[tokio::test]
    async fn f001_s08_create_admin_refuses_a_short_password() {
        let (accounts, _) = accounts();

        let result = accounts
            .create("proprio@example.test", &Password::new("too short"))
            .await;

        assert!(matches!(result, Err(CreateAdminError::WeakPassword(_))));
    }

    #[tokio::test]
    async fn f001_s08_create_admin_refuses_an_invalid_email() {
        let (accounts, _) = accounts();

        let result = accounts
            .create("not-an-email", &Password::new("correct horse battery"))
            .await;

        assert!(matches!(result, Err(CreateAdminError::InvalidEmail)));
    }

    #[tokio::test]
    async fn f001_s08_create_admin_refuses_a_duplicate_email() {
        let (accounts, _) = accounts();
        let password = Password::new("correct horse battery");
        accounts
            .create("proprio@example.test", &password)
            .await
            .unwrap();

        let result = accounts.create("PROPRIO@example.test", &password).await;

        assert!(matches!(result, Err(CreateAdminError::AlreadyExists)));
    }
}
