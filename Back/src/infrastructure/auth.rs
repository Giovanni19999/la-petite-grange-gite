//! Adapters of the authentication ports: `MariaDB` repositories, Argon2id, clock, randomness.

use anyhow::Context;
use argon2::Argon2;
use argon2::password_hash::{PasswordHasher as _, PasswordVerifier as _};
use sqlx::MySqlPool;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::application::auth::{
    AuthenticatedUser, Clock, CreateUserError, PasswordHasher, RandomSource, SessionRepository,
    UserCredentials, UserRepository,
};
use crate::application::health::BoxFuture;
use crate::domain::auth::{
    Email, Password, SESSION_TOKEN_BYTES, Session, SessionTokenHash, UserId,
};

/// Timestamps are stored as UTC `DATETIME(3)` values.
fn to_db(value: OffsetDateTime) -> PrimitiveDateTime {
    let utc = value.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(utc.date(), utc.time())
}

fn from_db(value: PrimitiveDateTime) -> OffsetDateTime {
    value.assume_utc()
}

fn parse_stored_email(raw: &str) -> anyhow::Result<Email> {
    Email::parse(raw).with_context(|| "invalid email stored in the users table")
}

/// User accounts in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbUsers {
    pool: MySqlPool,
}

impl MariaDbUsers {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl UserRepository for MariaDbUsers {
    fn find_by_email<'a>(
        &'a self,
        email: &'a Email,
    ) -> BoxFuture<'a, anyhow::Result<Option<UserCredentials>>> {
        Box::pin(async move {
            let row = sqlx::query!(
                "SELECT id, email, password_hash FROM users WHERE email = ?",
                email.as_str()
            )
            .fetch_optional(&self.pool)
            .await
            .context("cannot read user")?;
            row.map(|row| {
                Ok(UserCredentials {
                    id: UserId(row.id),
                    email: parse_stored_email(&row.email)?,
                    password_hash: row.password_hash,
                })
            })
            .transpose()
        })
    }

    fn find_by_id(&self, id: UserId) -> BoxFuture<'_, anyhow::Result<Option<AuthenticatedUser>>> {
        Box::pin(async move {
            let row = sqlx::query!("SELECT id, email FROM users WHERE id = ?", id.0)
                .fetch_optional(&self.pool)
                .await
                .context("cannot read user")?;
            row.map(|row| {
                Ok(AuthenticatedUser {
                    id: UserId(row.id),
                    email: parse_stored_email(&row.email)?,
                })
            })
            .transpose()
        })
    }

    fn create<'a>(
        &'a self,
        email: &'a Email,
        password_hash: &'a str,
    ) -> BoxFuture<'a, Result<UserId, CreateUserError>> {
        Box::pin(async move {
            let result = sqlx::query!(
                "INSERT INTO users (email, password_hash) VALUES (?, ?)",
                email.as_str(),
                password_hash
            )
            .execute(&self.pool)
            .await;
            match result {
                Ok(done) => Ok(UserId(done.last_insert_id())),
                Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                    Err(CreateUserError::AlreadyExists)
                }
                Err(error) => Err(CreateUserError::Other(
                    anyhow::Error::new(error).context("cannot create user"),
                )),
            }
        })
    }
}

/// Sessions in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbSessions {
    pool: MySqlPool,
}

impl MariaDbSessions {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl SessionRepository for MariaDbSessions {
    fn insert<'a>(&'a self, session: &'a Session) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            sqlx::query!(
                "INSERT INTO sessions (token_hash, user_id, created_at, last_used_at, expires_at) \
                 VALUES (?, ?, ?, ?, ?)",
                session.token_hash.0,
                session.user_id.0,
                to_db(session.created_at),
                to_db(session.last_used_at),
                to_db(session.expires_at)
            )
            .execute(&self.pool)
            .await
            .context("cannot create session")?;
            Ok(())
        })
    }

    fn find<'a>(
        &'a self,
        token_hash: &'a SessionTokenHash,
    ) -> BoxFuture<'a, anyhow::Result<Option<Session>>> {
        Box::pin(async move {
            let row = sqlx::query!(
                "SELECT token_hash, user_id, created_at, last_used_at, expires_at \
                 FROM sessions WHERE token_hash = ?",
                token_hash.0
            )
            .fetch_optional(&self.pool)
            .await
            .context("cannot read session")?;
            Ok(row.map(|row| Session {
                user_id: UserId(row.user_id),
                token_hash: SessionTokenHash(row.token_hash),
                created_at: from_db(row.created_at),
                last_used_at: from_db(row.last_used_at),
                expires_at: from_db(row.expires_at),
            }))
        })
    }

    fn touch<'a>(
        &'a self,
        token_hash: &'a SessionTokenHash,
        last_used_at: OffsetDateTime,
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            sqlx::query!(
                "UPDATE sessions SET last_used_at = ? WHERE token_hash = ?",
                to_db(last_used_at),
                token_hash.0
            )
            .execute(&self.pool)
            .await
            .context("cannot refresh session")?;
            Ok(())
        })
    }

    fn delete<'a>(&'a self, token_hash: &'a SessionTokenHash) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM sessions WHERE token_hash = ?", token_hash.0)
                .execute(&self.pool)
                .await
                .context("cannot delete session")?;
            Ok(())
        })
    }
}

/// Argon2id password hashing (default parameters), run on the blocking thread pool.
#[derive(Debug, Clone)]
pub struct Argon2Hasher {
    dummy_hash: String,
}

impl Argon2Hasher {
    /// Builds the hasher and computes the dummy hash used for unknown emails.
    ///
    /// # Errors
    ///
    /// Returns an error if hashing fails.
    pub fn new() -> anyhow::Result<Self> {
        let mut random = [0_u8; 32];
        rand::fill(&mut random);
        let dummy_hash = hash_blocking(&random)?;
        Ok(Self { dummy_hash })
    }
}

fn hash_blocking(password: &[u8]) -> anyhow::Result<String> {
    Argon2::default()
        .hash_password(password)
        .map(|hash| hash.to_string())
        .map_err(|error| anyhow::anyhow!("cannot hash password: {error}"))
}

impl PasswordHasher for Argon2Hasher {
    fn hash<'a>(&'a self, password: &'a Password) -> BoxFuture<'a, anyhow::Result<String>> {
        let password = password.expose().as_bytes().to_vec();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || hash_blocking(&password))
                .await
                .context("hashing task failed")?
        })
    }

    fn verify<'a>(&'a self, password: &'a Password, hash: &'a str) -> BoxFuture<'a, bool> {
        let password = password.expose().as_bytes().to_vec();
        let hash = hash.to_owned();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                Argon2::default()
                    .verify_password(&password, hash.as_str())
                    .is_ok()
            })
            .await
            .unwrap_or(false)
        })
    }

    fn dummy_hash(&self) -> &str {
        &self.dummy_hash
    }
}

/// System clock (UTC).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// Operating-system backed cryptographically secure randomness.
#[derive(Debug, Clone, Copy, Default)]
pub struct OsRandom;

impl RandomSource for OsRandom {
    fn session_bytes(&self) -> [u8; SESSION_TOKEN_BYTES] {
        let mut bytes = [0_u8; SESSION_TOKEN_BYTES];
        rand::fill(&mut bytes);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn argon2_hash_is_argon2id_and_verifies() {
        let hasher = Argon2Hasher::new().unwrap();
        let password = Password::new("correct horse battery");

        let hash = hasher.hash(&password).await.unwrap();

        assert!(hash.starts_with("$argon2id$"));
        assert!(!hash.contains("correct horse battery"));
        assert!(hasher.verify(&password, &hash).await);
        assert!(!hasher.verify(&Password::new("wrong"), &hash).await);
        assert!(!hasher.verify(&password, hasher.dummy_hash()).await);
    }

    #[test]
    fn random_session_bytes_differ() {
        assert_ne!(OsRandom.session_bytes(), OsRandom.session_bytes());
    }
}
