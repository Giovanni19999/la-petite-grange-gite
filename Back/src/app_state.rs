//! Dependencies shared by the HTTP handlers (dependency injection).

use std::fmt;
use std::sync::Arc;

use crate::application::auth::AuthService;
use crate::application::health::DatabaseProbe;

/// Application state: the use cases and ports the handlers need.
#[derive(Clone)]
pub struct AppState {
    pub database: Arc<dyn DatabaseProbe>,
    pub auth: Arc<AuthService>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}
