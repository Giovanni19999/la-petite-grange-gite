//! Owner authentication rules: email, password policy, session tokens and lifetimes.

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

/// Identifier of an admin user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub u64);

/// A normalised email address (trimmed, lower-case).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

/// Why an email was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid email address")]
pub struct InvalidEmail;

/// Maximum length of an email address (RFC 5321).
const EMAIL_MAX_LEN: usize = 254;

impl Email {
    /// Normalises and validates an email address.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidEmail`] when the address is empty, too long, or not of the
    /// form `local@domain.tld`.
    pub fn parse(raw: &str) -> Result<Self, InvalidEmail> {
        let email = raw.trim().to_lowercase();
        let Some((local, domain)) = email.split_once('@') else {
            return Err(InvalidEmail);
        };
        let valid = email.len() <= EMAIL_MAX_LEN
            && !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !domain.contains('@')
            && !email.chars().any(char::is_whitespace);
        if valid {
            Ok(Self(email))
        } else {
            Err(InvalidEmail)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A plain-text password, never printed.
#[derive(Clone)]
pub struct Password(String);

impl Password {
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(***)")
    }
}

/// Why a new password was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WeakPassword {
    #[error("the password must contain at least {min} characters")]
    TooShort { min: usize },
}

/// Minimum number of characters of a new password (technical security rule, ADR-0011).
pub const PASSWORD_MIN_CHARS: usize = 12;

/// Checks a password chosen for a new account.
///
/// # Errors
///
/// Returns [`WeakPassword`] when the password is shorter than [`PASSWORD_MIN_CHARS`].
pub fn check_new_password(password: &Password) -> Result<(), WeakPassword> {
    if password.expose().chars().count() < PASSWORD_MIN_CHARS {
        return Err(WeakPassword::TooShort {
            min: PASSWORD_MIN_CHARS,
        });
    }
    Ok(())
}

/// Random secret given to the browser in the session cookie. Only its hash is stored.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionToken(String);

/// Number of random bytes in a session token.
pub const SESSION_TOKEN_BYTES: usize = 32;

impl SessionToken {
    /// Builds a token from random bytes produced by a cryptographically secure generator.
    #[must_use]
    pub fn from_random_bytes(bytes: &[u8; SESSION_TOKEN_BYTES]) -> Self {
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Wraps a token received from a client cookie.
    #[must_use]
    pub fn from_cookie(value: &str) -> Self {
        Self(value.to_owned())
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// SHA-256 of the token, hex-encoded: the only form stored in the database.
    #[must_use]
    pub fn hash(&self) -> SessionTokenHash {
        use std::fmt::Write as _;

        let digest = Sha256::digest(self.0.as_bytes());
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in &digest {
            // Writing to a String cannot fail.
            let _ = write!(hex, "{byte:02x}");
        }
        SessionTokenHash(hex)
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionToken(***)")
    }
}

/// Hex-encoded SHA-256 of a session token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionTokenHash(pub String);

/// Lifetimes of a session (technical security settings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionPolicy {
    /// Maximum lifetime from creation.
    pub max_age: Duration,
    /// Maximum time without any request.
    pub idle_timeout: Duration,
}

/// A stored session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub user_id: UserId,
    pub token_hash: SessionTokenHash,
    pub created_at: OffsetDateTime,
    pub last_used_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

impl Session {
    /// Opens a session at `now`.
    #[must_use]
    pub fn open(
        user_id: UserId,
        token: &SessionToken,
        now: OffsetDateTime,
        policy: SessionPolicy,
    ) -> Self {
        Self {
            user_id,
            token_hash: token.hash(),
            created_at: now,
            last_used_at: now,
            expires_at: now + policy.max_age,
        }
    }

    /// A session is active until its absolute expiry and while it keeps being used.
    #[must_use]
    pub fn is_active(&self, now: OffsetDateTime, policy: SessionPolicy) -> bool {
        now < self.expires_at && now - self.last_used_at < policy.idle_timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    const POLICY: SessionPolicy = SessionPolicy {
        max_age: Duration::days(30),
        idle_timeout: Duration::days(7),
    };

    #[test]
    fn email_is_trimmed_and_lower_cased() {
        let email = Email::parse("  Proprio@Example.TEST ").unwrap();
        assert_eq!(email.as_str(), "proprio@example.test");
    }

    #[test]
    fn invalid_emails_are_rejected() {
        for raw in [
            "",
            "proprio",
            "@example.test",
            "a@b",
            "a@.test",
            "a b@example.test",
        ] {
            assert_eq!(Email::parse(raw), Err(InvalidEmail), "{raw:?}");
        }
    }

    #[test]
    fn short_password_is_rejected() {
        assert_eq!(
            check_new_password(&Password::new("elevenchars")),
            Err(WeakPassword::TooShort { min: 12 })
        );
        assert!(check_new_password(&Password::new("twelve chars")).is_ok());
    }

    #[test]
    fn password_and_token_are_never_printed() {
        let token = SessionToken::from_random_bytes(&[7; SESSION_TOKEN_BYTES]);
        assert_eq!(format!("{:?}", Password::new("secret")), "Password(***)");
        assert_eq!(format!("{token:?}"), "SessionToken(***)");
    }

    #[test]
    fn token_hash_is_stable_hex_sha256() {
        let token = SessionToken::from_cookie("abc");
        assert_eq!(
            token.hash().0,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn session_is_active_when_recently_used() {
        let now = datetime!(2026-11-20 10:00 UTC);
        let session = Session::open(UserId(1), &SessionToken::from_cookie("t"), now, POLICY);

        assert!(session.is_active(now + Duration::days(6), POLICY));
    }

    #[test]
    fn f001_s06_idle_session_expired() {
        let now = datetime!(2026-11-20 10:00 UTC);
        let session = Session::open(UserId(1), &SessionToken::from_cookie("t"), now, POLICY);

        assert!(!session.is_active(now + Duration::days(7), POLICY));
    }

    #[test]
    fn session_expires_after_max_age_even_if_used() {
        let now = datetime!(2026-11-20 10:00 UTC);
        let mut session = Session::open(UserId(1), &SessionToken::from_cookie("t"), now, POLICY);
        session.last_used_at = now + Duration::days(29) + Duration::hours(23);

        assert!(!session.is_active(now + Duration::days(30), POLICY));
    }
}
