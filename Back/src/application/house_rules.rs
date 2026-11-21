//! House rules use cases (F004): read and change the "Règles du gîte", with their history.

use std::fmt;
use std::sync::Arc;

use time::OffsetDateTime;

use crate::application::auth::Clock;
use crate::application::calendar::{CottageId, CottageRepository};
use crate::application::health::BoxFuture;
use crate::domain::auth::UserId;
use crate::domain::house_rules::{HouseRules, RuleError};

/// One line of the history of the house rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleChange {
    pub description: String,
    pub before: Option<HouseRules>,
    pub after: HouseRules,
    pub author: Option<UserId>,
    /// Email of the author, for the owners (none if the account was deleted).
    pub author_email: Option<String>,
    pub at: OffsetDateTime,
}

/// Port: house rules of a cottage and their history.
pub trait HouseRulesRepository: Send + Sync {
    fn load(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<HouseRules>>;
    /// Stores the rules and their history line in one transaction.
    fn save<'a>(
        &'a self,
        cottage: CottageId,
        change: &'a RuleChange,
    ) -> BoxFuture<'a, anyhow::Result<()>>;
    /// Latest lines first.
    fn history(
        &self,
        cottage: CottageId,
        limit: u32,
    ) -> BoxFuture<'_, anyhow::Result<Vec<RuleChange>>>;
}

/// Why the rules cannot be changed.
#[derive(Debug, thiserror::Error)]
pub enum HouseRulesError {
    #[error("règles invalides")]
    Invalid(Vec<RuleError>),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// House rules use cases.
#[derive(Clone)]
pub struct HouseRulesService {
    cottages: Arc<dyn CottageRepository>,
    rules: Arc<dyn HouseRulesRepository>,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for HouseRulesService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HouseRulesService").finish_non_exhaustive()
    }
}

impl HouseRulesService {
    #[must_use]
    pub fn new(
        cottages: Arc<dyn CottageRepository>,
        rules: Arc<dyn HouseRulesRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            cottages,
            rules,
            clock,
        }
    }

    /// Rules of the cottage.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn rules(&self) -> anyhow::Result<HouseRules> {
        let cottage = self.cottages.default_cottage().await?;
        self.rules.load(cottage.id).await
    }

    /// Replaces the rules and records what changed.
    ///
    /// # Errors
    ///
    /// Invalid rules, infrastructure.
    pub async fn update(&self, rules: HouseRules, author: UserId) -> Result<(), HouseRulesError> {
        let rules = rules.validated().map_err(HouseRulesError::Invalid)?;
        let cottage = self.cottages.default_cottage().await?;
        let before = self.rules.load(cottage.id).await?;
        self.rules
            .save(
                cottage.id,
                &RuleChange {
                    description: before.describe_changes(&rules),
                    before: Some(before),
                    after: rules,
                    author: Some(author),
                    author_email: None,
                    at: self.clock.now(),
                },
            )
            .await?;
        Ok(())
    }

    /// Latest changes first.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn history(&self, limit: u32) -> anyhow::Result<Vec<RuleChange>> {
        let cottage = self.cottages.default_cottage().await?;
        self.rules.history(cottage.id, limit).await
    }
}
