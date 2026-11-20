//! Dependencies shared by the HTTP handlers (dependency injection).

use std::fmt;
use std::sync::Arc;

use crate::application::health::DatabaseProbe;

/// Application state: the ports the handlers need, behind trait objects.
#[derive(Clone)]
pub struct AppState {
    pub database: Arc<dyn DatabaseProbe>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}
