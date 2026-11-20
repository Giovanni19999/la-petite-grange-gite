//! `MariaDB` adapter: connection pool and health probe.

use std::time::Duration;

use sqlx::MySqlPool;
use sqlx::mysql::MySqlPoolOptions;

use crate::application::health::{BoxFuture, DatabaseProbe};
use crate::config::Config;

/// How long to wait for a free connection before giving up.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(3);

/// Creates the connection pool without connecting yet, so the API can start
/// (and report a degraded health) while the database is still unavailable.
///
/// # Errors
///
/// Returns an error when the connection string cannot be parsed.
pub fn lazy_pool(config: &Config) -> Result<MySqlPool, sqlx::Error> {
    MySqlPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect_lazy(config.database_url.expose())
}

/// Health probe running `SELECT 1` on the pool.
#[derive(Debug, Clone)]
pub struct MariaDbProbe {
    pool: MySqlPool,
}

impl MariaDbProbe {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl DatabaseProbe for MariaDbProbe {
    fn ping(&self) -> BoxFuture<'_, bool> {
        Box::pin(async move {
            match sqlx::query("SELECT 1").execute(&self.pool).await {
                Ok(_) => true,
                Err(error) => {
                    tracing::warn!(%error, "database health check failed");
                    false
                }
            }
        })
    }
}
