//! Public calendar and quote of a stay (F004): which nights visitors may book, their site
//! price, and the exact amounts of a stay.

use std::fmt;
use std::sync::Arc;

use time::{Date, Duration};

use crate::application::auth::Clock;
use crate::application::calendar::{CalendarError, CalendarService};
use crate::application::house_rules::HouseRulesService;
use crate::application::pricing::{PriceBookError, PricingService};
use crate::domain::calendar::paris_date;
use crate::domain::house_rules::{HouseRules, earliest_arrival};
use crate::domain::quote::{NightOffer, Quote, QuoteContext, QuoteRefusal, StayRequest, quote};

/// Seeded code of the site's own channel (F006): visitors only see its prices.
pub const SITE_CHANNEL: &str = "direct";

/// What the public calendar needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicCalendar {
    pub earliest_arrival: Date,
    /// Last night that can be booked (booking horizon of F005).
    pub last_night: Date,
    pub rules: HouseRules,
    /// Each night of the asked range; `price` is `None` when it cannot be booked.
    pub nights: Vec<NightOffer>,
}

/// Why the calendar or a quote cannot be given.
#[derive(Debug, thiserror::Error)]
pub enum QuoteError {
    #[error("devis impossible")]
    Refused(Vec<QuoteRefusal>),
    #[error("période demandée invalide : {0}")]
    BadRange(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<CalendarError> for QuoteError {
    fn from(error: CalendarError) -> Self {
        match error {
            CalendarError::BadRange(message) => Self::BadRange(message),
            other => Self::Internal(anyhow::Error::new(other)),
        }
    }
}

impl From<PriceBookError> for QuoteError {
    fn from(error: PriceBookError) -> Self {
        match error {
            PriceBookError::BadRange(message) => Self::BadRange(message),
            other => Self::Internal(anyhow::Error::new(other)),
        }
    }
}

/// Public calendar and quote use cases.
#[derive(Clone)]
pub struct QuoteService {
    calendar: Arc<CalendarService>,
    pricing: Arc<PricingService>,
    rules: Arc<HouseRulesService>,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for QuoteService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuoteService").finish_non_exhaustive()
    }
}

impl QuoteService {
    #[must_use]
    pub fn new(
        calendar: Arc<CalendarService>,
        pricing: Arc<PricingService>,
        rules: Arc<HouseRulesService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            calendar,
            pricing,
            rules,
            clock,
        }
    }

    /// Nights from `from` to `to` (excluded) with the site price of the free ones; never says
    /// why a night is unavailable.
    ///
    /// # Errors
    ///
    /// Invalid or too long range, infrastructure.
    pub async fn calendar(&self, from: Date, to: Date) -> Result<PublicCalendar, QuoteError> {
        let rules = self.rules.rules().await?;
        let today = paris_date(self.clock.now());
        let earliest = earliest_arrival(today, rules.min_delay_days);
        let nights = self.offers(from, to, earliest).await?;
        Ok(PublicCalendar {
            earliest_arrival: earliest,
            last_night: rules.horizon.first_closed_night(today) - Duration::days(1),
            rules,
            nights,
        })
    }

    /// Exact quote of a stay, computed here only.
    ///
    /// # Errors
    ///
    /// The reasons the stay cannot be booked, a too long stay, infrastructure.
    pub async fn quote(&self, request: StayRequest) -> Result<Quote, QuoteError> {
        if request.departure <= request.arrival {
            return Err(QuoteError::Refused(vec![QuoteRefusal::DatesOrder]));
        }
        let rules = self.rules.rules().await?;
        let today = paris_date(self.clock.now());
        let earliest = earliest_arrival(today, rules.min_delay_days);
        let nights = self
            .offers(request.arrival, request.departure, earliest)
            .await?;
        let cleaning_fee = self.pricing.book().await?.settings.cleaning_fee;
        quote(
            request,
            QuoteContext {
                today,
                rules: &rules,
                nights: &nights,
                cleaning_fee,
            },
        )
        .map_err(QuoteError::Refused)
    }

    /// Each night with its site price, `None` when taken, closed, too soon, beyond the
    /// horizon or without price.
    async fn offers(
        &self,
        from: Date,
        to: Date,
        earliest: Date,
    ) -> Result<Vec<NightOffer>, QuoteError> {
        let unavailable = self.calendar.availability(from, to).await?;
        let nights = self.pricing.nights(from, to).await?;
        Ok(nights
            .into_iter()
            .map(|night| {
                let bookable = night.date >= earliest && !unavailable.contains(&night.date);
                NightOffer {
                    date: night.date,
                    price: night
                        .prices
                        .get(SITE_CHANNEL)
                        .copied()
                        .flatten()
                        .filter(|_| bookable)
                        .map(|price| price.listed),
                    min_nights: night.min_nights,
                }
            })
            .collect())
    }
}
