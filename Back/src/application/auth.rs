//! Owner authentication use cases (F001, ADR-0011) and their ports.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use time::{Duration, OffsetDateTime};

use crate::application::health::BoxFuture;
use crate::domain::auth::{
    Email, Password, SESSION_TOKEN_BYTES, Session, SessionPolicy, SessionToken, SessionTokenHash,
    UserId, WeakPassword, check_new_password,
};

/// Stored credentials of a user.
#[derive(Debug, Clone)]
pub struct UserCredentials {
    pub id: UserId,
    pub email: Email,
    pub password_hash: String,
}

/// The user behind a valid session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedUser {
    pub id: UserId,
    pub email: Email,
}

/// Error of a repository when the email is already taken.
#[derive(Debug, thiserror::Error)]
pub enum CreateUserError {
    #[error("an account already exists for this email")]
    AlreadyExists,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Port: user accounts.
pub trait UserRepository: Send + Sync {
    fn find_by_email<'a>(
        &'a self,
        email: &'a Email,
    ) -> BoxFuture<'a, anyhow::Result<Option<UserCredentials>>>;
    fn find_by_id(&self, id: UserId) -> BoxFuture<'_, anyhow::Result<Option<AuthenticatedUser>>>;
    fn create<'a>(
        &'a self,
        email: &'a Email,
        password_hash: &'a str,
    ) -> BoxFuture<'a, Result<UserId, CreateUserError>>;
}

/// Port: sessions, looked up by token hash.
pub trait SessionRepository: Send + Sync {
    fn insert<'a>(&'a self, session: &'a Session) -> BoxFuture<'a, anyhow::Result<()>>;
    fn find<'a>(
        &'a self,
        token_hash: &'a SessionTokenHash,
    ) -> BoxFuture<'a, anyhow::Result<Option<Session>>>;
    fn touch<'a>(
        &'a self,
        token_hash: &'a SessionTokenHash,
        last_used_at: OffsetDateTime,
    ) -> BoxFuture<'a, anyhow::Result<()>>;
    fn delete<'a>(&'a self, token_hash: &'a SessionTokenHash) -> BoxFuture<'a, anyhow::Result<()>>;
}

/// Port: password hashing (Argon2id in production).
pub trait PasswordHasher: Send + Sync {
    fn hash<'a>(&'a self, password: &'a Password) -> BoxFuture<'a, anyhow::Result<String>>;
    /// Returns `true` when `password` matches `hash`.
    fn verify<'a>(&'a self, password: &'a Password, hash: &'a str) -> BoxFuture<'a, bool>;
    /// A valid hash of an unknown password, verified when the email does not exist so
    /// that both cases take the same time.
    fn dummy_hash(&self) -> &str;
}

/// Port: current time.
pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

/// Port: cryptographically secure random bytes.
pub trait RandomSource: Send + Sync {
    fn session_bytes(&self) -> [u8; SESSION_TOKEN_BYTES];
}

/// Limits on failed login attempts (technical security settings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginThrottle {
    pub max_failures: usize,
    pub window: Duration,
}

/// Counts recent failed logins per key (email or client IP), in memory.
#[derive(Debug)]
struct FailedLogins {
    throttle: LoginThrottle,
    failures: Mutex<HashMap<String, VecDeque<OffsetDateTime>>>,
}

impl FailedLogins {
    fn new(throttle: LoginThrottle) -> Self {
        Self {
            throttle,
            failures: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, VecDeque<OffsetDateTime>>> {
        // A poisoned lock only means another request panicked; the counters stay usable.
        self.failures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn is_blocked(&self, keys: &[String], now: OffsetDateTime) -> bool {
        let mut failures = self.lock();
        keys.iter().any(|key| {
            failures.get_mut(key).is_some_and(|times| {
                times.retain(|time| now - *time < self.throttle.window);
                times.len() >= self.throttle.max_failures
            })
        })
    }

    fn record(&self, keys: &[String], now: OffsetDateTime) {
        let mut failures = self.lock();
        for key in keys {
            failures.entry(key.clone()).or_default().push_back(now);
        }
    }

    fn clear(&self, key: &str) {
        self.lock().remove(key);
    }
}

/// Why a login failed.
#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("invalid email or password")]
    InvalidCredentials,
    #[error("too many failed attempts")]
    TooManyAttempts,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Why an admin account could not be created.
#[derive(Debug, thiserror::Error)]
pub enum CreateAdminError {
    #[error("invalid email address")]
    InvalidEmail,
    #[error(transparent)]
    WeakPassword(#[from] WeakPassword),
    #[error("an account already exists for this email")]
    AlreadyExists,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Ports used by the authentication service.
#[derive(Clone)]
#[allow(missing_debug_implementations)] // trait objects without Debug
pub struct AuthPorts {
    pub users: Arc<dyn UserRepository>,
    pub sessions: Arc<dyn SessionRepository>,
    pub hasher: Arc<dyn PasswordHasher>,
    pub clock: Arc<dyn Clock>,
    pub random: Arc<dyn RandomSource>,
}

/// Authentication use cases.
pub struct AuthService {
    ports: AuthPorts,
    policy: SessionPolicy,
    failed_logins: FailedLogins,
}

impl fmt::Debug for AuthService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthService")
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

impl AuthService {
    #[must_use]
    pub fn new(ports: AuthPorts, policy: SessionPolicy, throttle: LoginThrottle) -> Self {
        Self {
            ports,
            policy,
            failed_logins: FailedLogins::new(throttle),
        }
    }

    #[must_use]
    pub fn policy(&self) -> SessionPolicy {
        self.policy
    }

    /// Checks the credentials and opens a session.
    ///
    /// # Errors
    ///
    /// [`LoginError::InvalidCredentials`] for a wrong email or password (same answer for
    /// both), [`LoginError::TooManyAttempts`] when the email or the client IP has too many
    /// recent failures, [`LoginError::Internal`] on infrastructure errors.
    pub async fn login(
        &self,
        raw_email: &str,
        password: &Password,
        client_ip: IpAddr,
    ) -> Result<(SessionToken, AuthenticatedUser), LoginError> {
        let now = self.ports.clock.now();
        let email_key = format!("email:{}", raw_email.trim().to_lowercase());
        let keys = [email_key.clone(), format!("ip:{client_ip}")];
        if self.failed_logins.is_blocked(&keys, now) {
            return Err(LoginError::TooManyAttempts);
        }

        let user = match Email::parse(raw_email) {
            Ok(email) => self.ports.users.find_by_email(&email).await?,
            Err(_) => None,
        };
        let hash = user
            .as_ref()
            .map_or(self.ports.hasher.dummy_hash(), |user| {
                user.password_hash.as_str()
            });
        let password_ok = self.ports.hasher.verify(password, hash).await;

        let Some(user) = user.filter(|_| password_ok) else {
            self.failed_logins.record(&keys, now);
            return Err(LoginError::InvalidCredentials);
        };
        self.failed_logins.clear(&email_key);

        let token = SessionToken::from_random_bytes(&self.ports.random.session_bytes());
        let session = Session::open(user.id, &token, now, self.policy);
        self.ports.sessions.insert(&session).await?;

        Ok((
            token,
            AuthenticatedUser {
                id: user.id,
                email: user.email,
            },
        ))
    }

    /// Returns the user of an active session and refreshes its idle timer.
    /// An expired session is deleted.
    ///
    /// # Errors
    ///
    /// Returns an error on infrastructure failures.
    pub async fn current_user(
        &self,
        token: &SessionToken,
    ) -> anyhow::Result<Option<AuthenticatedUser>> {
        let token_hash = token.hash();
        let Some(session) = self.ports.sessions.find(&token_hash).await? else {
            return Ok(None);
        };
        let now = self.ports.clock.now();
        if !session.is_active(now, self.policy) {
            self.ports.sessions.delete(&token_hash).await?;
            return Ok(None);
        }
        self.ports.sessions.touch(&token_hash, now).await?;
        self.ports.users.find_by_id(session.user_id).await
    }

    /// Ends a session.
    ///
    /// # Errors
    ///
    /// Returns an error on infrastructure failures.
    pub async fn logout(&self, token: &SessionToken) -> anyhow::Result<()> {
        self.ports.sessions.delete(&token.hash()).await
    }

    /// Creates an admin account (command-line tool only).
    ///
    /// # Errors
    ///
    /// See [`CreateAdminError`].
    pub async fn create_admin(
        &self,
        raw_email: &str,
        password: &Password,
    ) -> Result<UserId, CreateAdminError> {
        let email = Email::parse(raw_email).map_err(|_| CreateAdminError::InvalidEmail)?;
        check_new_password(password)?;
        let hash = self.ports.hasher.hash(password).await?;
        self.ports
            .users
            .create(&email, &hash)
            .await
            .map_err(|error| match error {
                CreateUserError::AlreadyExists => CreateAdminError::AlreadyExists,
                CreateUserError::Other(error) => CreateAdminError::Internal(error),
            })
    }
}

#[cfg(test)]
#[allow(clippy::missing_panics_doc, clippy::unnecessary_literal_bound)] // test doubles
pub mod fakes {
    //! In-memory implementations of the ports, for tests.

    use super::*;

    #[derive(Debug, Default)]
    pub struct InMemoryUsers {
        users: Mutex<Vec<UserCredentials>>,
    }

    impl UserRepository for InMemoryUsers {
        fn find_by_email<'a>(
            &'a self,
            email: &'a Email,
        ) -> BoxFuture<'a, anyhow::Result<Option<UserCredentials>>> {
            let found = self
                .users
                .lock()
                .unwrap()
                .iter()
                .find(|user| &user.email == email)
                .cloned();
            Box::pin(async move { Ok(found) })
        }

        fn find_by_id(
            &self,
            id: UserId,
        ) -> BoxFuture<'_, anyhow::Result<Option<AuthenticatedUser>>> {
            let found = self
                .users
                .lock()
                .unwrap()
                .iter()
                .find(|user| user.id == id)
                .map(|user| AuthenticatedUser {
                    id: user.id,
                    email: user.email.clone(),
                });
            Box::pin(async move { Ok(found) })
        }

        fn create<'a>(
            &'a self,
            email: &'a Email,
            password_hash: &'a str,
        ) -> BoxFuture<'a, Result<UserId, CreateUserError>> {
            let mut users = self.users.lock().unwrap();
            let result = if users.iter().any(|user| &user.email == email) {
                Err(CreateUserError::AlreadyExists)
            } else {
                let id = UserId(users.len() as u64 + 1);
                users.push(UserCredentials {
                    id,
                    email: email.clone(),
                    password_hash: password_hash.to_owned(),
                });
                Ok(id)
            };
            Box::pin(async move { result })
        }
    }

    #[derive(Debug, Default)]
    pub struct InMemorySessions {
        pub sessions: Mutex<Vec<Session>>,
    }

    impl SessionRepository for InMemorySessions {
        fn insert<'a>(&'a self, session: &'a Session) -> BoxFuture<'a, anyhow::Result<()>> {
            self.sessions.lock().unwrap().push(session.clone());
            Box::pin(async { Ok(()) })
        }

        fn find<'a>(
            &'a self,
            token_hash: &'a SessionTokenHash,
        ) -> BoxFuture<'a, anyhow::Result<Option<Session>>> {
            let found = self
                .sessions
                .lock()
                .unwrap()
                .iter()
                .find(|session| &session.token_hash == token_hash)
                .cloned();
            Box::pin(async move { Ok(found) })
        }

        fn touch<'a>(
            &'a self,
            token_hash: &'a SessionTokenHash,
            last_used_at: OffsetDateTime,
        ) -> BoxFuture<'a, anyhow::Result<()>> {
            for session in self.sessions.lock().unwrap().iter_mut() {
                if &session.token_hash == token_hash {
                    session.last_used_at = last_used_at;
                }
            }
            Box::pin(async { Ok(()) })
        }

        fn delete<'a>(
            &'a self,
            token_hash: &'a SessionTokenHash,
        ) -> BoxFuture<'a, anyhow::Result<()>> {
            self.sessions
                .lock()
                .unwrap()
                .retain(|session| &session.token_hash != token_hash);
            Box::pin(async { Ok(()) })
        }
    }

    /// Stores passwords as `plain:<password>`: fast and deterministic, tests only.
    #[derive(Debug)]
    pub struct PlainHasher;

    impl PasswordHasher for PlainHasher {
        fn hash<'a>(&'a self, password: &'a Password) -> BoxFuture<'a, anyhow::Result<String>> {
            let hash = format!("plain:{}", password.expose());
            Box::pin(async move { Ok(hash) })
        }

        fn verify<'a>(&'a self, password: &'a Password, hash: &'a str) -> BoxFuture<'a, bool> {
            let ok = hash == format!("plain:{}", password.expose());
            Box::pin(async move { ok })
        }

        fn dummy_hash(&self) -> &str {
            "plain:\u{0}never-matches"
        }
    }

    #[derive(Debug)]
    pub struct FixedClock(pub Mutex<OffsetDateTime>);

    impl FixedClock {
        pub fn advance(&self, by: Duration) {
            *self.0.lock().unwrap() += by;
        }
    }

    impl Clock for FixedClock {
        fn now(&self) -> OffsetDateTime {
            *self.0.lock().unwrap()
        }
    }

    #[derive(Debug, Default)]
    pub struct CountingRandom(Mutex<u8>);

    impl RandomSource for CountingRandom {
        fn session_bytes(&self) -> [u8; SESSION_TOKEN_BYTES] {
            let mut counter = self.0.lock().unwrap();
            *counter = counter.wrapping_add(1);
            [*counter; SESSION_TOKEN_BYTES]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fakes::*;
    use super::*;
    use std::net::Ipv4Addr;
    use time::macros::datetime;

    const IP: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
    const OTHER_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));

    struct Fixture {
        service: AuthService,
        clock: Arc<FixedClock>,
        sessions: Arc<InMemorySessions>,
    }

    async fn fixture() -> Fixture {
        let clock = Arc::new(FixedClock(Mutex::new(datetime!(2026-11-20 10:00 UTC))));
        let sessions = Arc::new(InMemorySessions::default());
        let service = AuthService::new(
            AuthPorts {
                users: Arc::new(InMemoryUsers::default()),
                sessions: sessions.clone(),
                hasher: Arc::new(PlainHasher),
                clock: clock.clone(),
                random: Arc::new(CountingRandom::default()),
            },
            SessionPolicy {
                max_age: Duration::days(30),
                idle_timeout: Duration::days(7),
            },
            LoginThrottle {
                max_failures: 5,
                window: Duration::minutes(15),
            },
        );
        service
            .create_admin(
                "proprio@example.test",
                &Password::new("correct horse battery"),
            )
            .await
            .unwrap();
        Fixture {
            service,
            clock,
            sessions,
        }
    }

    #[tokio::test]
    async fn login_opens_a_session_storing_only_the_token_hash() {
        let f = fixture().await;

        let (token, user) = f
            .service
            .login(
                " PROPRIO@example.test",
                &Password::new("correct horse battery"),
                IP,
            )
            .await
            .unwrap();

        assert_eq!(user.email.as_str(), "proprio@example.test");
        let stored = f.sessions.sessions.lock().unwrap()[0].clone();
        assert_eq!(stored.token_hash, token.hash());
        assert_ne!(stored.token_hash.0, token.expose());
        assert_eq!(f.service.current_user(&token).await.unwrap(), Some(user));
    }

    #[tokio::test]
    async fn wrong_password_and_unknown_email_give_the_same_error() {
        let f = fixture().await;

        let wrong = f
            .service
            .login(
                "proprio@example.test",
                &Password::new("wrong password!"),
                IP,
            )
            .await;
        let unknown = f
            .service
            .login("nobody@example.test", &Password::new("whatever pass"), IP)
            .await;

        assert!(matches!(wrong, Err(LoginError::InvalidCredentials)));
        assert!(matches!(unknown, Err(LoginError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn five_failures_block_the_email_even_with_the_right_password() {
        let f = fixture().await;
        for _ in 0..5 {
            let _ = f
                .service
                .login("proprio@example.test", &Password::new("wrong"), OTHER_IP)
                .await;
        }

        let result = f
            .service
            .login(
                "proprio@example.test",
                &Password::new("correct horse battery"),
                IP,
            )
            .await;

        assert!(matches!(result, Err(LoginError::TooManyAttempts)));
    }

    #[tokio::test]
    async fn the_block_ends_after_the_window() {
        let f = fixture().await;
        for _ in 0..5 {
            let _ = f
                .service
                .login("proprio@example.test", &Password::new("wrong"), IP)
                .await;
        }
        f.clock.advance(Duration::minutes(15));

        let result = f
            .service
            .login(
                "proprio@example.test",
                &Password::new("correct horse battery"),
                IP,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn logout_ends_the_session() {
        let f = fixture().await;
        let (token, _) = f
            .service
            .login(
                "proprio@example.test",
                &Password::new("correct horse battery"),
                IP,
            )
            .await
            .unwrap();

        f.service.logout(&token).await.unwrap();

        assert_eq!(f.service.current_user(&token).await.unwrap(), None);
    }

    #[tokio::test]
    async fn idle_session_is_rejected_and_deleted() {
        let f = fixture().await;
        let (token, _) = f
            .service
            .login(
                "proprio@example.test",
                &Password::new("correct horse battery"),
                IP,
            )
            .await
            .unwrap();
        f.clock.advance(Duration::days(7));

        assert_eq!(f.service.current_user(&token).await.unwrap(), None);
        assert!(f.sessions.sessions.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn using_a_session_keeps_it_alive() {
        let f = fixture().await;
        let (token, _) = f
            .service
            .login(
                "proprio@example.test",
                &Password::new("correct horse battery"),
                IP,
            )
            .await
            .unwrap();
        for _ in 0..4 {
            f.clock.advance(Duration::days(6));
            assert!(f.service.current_user(&token).await.unwrap().is_some());
        }
    }

    #[tokio::test]
    async fn f001_s08_create_admin_hashes_the_password() {
        let f = fixture().await;

        f.service
            .create_admin("second@example.test", &Password::new("another long secret"))
            .await
            .unwrap();
        let (_, user) = f
            .service
            .login(
                "second@example.test",
                &Password::new("another long secret"),
                IP,
            )
            .await
            .unwrap();

        assert_eq!(user.email.as_str(), "second@example.test");
    }

    #[tokio::test]
    async fn f001_s08_create_admin_refuses_a_short_password() {
        let f = fixture().await;

        let result = f
            .service
            .create_admin("second@example.test", &Password::new("too short"))
            .await;

        assert!(matches!(result, Err(CreateAdminError::WeakPassword(_))));
    }

    #[tokio::test]
    async fn f001_s08_create_admin_refuses_a_duplicate_email() {
        let f = fixture().await;

        let result = f
            .service
            .create_admin(
                "Proprio@Example.test",
                &Password::new("another long secret"),
            )
            .await;

        assert!(matches!(result, Err(CreateAdminError::AlreadyExists)));
    }
}
