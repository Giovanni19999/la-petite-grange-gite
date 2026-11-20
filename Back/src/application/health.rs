//! Health check use case and its port.

use std::future::Future;
use std::pin::Pin;

/// A boxed future, used by ports so they can be called through `dyn` trait objects.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Port: tells whether the database answers.
pub trait DatabaseProbe: Send + Sync {
    /// Returns `true` when the database answers a trivial query.
    fn ping(&self) -> BoxFuture<'_, bool>;
}

/// Status of one component of the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentStatus {
    Up,
    Down,
}

/// Result of a health check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthReport {
    pub database: ComponentStatus,
}

impl HealthReport {
    /// The API is healthy only when every component is up.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.database == ComponentStatus::Up
    }
}

/// Checks every component the API depends on.
pub async fn check_health(database: &dyn DatabaseProbe) -> HealthReport {
    let database = if database.ping().await {
        ComponentStatus::Up
    } else {
        ComponentStatus::Down
    };
    HealthReport { database }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedProbe(bool);

    impl DatabaseProbe for FixedProbe {
        fn ping(&self) -> BoxFuture<'_, bool> {
            let answer = self.0;
            Box::pin(async move { answer })
        }
    }

    #[tokio::test]
    async fn healthy_when_database_answers() {
        let report = check_health(&FixedProbe(true)).await;

        assert_eq!(report.database, ComponentStatus::Up);
        assert!(report.is_healthy());
    }

    #[tokio::test]
    async fn unhealthy_when_database_does_not_answer() {
        let report = check_health(&FixedProbe(false)).await;

        assert_eq!(report.database, ComponentStatus::Down);
        assert!(!report.is_healthy());
    }
}
