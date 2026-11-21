//! Calendar use cases (F005): blocked periods of the cottage and the nights visitors cannot
//! book.

use std::fmt;
use std::sync::Arc;

use time::{Date, OffsetDateTime};

use crate::application::auth::Clock;
use crate::application::health::BoxFuture;
use crate::domain::auth::UserId;
use crate::domain::calendar::{
    BlockedPeriodInput, BookingHorizon, DateRange, PeriodError, UnavailableNights, paris_date,
    unavailable_nights,
};

/// Identifier of a cottage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CottageId(pub u64);

/// A cottage and the house rules F005 needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cottage {
    pub id: CottageId,
    pub slug: String,
    pub name: String,
    pub horizon: BookingHorizon,
}

/// Identifier of a blocked period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeriodId(pub u64);

/// A blocked period as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPeriod {
    pub id: PeriodId,
    pub cottage: CottageId,
    pub period: BlockedPeriodInput,
    pub updated_at: OffsetDateTime,
}

/// What happened to a period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodAction {
    Created,
    Updated,
    Deleted,
}

impl PeriodAction {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Deleted => "deleted",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "created" => Some(Self::Created),
            "updated" => Some(Self::Updated),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }
}

/// One line of the history of the periods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodEvent {
    pub period: PeriodId,
    pub action: PeriodAction,
    pub before: Option<BlockedPeriodInput>,
    pub after: Option<BlockedPeriodInput>,
    pub author: Option<UserId>,
    /// Email of the author, for the owners (none if the account was deleted).
    pub author_email: Option<String>,
    pub at: OffsetDateTime,
}

/// A stay (booking, hold) occupying nights; its label names it for the owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stay {
    pub label: String,
    pub range: DateRange,
}

/// Port: the cottage and its house rules.
pub trait CottageRepository: Send + Sync {
    /// The cottage the site manages (the first one).
    fn default_cottage(&self) -> BoxFuture<'_, anyhow::Result<Cottage>>;
}

/// Port: blocked periods and their history.
pub trait BlockedPeriodRepository: Send + Sync {
    fn list(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<Vec<StoredPeriod>>>;
    fn find(&self, id: PeriodId) -> BoxFuture<'_, anyhow::Result<Option<StoredPeriod>>>;
    /// Stores a change and its history line in one transaction; returns the period id.
    fn save<'a>(&'a self, change: &'a PeriodChange) -> BoxFuture<'a, anyhow::Result<PeriodId>>;
    /// Latest history lines first.
    fn history(
        &self,
        cottage: CottageId,
        limit: u32,
    ) -> BoxFuture<'_, anyhow::Result<Vec<PeriodEvent>>>;
}

/// A change to store: the period after the change (`None` when deleted) and its history line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodChange {
    pub cottage: CottageId,
    /// `None` for a new period.
    pub id: Option<PeriodId>,
    pub action: PeriodAction,
    pub before: Option<BlockedPeriodInput>,
    pub after: Option<BlockedPeriodInput>,
    pub author: UserId,
    pub at: OffsetDateTime,
}

/// Port: stays occupying nights (bookings and holds; F007, F014).
pub trait StayCalendar: Send + Sync {
    /// Stays sharing at least one night with `range`.
    fn stays<'a>(
        &'a self,
        cottage: CottageId,
        range: &'a DateRange,
    ) -> BoxFuture<'a, anyhow::Result<Vec<Stay>>>;
}

/// No stay at all: bookings do not exist yet (F007, F014 plug theirs here).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoStays;

impl StayCalendar for NoStays {
    fn stays<'a>(
        &'a self,
        _cottage: CottageId,
        _range: &'a DateRange,
    ) -> BoxFuture<'a, anyhow::Result<Vec<Stay>>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

/// Why a calendar operation failed.
#[derive(Debug, thiserror::Error)]
pub enum CalendarError {
    #[error("période introuvable")]
    NotFound,
    #[error("période invalide")]
    Invalid(Vec<PeriodError>),
    #[error("ces dates chevauchent une autre période ({})", describe(&.0.period))]
    Overlap(Box<StoredPeriod>),
    #[error("des séjours occupent déjà ces dates : {}", .0.iter().map(|stay| stay.label.clone()).collect::<Vec<_>>().join(", "))]
    StayConflict(Vec<Stay>),
    #[error("période demandée invalide : {0}")]
    BadRange(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// "du 10/03/2027 au 15/03/2027" or "à partir du 01/11/2026".
fn describe(period: &BlockedPeriodInput) -> String {
    let day = |date: Date| {
        format!(
            "{:02}/{:02}/{}",
            date.day(),
            date.month() as u8,
            date.year()
        )
    };
    match period.end {
        Some(end) => format!("du {} au {}", day(period.start), day(end)),
        None => format!("à partir du {}", day(period.start)),
    }
}

/// Ports of the calendar service.
pub struct CalendarPorts {
    pub cottages: Arc<dyn CottageRepository>,
    pub periods: Arc<dyn BlockedPeriodRepository>,
    pub stays: Arc<dyn StayCalendar>,
    pub clock: Arc<dyn Clock>,
}

impl fmt::Debug for CalendarPorts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarPorts").finish_non_exhaustive()
    }
}

/// Calendar use cases.
#[derive(Clone)]
pub struct CalendarService {
    cottages: Arc<dyn CottageRepository>,
    periods: Arc<dyn BlockedPeriodRepository>,
    stays: Arc<dyn StayCalendar>,
    clock: Arc<dyn Clock>,
    /// Longest range the availability endpoint answers (technical limit).
    max_range_days: u32,
}

impl fmt::Debug for CalendarService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarService")
            .field("max_range_days", &self.max_range_days)
            .finish_non_exhaustive()
    }
}

impl CalendarService {
    #[must_use]
    pub fn new(ports: CalendarPorts, max_range_days: u32) -> Self {
        Self {
            cottages: ports.cottages,
            periods: ports.periods,
            stays: ports.stays,
            clock: ports.clock,
            max_range_days,
        }
    }

    /// The cottage the site manages.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn cottage(&self) -> anyhow::Result<Cottage> {
        self.cottages.default_cottage().await
    }

    /// Every blocked period of the cottage, by start date.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn periods(&self) -> anyhow::Result<Vec<StoredPeriod>> {
        let cottage = self.cottage().await?;
        let mut periods = self.periods.list(cottage.id).await?;
        periods.sort_by_key(|period| (period.period.start, period.id.0));
        Ok(periods)
    }

    /// Closes dates.
    ///
    /// # Errors
    ///
    /// Invalid period, overlap with another period, conflict with a stay, infrastructure.
    pub async fn create(
        &self,
        input: BlockedPeriodInput,
        author: UserId,
    ) -> Result<PeriodId, CalendarError> {
        let period = input.validated().map_err(CalendarError::Invalid)?;
        let cottage = self.cottage().await?;
        self.check_free(cottage.id, None, &period).await?;
        Ok(self
            .periods
            .save(&PeriodChange {
                cottage: cottage.id,
                id: None,
                action: PeriodAction::Created,
                before: None,
                after: Some(period),
                author,
                at: self.clock.now(),
            })
            .await?)
    }

    /// Changes a period.
    ///
    /// # Errors
    ///
    /// Not found, invalid period, overlap, conflict with a stay, infrastructure.
    pub async fn update(
        &self,
        id: PeriodId,
        input: BlockedPeriodInput,
        author: UserId,
    ) -> Result<(), CalendarError> {
        let period = input.validated().map_err(CalendarError::Invalid)?;
        let stored = self
            .periods
            .find(id)
            .await?
            .ok_or(CalendarError::NotFound)?;
        self.check_free(stored.cottage, Some(id), &period).await?;
        self.periods
            .save(&PeriodChange {
                cottage: stored.cottage,
                id: Some(id),
                action: PeriodAction::Updated,
                before: Some(stored.period),
                after: Some(period),
                author,
                at: self.clock.now(),
            })
            .await?;
        Ok(())
    }

    /// Reopens the dates of a period.
    ///
    /// # Errors
    ///
    /// Not found, infrastructure.
    pub async fn delete(&self, id: PeriodId, author: UserId) -> Result<(), CalendarError> {
        let stored = self
            .periods
            .find(id)
            .await?
            .ok_or(CalendarError::NotFound)?;
        self.periods
            .save(&PeriodChange {
                cottage: stored.cottage,
                id: Some(id),
                action: PeriodAction::Deleted,
                before: Some(stored.period),
                after: None,
                author,
                at: self.clock.now(),
            })
            .await?;
        Ok(())
    }

    /// History of the changes, latest first.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn history(&self, limit: u32) -> anyhow::Result<Vec<PeriodEvent>> {
        let cottage = self.cottage().await?;
        self.periods.history(cottage.id, limit).await
    }

    /// Nights from `from` to `to` (excluded) visitors cannot book, without any detail.
    ///
    /// # Errors
    ///
    /// Invalid or too long range, infrastructure.
    pub async fn availability(
        &self,
        from: Date,
        to: Date,
    ) -> Result<UnavailableNights, CalendarError> {
        if to <= from {
            return Err(CalendarError::BadRange(
                "la fin doit être après le début".to_owned(),
            ));
        }
        if (to - from).whole_days() > i64::from(self.max_range_days) {
            return Err(CalendarError::BadRange(format!(
                "{} jours au maximum",
                self.max_range_days
            )));
        }
        let cottage = self.cottage().await?;
        let range = DateRange {
            start: from,
            end: Some(to),
        };
        let blocked: Vec<DateRange> = self
            .periods
            .list(cottage.id)
            .await?
            .iter()
            .map(|period| period.period.range())
            .filter(|period| period.overlaps(&range))
            .collect();
        let stays: Vec<DateRange> = self
            .stays
            .stays(cottage.id, &range)
            .await?
            .into_iter()
            .map(|stay| stay.range)
            .collect();
        let today = paris_date(self.clock.now());
        Ok(unavailable_nights(
            from,
            to,
            &blocked,
            &stays,
            cottage.horizon,
            today,
        ))
    }

    /// A period may not overlap another period nor a stay.
    async fn check_free(
        &self,
        cottage: CottageId,
        itself: Option<PeriodId>,
        period: &BlockedPeriodInput,
    ) -> Result<(), CalendarError> {
        let range = period.range();
        if let Some(other) = self
            .periods
            .list(cottage)
            .await?
            .into_iter()
            .find(|other| Some(other.id) != itself && other.period.range().overlaps(&range))
        {
            return Err(CalendarError::Overlap(Box::new(other)));
        }
        let stays = self.stays.stays(cottage, &range).await?;
        if !stays.is_empty() {
            return Err(CalendarError::StayConflict(stays));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "calendar_tests.rs"]
mod tests;
