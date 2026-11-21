//! Promotions and options use cases (F026): the owners manage them, the quote uses them.

use std::fmt;
use std::sync::Arc;

use time::OffsetDateTime;

use crate::application::auth::Clock;
use crate::application::calendar::{CottageId, CottageRepository};
use crate::application::health::BoxFuture;
use crate::domain::auth::UserId;
use crate::domain::offers::{OfferError, Promotion, StayOption};

/// A promotion as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPromotion {
    pub id: u64,
    pub promotion: Promotion,
}

/// An option as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOption {
    pub id: u64,
    pub option: StayOption,
}

/// History line of a change (kept with the price history of F006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferChange {
    pub description: String,
    pub author: UserId,
    pub at: OffsetDateTime,
}

/// What to store: the new value (`None` to delete) of an existing (`Some` id) or new item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferWrite<'a, T> {
    pub id: Option<u64>,
    pub value: Option<&'a T>,
    pub change: &'a OfferChange,
}

/// Port: promotions and options of a cottage.
pub trait OffersRepository: Send + Sync {
    fn promotions(&self, cottage: CottageId)
    -> BoxFuture<'_, anyhow::Result<Vec<StoredPromotion>>>;
    /// In display order.
    fn options(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<Vec<StoredOption>>>;
    /// Stores the change and its history line in one transaction; returns the id.
    fn save_promotion<'a>(
        &'a self,
        cottage: CottageId,
        write: OfferWrite<'a, Promotion>,
    ) -> BoxFuture<'a, anyhow::Result<u64>>;
    fn save_option<'a>(
        &'a self,
        cottage: CottageId,
        write: OfferWrite<'a, StayOption>,
    ) -> BoxFuture<'a, anyhow::Result<u64>>;
}

/// Why a promotion or option cannot be changed.
#[derive(Debug, thiserror::Error)]
pub enum OffersError {
    #[error("introuvable")]
    NotFound,
    #[error("contenu invalide")]
    Invalid(Vec<OfferError>),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Promotions and options use cases.
#[derive(Clone)]
pub struct OffersService {
    cottages: Arc<dyn CottageRepository>,
    offers: Arc<dyn OffersRepository>,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for OffersService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OffersService").finish_non_exhaustive()
    }
}

impl OffersService {
    #[must_use]
    pub fn new(
        cottages: Arc<dyn CottageRepository>,
        offers: Arc<dyn OffersRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            cottages,
            offers,
            clock,
        }
    }

    async fn cottage(&self) -> anyhow::Result<CottageId> {
        Ok(self.cottages.default_cottage().await?.id)
    }

    /// Every promotion.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn promotions(&self) -> anyhow::Result<Vec<StoredPromotion>> {
        let cottage = self.cottage().await?;
        self.offers.promotions(cottage).await
    }

    /// Every option, in display order.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn options(&self) -> anyhow::Result<Vec<StoredOption>> {
        let cottage = self.cottage().await?;
        self.offers.options(cottage).await
    }

    fn change(&self, description: String, author: UserId) -> OfferChange {
        OfferChange {
            description,
            author,
            at: self.clock.now(),
        }
    }

    /// Creates (`id` `None`), replaces or deletes (`promotion` `None`) a promotion.
    ///
    /// # Errors
    ///
    /// Unknown id, invalid promotion, infrastructure.
    pub async fn write_promotion(
        &self,
        id: Option<u64>,
        promotion: Option<Promotion>,
        author: UserId,
    ) -> Result<u64, OffersError> {
        let promotion = promotion
            .map(Promotion::validated)
            .transpose()
            .map_err(OffersError::Invalid)?;
        let cottage = self.cottage().await?;
        let before = match id {
            Some(id) => Some(
                self.offers
                    .promotions(cottage)
                    .await?
                    .into_iter()
                    .find(|stored| stored.id == id)
                    .ok_or(OffersError::NotFound)?
                    .promotion,
            ),
            None => None,
        };
        let description = match (&before, &promotion) {
            (None, Some(after)) => format!("Promotion ajoutée : {}", after.describe()),
            (Some(_), Some(after)) => format!("Promotion modifiée : {}", after.describe()),
            (Some(before), None) => format!("Promotion supprimée : {}", before.describe()),
            (None, None) => return Err(OffersError::NotFound),
        };
        let change = self.change(description, author);
        Ok(self
            .offers
            .save_promotion(
                cottage,
                OfferWrite {
                    id,
                    value: promotion.as_ref(),
                    change: &change,
                },
            )
            .await?)
    }

    /// Creates (`id` `None`), replaces or deletes (`option` `None`) an option.
    ///
    /// # Errors
    ///
    /// Unknown id, invalid option, infrastructure.
    pub async fn write_option(
        &self,
        id: Option<u64>,
        option: Option<StayOption>,
        author: UserId,
    ) -> Result<u64, OffersError> {
        let option = option
            .map(StayOption::validated)
            .transpose()
            .map_err(OffersError::Invalid)?;
        let cottage = self.cottage().await?;
        let before = match id {
            Some(id) => Some(
                self.offers
                    .options(cottage)
                    .await?
                    .into_iter()
                    .find(|stored| stored.id == id)
                    .ok_or(OffersError::NotFound)?
                    .option,
            ),
            None => None,
        };
        let description = match (&before, &option) {
            (None, Some(after)) => format!("Option ajoutée : {}", after.describe()),
            (Some(_), Some(after)) => format!("Option modifiée : {}", after.describe()),
            (Some(before), None) => format!("Option supprimée : {}", before.describe()),
            (None, None) => return Err(OffersError::NotFound),
        };
        let change = self.change(description, author);
        Ok(self
            .offers
            .save_option(
                cottage,
                OfferWrite {
                    id,
                    value: option.as_ref(),
                    change: &change,
                },
            )
            .await?)
    }
}
