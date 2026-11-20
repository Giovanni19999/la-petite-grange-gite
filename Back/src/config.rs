//! Technical configuration, read from environment variables and validated at startup.
//!
//! Only technical settings live here (environment, addresses, connection strings).
//! Business values are owner-editable settings stored in the database
//! (see `docs/adr/0010-no-business-value-hardcoded.md`).

use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use time::Duration;

use crate::application::auth::LoginThrottle;
use crate::application::media::MediaSettings;
use crate::domain::auth::SessionPolicy;
use crate::domain::media::{ImageSettings, UploadLimits};

/// Deployment environment of the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    Dev,
    Test,
    Staging,
    Production,
}

impl AppEnv {
    const ALLOWED: &'static str = "dev, test, staging, production";
}

impl FromStr for AppEnv {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "dev" => Ok(Self::Dev),
            "test" => Ok(Self::Test),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            other => Err(ConfigError::Invalid {
                variable: "APP_ENV",
                reason: format!("\"{other}\" is not one of: {}", Self::ALLOWED),
            }),
        }
    }
}

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable, for development.
    Pretty,
    /// One JSON object per line, for production log collection.
    Json,
}

impl FromStr for LogFormat {
    type Err = ConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            other => Err(ConfigError::Invalid {
                variable: "LOG_FORMAT",
                reason: format!("\"{other}\" is not one of: pretty, json"),
            }),
        }
    }
}

/// A value that must never be printed (passwords, connection strings…).
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Returns the secret value. Call only where the value is actually used.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// Validated technical configuration of the API.
#[derive(Debug, Clone)]
pub struct Config {
    pub app_env: AppEnv,
    pub bind_addr: SocketAddr,
    pub database_url: Secret,
    pub database_max_connections: u32,
    pub log_format: LogFormat,
    /// Admin sessions lifetimes (ADR-0011).
    pub session_policy: SessionPolicy,
    /// Failed-login throttling (ADR-0011).
    pub login_throttle: LoginThrottle,
    /// Trust `X-Forwarded-For` from the reverse proxy (true only behind it).
    pub trust_proxy_headers: bool,
    /// Directory of the media files (F003, ADR-0017): `public/` and `originals/`.
    pub media_dir: PathBuf,
    /// Media library limits, sizes and public URL.
    pub media: MediaSettings,
    /// Serve the public media files from the API (development only; Caddy does it otherwise).
    pub serve_media_files: bool,
}

/// Configuration error, always naming the faulty variable.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing environment variable {0}")]
    Missing(&'static str),
    #[error("invalid environment variable {variable}: {reason}")]
    Invalid {
        variable: &'static str,
        reason: String,
    },
}

/// Technical defaults used when an optional variable is not set.
mod defaults {
    pub const BIND_ADDR: &str = "127.0.0.1:8080";
    pub const DATABASE_MAX_CONNECTIONS: &str = "10";
    pub const LOG_FORMAT: &str = "pretty";
    pub const SESSION_MAX_AGE_DAYS: &str = "30";
    pub const SESSION_IDLE_DAYS: &str = "7";
    pub const LOGIN_MAX_FAILURES: &str = "5";
    pub const LOGIN_WINDOW_MINUTES: &str = "15";
    pub const MEDIA_DIR: &str = "./data/media";
    pub const MEDIA_PUBLIC_URL: &str = "/uploads";
    pub const MEDIA_MAX_FILE_MB: &str = "20";
    pub const MEDIA_MAX_FILES: &str = "20";
    pub const MEDIA_MAX_MEGAPIXELS: &str = "50";
    pub const MEDIA_WIDTHS: &str = "480,960,1440,1920";
    pub const MEDIA_WEBP_QUALITY: &str = "80";
}

impl Config {
    /// Reads the configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] naming the first missing or invalid variable.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the configuration through `lookup`, which returns the value of a variable.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] naming the first missing or invalid variable.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let required = |name: &'static str| {
            lookup(name)
                .filter(|value| !value.trim().is_empty())
                .ok_or(ConfigError::Missing(name))
        };
        let optional =
            |name: &str, default: &str| lookup(name).unwrap_or_else(|| default.to_owned());

        let app_env: AppEnv = required("APP_ENV")?.parse()?;

        let bind_addr = optional("BIND_ADDR", defaults::BIND_ADDR)
            .parse::<SocketAddr>()
            .map_err(|error| ConfigError::Invalid {
                variable: "BIND_ADDR",
                reason: error.to_string(),
            })?;

        let database_url = required("DATABASE_URL")?;
        if !database_url.starts_with("mysql://") && !database_url.starts_with("mariadb://") {
            return Err(ConfigError::Invalid {
                variable: "DATABASE_URL",
                reason: "must start with mysql:// or mariadb://".to_owned(),
            });
        }

        let database_max_connections = optional(
            "DATABASE_MAX_CONNECTIONS",
            defaults::DATABASE_MAX_CONNECTIONS,
        )
        .parse::<u32>()
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| ConfigError::Invalid {
            variable: "DATABASE_MAX_CONNECTIONS",
            reason: "must be a positive integer".to_owned(),
        })?;

        let log_format: LogFormat = optional("LOG_FORMAT", defaults::LOG_FORMAT).parse()?;

        let positive = |name: &'static str, default: &str| {
            optional(name, default)
                .parse::<u32>()
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| ConfigError::Invalid {
                    variable: name,
                    reason: "must be a positive integer".to_owned(),
                })
        };
        let session_policy = SessionPolicy {
            max_age: Duration::days(i64::from(positive(
                "SESSION_MAX_AGE_DAYS",
                defaults::SESSION_MAX_AGE_DAYS,
            )?)),
            idle_timeout: Duration::days(i64::from(positive(
                "SESSION_IDLE_DAYS",
                defaults::SESSION_IDLE_DAYS,
            )?)),
        };
        let trust_proxy_headers = match optional("TRUST_PROXY_HEADERS", "false").as_str() {
            "true" => true,
            "false" => false,
            _ => {
                return Err(ConfigError::Invalid {
                    variable: "TRUST_PROXY_HEADERS",
                    reason: "must be true or false".to_owned(),
                });
            }
        };
        let login_throttle = LoginThrottle {
            max_failures: positive("LOGIN_MAX_FAILURES", defaults::LOGIN_MAX_FAILURES)? as usize,
            window: Duration::minutes(i64::from(positive(
                "LOGIN_WINDOW_MINUTES",
                defaults::LOGIN_WINDOW_MINUTES,
            )?)),
        };

        let (media_dir, media, serve_media_files) = media_config(&lookup)?;

        Ok(Self {
            app_env,
            bind_addr,
            database_url: Secret(database_url),
            database_max_connections,
            log_format,
            session_policy,
            login_throttle,
            trust_proxy_headers,
            media_dir,
            media,
            serve_media_files,
        })
    }
}

fn optional(lookup: &impl Fn(&str) -> Option<String>, name: &str, default: &str) -> String {
    lookup(name).unwrap_or_else(|| default.to_owned())
}

fn positive(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: &str,
) -> Result<u32, ConfigError> {
    optional(lookup, name, default)
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| ConfigError::Invalid {
            variable: name,
            reason: "must be a positive integer".to_owned(),
        })
}

fn boolean(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: &str,
) -> Result<bool, ConfigError> {
    match optional(lookup, name, default).as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(ConfigError::Invalid {
            variable: name,
            reason: "must be true or false".to_owned(),
        }),
    }
}

/// Media library settings (F003, ADR-0017): directory, limits, sizes, public URL.
fn media_config(
    lookup: &impl Fn(&str) -> Option<String>,
) -> Result<(PathBuf, MediaSettings, bool), ConfigError> {
    let mut widths = Vec::new();
    for width in optional(lookup, "MEDIA_WIDTHS", defaults::MEDIA_WIDTHS).split(',') {
        let width = width
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|width| *width > 0)
            .ok_or_else(|| ConfigError::Invalid {
                variable: "MEDIA_WIDTHS",
                reason: "must be positive integers separated by commas".to_owned(),
            })?;
        widths.push(width);
    }
    widths.sort_unstable();
    widths.dedup();
    let quality = positive(lookup, "MEDIA_WEBP_QUALITY", defaults::MEDIA_WEBP_QUALITY)?;
    if quality > 100 {
        return Err(ConfigError::Invalid {
            variable: "MEDIA_WEBP_QUALITY",
            reason: "must be between 1 and 100".to_owned(),
        });
    }
    let media = MediaSettings {
        limits: UploadLimits {
            max_file_bytes: positive(lookup, "MEDIA_MAX_FILE_MB", defaults::MEDIA_MAX_FILE_MB)?
                as usize
                * 1024
                * 1024,
            max_files: positive(lookup, "MEDIA_MAX_FILES", defaults::MEDIA_MAX_FILES)? as usize,
            max_pixels: u64::from(positive(
                lookup,
                "MEDIA_MAX_MEGAPIXELS",
                defaults::MEDIA_MAX_MEGAPIXELS,
            )?) * 1_000_000,
        },
        images: ImageSettings {
            widths,
            #[allow(clippy::cast_precision_loss)]
            quality: quality as f32,
        },
        public_url: optional(lookup, "MEDIA_PUBLIC_URL", defaults::MEDIA_PUBLIC_URL),
    };
    let media_dir = PathBuf::from(optional(lookup, "MEDIA_DIR", defaults::MEDIA_DIR));
    let serve_media_files = boolean(lookup, "SERVE_MEDIA_FILES", "false")?;
    Ok((media_dir, media, serve_media_files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        move |name| map.get(name).cloned()
    }

    const VALID: &[(&str, &str)] = &[
        ("APP_ENV", "dev"),
        ("DATABASE_URL", "mysql://user:pass@localhost:3306/gite"),
    ];

    #[test]
    fn valid_minimal_configuration_uses_technical_defaults() {
        let config = Config::from_lookup(lookup_from(VALID)).expect("valid configuration");

        assert_eq!(config.app_env, AppEnv::Dev);
        assert_eq!(config.bind_addr.to_string(), "127.0.0.1:8080");
        assert_eq!(config.database_max_connections, 10);
        assert_eq!(config.log_format, LogFormat::Pretty);
        assert_eq!(config.session_policy.max_age, Duration::days(30));
        assert_eq!(config.session_policy.idle_timeout, Duration::days(7));
        assert_eq!(config.login_throttle.max_failures, 5);
        assert_eq!(config.login_throttle.window, Duration::minutes(15));
        assert!(!config.trust_proxy_headers);
        assert_eq!(config.media.images.widths, vec![480, 960, 1440, 1920]);
        assert_eq!(config.media.limits.max_file_bytes, 20 * 1024 * 1024);
        assert_eq!(config.media.limits.max_pixels, 50_000_000);
        assert_eq!(config.media.public_url, "/uploads");
        assert!(!config.serve_media_files);
    }

    #[test]
    fn rejects_invalid_media_widths() {
        let mut pairs = VALID.to_vec();
        pairs.push(("MEDIA_WIDTHS", "480,abc"));
        assert!(matches!(
            Config::from_lookup(lookup_from(&pairs)),
            Err(ConfigError::Invalid {
                variable: "MEDIA_WIDTHS",
                ..
            })
        ));
    }

    #[test]
    fn f000_s03_missing_database_url() {
        let result = Config::from_lookup(lookup_from(&[("APP_ENV", "dev")]));

        assert_eq!(result.unwrap_err(), ConfigError::Missing("DATABASE_URL"));
    }

    #[test]
    fn f000_s04_invalid_app_env() {
        let result = Config::from_lookup(lookup_from(&[
            ("APP_ENV", "prod"),
            ("DATABASE_URL", "mysql://localhost/gite"),
        ]));

        let error = result.unwrap_err();
        assert!(matches!(
            error,
            ConfigError::Invalid {
                variable: "APP_ENV",
                ..
            }
        ));
        assert!(error.to_string().contains("dev, test, staging, production"));
    }

    #[test]
    fn rejects_database_url_with_wrong_scheme() {
        let result = Config::from_lookup(lookup_from(&[
            ("APP_ENV", "dev"),
            ("DATABASE_URL", "postgres://localhost/gite"),
        ]));

        assert!(matches!(
            result.unwrap_err(),
            ConfigError::Invalid {
                variable: "DATABASE_URL",
                ..
            }
        ));
    }

    #[test]
    fn rejects_zero_max_connections() {
        let mut pairs = VALID.to_vec();
        pairs.push(("DATABASE_MAX_CONNECTIONS", "0"));

        assert!(matches!(
            Config::from_lookup(lookup_from(&pairs)).unwrap_err(),
            ConfigError::Invalid {
                variable: "DATABASE_MAX_CONNECTIONS",
                ..
            }
        ));
    }

    #[test]
    fn debug_output_never_shows_the_database_url() {
        let config = Config::from_lookup(lookup_from(VALID)).expect("valid configuration");

        let printed = format!("{config:?}");

        assert!(!printed.contains("pass"));
        assert!(printed.contains("Secret(***)"));
    }
}
