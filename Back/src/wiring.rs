//! Assembles the production adapters (composition, ADR-0002).
//!
//! [`Infrastructure`] owns the shared resources (database pool) and their lifecycle;
//! the free functions build use cases from them.

use std::sync::Arc;

use anyhow::Context;
use sqlx::MySqlPool;

use crate::app_state::AppState;
use crate::application::admin_accounts::AdminAccounts;
use crate::application::auth::{AuthPorts, AuthService, LoginThrottle};
use crate::config::Config;
use crate::domain::auth::SessionPolicy;
use crate::infrastructure::auth::{
    Argon2Hasher, MariaDbSessions, MariaDbUsers, OsRandom, SystemClock,
};
use crate::infrastructure::mariadb::{MariaDbProbe, lazy_pool, migrate};

/// What to do when the migrations cannot be applied at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Migrations {
    /// Log the error and continue (the server starts, health reports the database down).
    BestEffort,
    /// Fail (one-shot commands need a ready database).
    Required,
}

/// Shared infrastructure resources and their lifecycle.
#[derive(Debug, Clone)]
pub struct Infrastructure {
    pool: MySqlPool,
    config: Config,
}

impl Infrastructure {
    /// Creates the database pool and applies the pending migrations.
    ///
    /// # Errors
    ///
    /// Returns an error when `DATABASE_URL` is invalid, or when the migrations fail and
    /// `migrations` is [`Migrations::Required`].
    pub async fn start(config: &Config, migrations: Migrations) -> anyhow::Result<Self> {
        let pool = lazy_pool(config).context("invalid DATABASE_URL")?;
        if let Err(error) = migrate(&pool).await {
            match migrations {
                Migrations::Required => {
                    return Err(error).context("cannot apply database migrations");
                }
                Migrations::BestEffort => {
                    tracing::error!(%error, "database migrations not applied");
                }
            }
        }
        Ok(Self {
            pool,
            config: config.clone(),
        })
    }

    /// State of the HTTP adapter.
    ///
    /// # Errors
    ///
    /// Returns an error if a service cannot be initialised.
    pub fn app_state(&self) -> anyhow::Result<AppState> {
        app_state(
            &self.pool,
            self.config.session_policy,
            self.config.login_throttle,
        )
    }

    /// Use case of the command-line adapter.
    ///
    /// # Errors
    ///
    /// Returns an error if the password hasher cannot be initialised.
    pub fn admin_accounts(&self) -> anyhow::Result<AdminAccounts> {
        admin_accounts(&self.pool)
    }

    /// Closes the database connections.
    pub async fn shutdown(self) {
        self.pool.close().await;
    }
}

/// Builds the authentication service backed by `MariaDB`.
///
/// # Errors
///
/// Returns an error if the password hasher cannot be initialised.
pub fn auth_service(
    pool: &MySqlPool,
    policy: SessionPolicy,
    throttle: LoginThrottle,
) -> anyhow::Result<AuthService> {
    let ports = AuthPorts {
        users: Arc::new(MariaDbUsers::new(pool.clone())),
        sessions: Arc::new(MariaDbSessions::new(pool.clone())),
        hasher: Arc::new(Argon2Hasher::new()?),
        clock: Arc::new(SystemClock),
        random: Arc::new(OsRandom),
    };
    Ok(AuthService::new(ports, policy, throttle))
}

/// Builds the admin account management use case backed by `MariaDB`.
///
/// # Errors
///
/// Returns an error if the password hasher cannot be initialised.
pub fn admin_accounts(pool: &MySqlPool) -> anyhow::Result<AdminAccounts> {
    Ok(AdminAccounts::new(
        Arc::new(MariaDbUsers::new(pool.clone())),
        Arc::new(Argon2Hasher::new()?),
    ))
}

/// Builds the full application state.
///
/// # Errors
///
/// Returns an error if a service cannot be initialised.
pub fn app_state(
    pool: &MySqlPool,
    policy: SessionPolicy,
    throttle: LoginThrottle,
) -> anyhow::Result<AppState> {
    Ok(AppState {
        database: Arc::new(MariaDbProbe::new(pool.clone())),
        auth: Arc::new(auth_service(pool, policy, throttle)?),
    })
}
