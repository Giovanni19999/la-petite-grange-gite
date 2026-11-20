//! Assembles the production adapters into the application state.

use std::sync::Arc;

use sqlx::MySqlPool;

use crate::app_state::AppState;
use crate::application::auth::{AuthPorts, AuthService, LoginThrottle};
use crate::domain::auth::SessionPolicy;
use crate::infrastructure::auth::{
    Argon2Hasher, MariaDbSessions, MariaDbUsers, OsRandom, SystemClock,
};
use crate::infrastructure::mariadb::MariaDbProbe;

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
