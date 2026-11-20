//! Dependencies shared by the HTTP handlers (dependency injection).

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use crate::application::auth::AuthService;
use crate::application::health::DatabaseProbe;
use crate::application::media::MediaService;
use crate::application::pages::PageService;

/// Application state: the use cases and ports the handlers need.
#[derive(Clone)]
pub struct AppState {
    pub database: Arc<dyn DatabaseProbe>,
    pub auth: Arc<AuthService>,
    pub pages: Arc<PageService>,
    pub media: Arc<MediaService>,
    /// Directory of the public media files, served by the API itself in development only
    /// (`SERVE_MEDIA_FILES`); in production Caddy serves them (ADR-0017).
    pub serve_media_dir: Option<PathBuf>,
    /// Read the client IP from `X-Forwarded-For` (only behind our reverse proxy).
    pub trust_proxy_headers: bool,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}
