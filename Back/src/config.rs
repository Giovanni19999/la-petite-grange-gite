//! Technical configuration, read from environment variables and validated at startup.
//!
//! Only technical settings live here (environment, addresses, connection strings).
//! Business values are owner-editable settings stored in the database
//! (see `docs/adr/0010-no-business-value-hardcoded.md`).

use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;

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

        Ok(Self {
            app_env,
            bind_addr,
            database_url: Secret(database_url),
            database_max_connections,
            log_format,
        })
    }
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
