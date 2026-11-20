//! Logging setup (`tracing`).

use tracing_subscriber::EnvFilter;

use crate::config::LogFormat;

/// Default filter when `RUST_LOG` is not set.
const DEFAULT_FILTER: &str = "info,tower_http=info";

/// Installs the global logger. Call once, at startup.
pub fn init(format: LogFormat) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match format {
        LogFormat::Pretty => builder.init(),
        LogFormat::Json => builder.json().init(),
    }
}
