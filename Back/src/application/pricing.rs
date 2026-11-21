//! Price use cases (F006): the price book of the cottage (channels and their rates, default
//! prices, ranges with prices per channel, cleaning fee, minimum nights) and its history.
//!
//! Each change loads the whole book, applies the domain rules in memory and saves the new
//! book with its history line in one transaction: a cottage has a few dozen ranges at most.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use serde::Serialize;
use time::{Date, OffsetDateTime};

use crate::application::auth::Clock;
use crate::application::calendar::{CottageId, CottageRepository};
use crate::application::health::BoxFuture;
use crate::domain::auth::UserId;
use crate::domain::pricing::{
    ChannelPrice, ChannelRates, MIN_NIGHTS, PricingError, RangeInput, Ranged, cut_ranges,
    next_year, night_price, typical_stay,
};

/// Code of a channel (`direct`, `airbnb`, `booking`…).
pub type ChannelCode = String;

/// A channel and the rates used to help typing its prices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Channel {
    pub code: ChannelCode,
    pub name: String,
    #[serde(flatten)]
    pub rates: ChannelRates,
}

/// Settings of the cottage's prices (house rules, ADR-0010).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PricingSettings {
    /// Fixed per stay, in cents.
    pub cleaning_fee: u64,
    pub default_min_nights: u8,
    /// Warning threshold of the commission help, in percentage points.
    pub commission_alert_points: u8,
}

/// A range with its prices per channel (channels without price use their default price).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PricedRange {
    /// Identifier in the database; changes at every save of the book.
    pub id: u64,
    #[serde(flatten)]
    pub range: RangeInput,
    pub prices: BTreeMap<ChannelCode, ChannelPrice>,
}

/// Every price of the cottage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PriceBook {
    pub channels: Vec<Channel>,
    pub settings: PricingSettings,
    pub defaults: BTreeMap<ChannelCode, ChannelPrice>,
    /// Never overlapping, by start date.
    pub ranges: Vec<PricedRange>,
}

/// One line of the price history.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceChange {
    pub description: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub author: Option<UserId>,
    pub author_email: Option<String>,
    pub at: OffsetDateTime,
}

/// Port: the price book of a cottage and its history.
pub trait PricingRepository: Send + Sync {
    fn load(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<PriceBook>>;
    /// Replaces the whole book and records the change, in one transaction.
    fn save<'a>(
        &'a self,
        cottage: CottageId,
        book: &'a PriceBook,
        change: &'a PriceChange,
    ) -> BoxFuture<'a, anyhow::Result<()>>;
    /// Latest changes first.
    fn history(
        &self,
        cottage: CottageId,
        limit: u32,
    ) -> BoxFuture<'_, anyhow::Result<Vec<PriceChange>>>;
}

/// Prices of one night per channel (`None`: incomplete channel) and its minimum stay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NightPrices {
    pub date: Date,
    pub min_nights: u8,
    pub prices: BTreeMap<ChannelCode, Option<ChannelPrice>>,
}

/// Totals of a typical stay per channel: `(listed, net)` in cents, `None` when incomplete.
pub type TypicalStays = BTreeMap<ChannelCode, Option<(u64, u64)>>;

/// Why a price operation failed.
#[derive(Debug, thiserror::Error)]
pub enum PriceBookError {
    #[error("plage introuvable")]
    NotFound,
    #[error("prix invalides")]
    Invalid(Vec<PricingError>),
    #[error("période demandée invalide : {0}")]
    BadRange(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<PricingError> for PriceBookError {
    fn from(error: PricingError) -> Self {
        Self::Invalid(vec![error])
    }
}

/// Ports of the price service.
pub struct PricingPorts {
    pub cottages: Arc<dyn CottageRepository>,
    pub prices: Arc<dyn PricingRepository>,
    pub clock: Arc<dyn Clock>,
}

impl fmt::Debug for PricingPorts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PricingPorts").finish_non_exhaustive()
    }
}

/// Price use cases.
#[derive(Clone)]
pub struct PricingService {
    cottages: Arc<dyn CottageRepository>,
    prices: Arc<dyn PricingRepository>,
    clock: Arc<dyn Clock>,
    max_range_days: u32,
}

impl fmt::Debug for PricingService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PricingService").finish_non_exhaustive()
    }
}

fn snapshot<T: Serialize>(value: &T) -> Option<serde_json::Value> {
    serde_json::to_value(value).ok()
}

fn euros(cents: u64) -> String {
    if cents.is_multiple_of(100) {
        format!("{} €", cents / 100)
    } else {
        format!("{},{:02} €", cents / 100, cents % 100)
    }
}

fn day(date: Date) -> String {
    format!(
        "{:02}/{:02}/{}",
        date.day(),
        date.month() as u8,
        date.year()
    )
}

impl PricingService {
    #[must_use]
    pub fn new(ports: PricingPorts, max_range_days: u32) -> Self {
        Self {
            cottages: ports.cottages,
            prices: ports.prices,
            clock: ports.clock,
            max_range_days,
        }
    }

    async fn load(&self) -> anyhow::Result<(CottageId, PriceBook)> {
        let cottage = self.cottages.default_cottage().await?.id;
        Ok((cottage, self.prices.load(cottage).await?))
    }

    async fn save(
        &self,
        cottage: CottageId,
        book: &PriceBook,
        description: String,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
        author: UserId,
    ) -> anyhow::Result<()> {
        self.prices
            .save(
                cottage,
                book,
                &PriceChange {
                    description,
                    before,
                    after,
                    author: Some(author),
                    author_email: None,
                    at: self.clock.now(),
                },
            )
            .await
    }

    /// The whole price book.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn book(&self) -> anyhow::Result<PriceBook> {
        Ok(self.load().await?.1)
    }

    /// Channels without default price: incomplete on every night without range price.
    #[must_use]
    pub fn incomplete_channels(book: &PriceBook) -> Vec<ChannelCode> {
        book.channels
            .iter()
            .filter(|channel| !book.defaults.contains_key(&channel.code))
            .map(|channel| channel.code.clone())
            .collect()
    }

    /// Totals of a stay of `nights` with the given prices per channel (range prices falling
    /// back to the default prices).
    #[must_use]
    pub fn typical_stays(
        book: &PriceBook,
        nights: u64,
        range_prices: Option<&BTreeMap<ChannelCode, ChannelPrice>>,
    ) -> TypicalStays {
        book.channels
            .iter()
            .map(|channel| {
                let price = night_price(
                    range_prices.and_then(|prices| prices.get(&channel.code).copied()),
                    book.defaults.get(&channel.code).copied(),
                );
                let totals = price.map(|price| {
                    typical_stay(nights, price, book.settings.cleaning_fee, channel.rates)
                });
                (channel.code.clone(), totals)
            })
            .collect()
    }

    /// Cleaning fee, default minimum nights and warning threshold.
    ///
    /// # Errors
    ///
    /// Invalid minimum nights, infrastructure.
    pub async fn set_settings(
        &self,
        settings: PricingSettings,
        author: UserId,
    ) -> Result<(), PriceBookError> {
        if !MIN_NIGHTS.contains(&settings.default_min_nights) {
            return Err(PricingError::MinNights.into());
        }
        let (cottage, mut book) = self.load().await?;
        let before = snapshot(&book.settings);
        book.settings = settings;
        let description = format!(
            "Ménage {}, {} nuit(s) minimum par défaut",
            euros(settings.cleaning_fee),
            settings.default_min_nights
        );
        self.save(
            cottage,
            &book,
            description,
            before,
            snapshot(&settings),
            author,
        )
        .await?;
        Ok(())
    }

    /// Commission, payment fees and VAT of a channel.
    ///
    /// # Errors
    ///
    /// Unknown channel, invalid rate, infrastructure.
    pub async fn set_channel_rates(
        &self,
        code: &str,
        rates: ChannelRates,
        author: UserId,
    ) -> Result<(), PriceBookError> {
        let rates = rates.validated()?;
        let (cottage, mut book) = self.load().await?;
        let channel = book
            .channels
            .iter_mut()
            .find(|channel| channel.code == code)
            .ok_or_else(|| PricingError::UnknownChannel(code.to_owned()))?;
        let before = snapshot(&channel.rates);
        channel.rates = rates;
        let description = format!("Taux de {} modifiés", channel.name);
        self.save(
            cottage,
            &book,
            description,
            before,
            snapshot(&rates),
            author,
        )
        .await?;
        Ok(())
    }

    /// Sets the default prices of every channel at once ("Valider" of the default table): a
    /// channel with `None` has no default price.
    ///
    /// # Errors
    ///
    /// Unknown channel, invalid price, infrastructure.
    pub async fn set_default_prices(
        &self,
        prices: BTreeMap<ChannelCode, Option<ChannelPrice>>,
        author: UserId,
    ) -> Result<(), PriceBookError> {
        let (cottage, mut book) = self.load().await?;
        let set: BTreeMap<ChannelCode, ChannelPrice> = prices
            .iter()
            .filter_map(|(code, price)| price.map(|price| (code.clone(), price)))
            .collect();
        let set = validated_prices(&book, set)?;
        for code in prices.keys() {
            channel_name(&book, code)?;
        }
        let before = book.defaults.clone();
        for (code, price) in &prices {
            match price {
                Some(_) => book.defaults.insert(code.clone(), set[code]),
                None => book.defaults.remove(code),
            };
        }
        let summary: Vec<String> = book
            .channels
            .iter()
            .filter(|channel| prices.contains_key(&channel.code))
            .map(|channel| match book.defaults.get(&channel.code) {
                Some(price) => format!(
                    "{} {} / {}",
                    channel.name,
                    euros(price.listed),
                    euros(price.net)
                ),
                None => format!("{} sans prix", channel.name),
            })
            .collect();
        let description = format!("Prix par défaut : {}", summary.join(", "));
        let after = book.defaults.clone();
        self.save(
            cottage,
            &book,
            description,
            snapshot(&before),
            snapshot(&after),
            author,
        )
        .await?;
        Ok(())
    }

    /// Adds a range (it overwrites the ranges it overlaps).
    ///
    /// # Errors
    ///
    /// Invalid range or prices, unknown channel, infrastructure.
    pub async fn create_range(
        &self,
        input: RangeInput,
        prices: BTreeMap<ChannelCode, ChannelPrice>,
        author: UserId,
    ) -> Result<(), PriceBookError> {
        self.put_range(None, input, prices, author).await
    }

    /// Changes a range (dates, label, minimum nights, prices).
    ///
    /// # Errors
    ///
    /// Not found, invalid range or prices, unknown channel, infrastructure.
    pub async fn update_range(
        &self,
        id: u64,
        input: RangeInput,
        prices: BTreeMap<ChannelCode, ChannelPrice>,
        author: UserId,
    ) -> Result<(), PriceBookError> {
        self.put_range(Some(id), input, prices, author).await
    }

    async fn put_range(
        &self,
        id: Option<u64>,
        input: RangeInput,
        prices: BTreeMap<ChannelCode, ChannelPrice>,
        author: UserId,
    ) -> Result<(), PriceBookError> {
        let input = input.validated().map_err(PriceBookError::Invalid)?;
        let (cottage, mut book) = self.load().await?;
        let prices = validated_prices(&book, prices)?;
        let before = match id {
            Some(id) => {
                let index = book
                    .ranges
                    .iter()
                    .position(|range| range.id == id)
                    .ok_or(PriceBookError::NotFound)?;
                Some(book.ranges.remove(index))
            }
            None => None,
        };
        let new = PricedRange {
            id: 0,
            range: input,
            prices,
        };
        apply_range(&mut book.ranges, new.clone());
        let label = new
            .range
            .label
            .clone()
            .map_or_else(String::new, |label| format!(" « {label} »"));
        let description = format!(
            "Plage{label} du {} au {} {}",
            day(new.range.start),
            day(new.range.end),
            if before.is_some() {
                "modifiée"
            } else {
                "créée"
            }
        );
        self.save(
            cottage,
            &book,
            description,
            snapshot(&before),
            snapshot(&new),
            author,
        )
        .await?;
        Ok(())
    }

    /// Removes a range: its nights use the default prices again.
    ///
    /// # Errors
    ///
    /// Not found, infrastructure.
    pub async fn delete_range(&self, id: u64, author: UserId) -> Result<(), PriceBookError> {
        let (cottage, mut book) = self.load().await?;
        let index = book
            .ranges
            .iter()
            .position(|range| range.id == id)
            .ok_or(PriceBookError::NotFound)?;
        let removed = book.ranges.remove(index);
        let description = format!(
            "Plage du {} au {} supprimée",
            day(removed.range.start),
            day(removed.range.end)
        );
        self.save(
            cottage,
            &book,
            description,
            snapshot(&removed),
            None,
            author,
        )
        .await?;
        Ok(())
    }

    /// Copies the ranges starting in `year` to the next year, on the same weekdays.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn copy_year(&self, year: i32, author: UserId) -> Result<usize, PriceBookError> {
        let (cottage, mut book) = self.load().await?;
        let copies: Vec<PricedRange> = book
            .ranges
            .iter()
            .filter(|range| range.range.start.year() == year)
            .map(|range| PricedRange {
                id: 0,
                range: next_year(&range.range),
                prices: range.prices.clone(),
            })
            .collect();
        for copy in &copies {
            apply_range(&mut book.ranges, copy.clone());
        }
        let description = format!(
            "{} plage(s) de {year} recopiée(s) sur {}",
            copies.len(),
            year + 1
        );
        self.save(cottage, &book, description, None, snapshot(&copies), author)
            .await?;
        Ok(copies.len())
    }

    /// Copies every price of channel `from` to channel `to` (default and ranges).
    ///
    /// # Errors
    ///
    /// Unknown channel, infrastructure.
    pub async fn copy_channel(
        &self,
        from: &str,
        to: &str,
        author: UserId,
    ) -> Result<(), PriceBookError> {
        let (cottage, mut book) = self.load().await?;
        let from_name = channel_name(&book, from)?;
        let to_name = channel_name(&book, to)?;
        let before = snapshot(&(&book.defaults, &book.ranges));
        copy_prices(&mut book.defaults, from, to);
        for range in &mut book.ranges {
            copy_prices(&mut range.prices, from, to);
        }
        let description = format!("Prix de {from_name} recopiés sur {to_name}");
        self.save(cottage, &book, description, before, None, author)
            .await?;
        Ok(())
    }

    /// Prices and minimum stay of each night from `from` to `to` (excluded).
    ///
    /// # Errors
    ///
    /// Invalid or too long range, infrastructure.
    pub async fn nights(&self, from: Date, to: Date) -> Result<Vec<NightPrices>, PriceBookError> {
        if to <= from {
            return Err(PriceBookError::BadRange(
                "la fin doit être après le début".to_owned(),
            ));
        }
        if (to - from).whole_days() > i64::from(self.max_range_days) {
            return Err(PriceBookError::BadRange(format!(
                "{} jours au maximum",
                self.max_range_days
            )));
        }
        let (_, book) = self.load().await?;
        let mut nights = Vec::new();
        let mut night = from;
        while night < to {
            let range = book
                .ranges
                .iter()
                .find(|range| range.range.range().contains_night(night));
            nights.push(NightPrices {
                date: night,
                min_nights: range.map_or(book.settings.default_min_nights, |range| {
                    range.range.min_nights
                }),
                prices: book
                    .channels
                    .iter()
                    .map(|channel| {
                        let price = night_price(
                            range.and_then(|range| range.prices.get(&channel.code).copied()),
                            book.defaults.get(&channel.code).copied(),
                        );
                        (channel.code.clone(), price)
                    })
                    .collect(),
            });
            match night.next_day() {
                Some(next) => night = next,
                None => break,
            }
        }
        Ok(nights)
    }

    /// Latest changes first.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn history(&self, limit: u32) -> anyhow::Result<Vec<PriceChange>> {
        let cottage = self.cottages.default_cottage().await?.id;
        self.prices.history(cottage, limit).await
    }
}

fn channel_name(book: &PriceBook, code: &str) -> Result<String, PriceBookError> {
    book.channels
        .iter()
        .find(|channel| channel.code == code)
        .map(|channel| channel.name.clone())
        .ok_or_else(|| PricingError::UnknownChannel(code.to_owned()).into())
}

fn validated_prices(
    book: &PriceBook,
    prices: BTreeMap<ChannelCode, ChannelPrice>,
) -> Result<BTreeMap<ChannelCode, ChannelPrice>, PriceBookError> {
    let mut errors = Vec::new();
    let mut valid = BTreeMap::new();
    for (code, price) in prices {
        if !book.channels.iter().any(|channel| channel.code == code) {
            errors.push(PricingError::UnknownChannel(code));
            continue;
        }
        match price.validated() {
            Ok(price) => {
                valid.insert(code, price);
            }
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(valid)
    } else {
        Err(PriceBookError::Invalid(errors))
    }
}

fn copy_prices(prices: &mut BTreeMap<ChannelCode, ChannelPrice>, from: &str, to: &str) {
    match prices.get(from).copied() {
        Some(price) => prices.insert(to.to_owned(), price),
        None => prices.remove(to),
    };
}

/// Adds `new` to `ranges`, cutting the ranges it overlaps (domain rule `cut_ranges`).
fn apply_range(ranges: &mut Vec<PricedRange>, new: PricedRange) {
    let existing: Vec<Ranged<usize>> = ranges
        .iter()
        .enumerate()
        .map(|(index, range)| Ranged {
            id: index,
            range: range.range.clone(),
        })
        .collect();
    let cuts = cut_ranges(&existing, &new.range);
    let mut kept = Vec::with_capacity(ranges.len() + 2);
    for (index, range) in ranges.drain(..).enumerate() {
        match cuts.iter().find(|cut| cut.id == index) {
            None => kept.push(range),
            Some(cut) => {
                for (start, end) in &cut.pieces {
                    kept.push(PricedRange {
                        id: range.id,
                        range: RangeInput {
                            start: *start,
                            end: *end,
                            ..range.range.clone()
                        },
                        prices: range.prices.clone(),
                    });
                }
            }
        }
    }
    kept.push(new);
    kept.sort_by_key(|range| range.range.start);
    *ranges = kept;
}

#[cfg(test)]
#[path = "pricing_tests.rs"]
mod tests;
