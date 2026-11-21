//! `MariaDB` adapter of the price book (F006).

use std::collections::BTreeMap;

use anyhow::Context;
use sqlx::{FromRow, MySql, MySqlPool, Transaction};
use time::{Date, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::application::calendar::CottageId;
use crate::application::health::BoxFuture;
use crate::application::pricing::{
    Channel, PriceBook, PriceChange, PricedRange, PricingRepository, PricingSettings,
};
use crate::domain::auth::UserId;
use crate::domain::pricing::{ChannelPrice, ChannelRates, RangeInput};

fn to_db(value: OffsetDateTime) -> PrimitiveDateTime {
    let utc = value.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(utc.date(), utc.time())
}

/// The price book in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbPricing {
    pool: MySqlPool,
}

impl MariaDbPricing {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct ChannelRow {
    code: String,
    name: String,
    commission_bp: u16,
    payment_fees_bp: u16,
    vat_on_commission: bool,
}

#[derive(FromRow)]
struct SettingsRow {
    cleaning_fee: u64,
    default_min_nights: u8,
    commission_alert_points: u8,
}

#[derive(FromRow)]
struct RangeRow {
    id: u64,
    start_date: Date,
    end_date: Date,
    label: Option<String>,
    min_nights: u8,
}

#[derive(FromRow)]
struct PriceRow {
    rate_period_id: Option<u64>,
    code: String,
    listed: u64,
    net: u64,
}

#[derive(FromRow)]
struct ChangeRow {
    description: String,
    before_value: Option<String>,
    after_value: Option<String>,
    author_id: Option<u64>,
    author_email: Option<String>,
    at: PrimitiveDateTime,
}

fn parse_json(value: Option<&str>) -> anyhow::Result<Option<serde_json::Value>> {
    value
        .map(|json| serde_json::from_str(json).context("invalid price history stored"))
        .transpose()
}

impl PricingRepository for MariaDbPricing {
    fn load(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<PriceBook>> {
        Box::pin(async move {
            let channels: Vec<ChannelRow> = sqlx::query_as(
                "SELECT code, name, commission_bp, payment_fees_bp, vat_on_commission \
                 FROM channels ORDER BY position, id",
            )
            .fetch_all(&self.pool)
            .await
            .context("cannot read channels")?;
            let settings: SettingsRow = sqlx::query_as(
                "SELECT c.cleaning_fee, c.default_min_nights, r.commission_alert_points \
                 FROM cottages c JOIN booking_rules r ON r.cottage_id = c.id WHERE c.id = ?",
            )
            .bind(cottage.0)
            .fetch_one(&self.pool)
            .await
            .context("cannot read price settings")?;
            let ranges: Vec<RangeRow> = sqlx::query_as(
                "SELECT id, start_date, end_date, label, min_nights FROM rate_periods \
                 WHERE cottage_id = ? ORDER BY start_date",
            )
            .bind(cottage.0)
            .fetch_all(&self.pool)
            .await
            .context("cannot read price ranges")?;
            let prices: Vec<PriceRow> = sqlx::query_as(
                "SELECT p.rate_period_id, ch.code, p.listed, p.net FROM channel_prices p \
                 JOIN channels ch ON ch.id = p.channel_id WHERE p.cottage_id = ?",
            )
            .bind(cottage.0)
            .fetch_all(&self.pool)
            .await
            .context("cannot read channel prices")?;

            let mut defaults = BTreeMap::new();
            let mut by_range: BTreeMap<u64, BTreeMap<String, ChannelPrice>> = BTreeMap::new();
            for row in prices {
                let price = ChannelPrice {
                    listed: row.listed,
                    net: row.net,
                };
                match row.rate_period_id {
                    None => {
                        defaults.insert(row.code, price);
                    }
                    Some(range) => {
                        by_range.entry(range).or_default().insert(row.code, price);
                    }
                }
            }
            Ok(PriceBook {
                channels: channels
                    .into_iter()
                    .map(|row| Channel {
                        code: row.code,
                        name: row.name,
                        rates: ChannelRates {
                            commission_bp: row.commission_bp,
                            payment_fees_bp: row.payment_fees_bp,
                            vat_on_commission: row.vat_on_commission,
                        },
                    })
                    .collect(),
                settings: PricingSettings {
                    cleaning_fee: settings.cleaning_fee,
                    default_min_nights: settings.default_min_nights,
                    commission_alert_points: settings.commission_alert_points,
                },
                defaults,
                ranges: ranges
                    .into_iter()
                    .map(|row| PricedRange {
                        id: row.id,
                        range: RangeInput {
                            start: row.start_date,
                            end: row.end_date,
                            label: row.label,
                            min_nights: row.min_nights,
                        },
                        prices: by_range.remove(&row.id).unwrap_or_default(),
                    })
                    .collect(),
            })
        })
    }

    fn save<'a>(
        &'a self,
        cottage: CottageId,
        book: &'a PriceBook,
        change: &'a PriceChange,
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start transaction")?;
            // One save at a time per cottage.
            sqlx::query("SELECT id FROM cottages WHERE id = ? FOR UPDATE")
                .bind(cottage.0)
                .execute(&mut *tx)
                .await
                .context("cannot lock the cottage")?;
            let channel_ids = save_settings_and_channels(&mut tx, cottage, book).await?;
            save_prices(&mut tx, cottage, book, &channel_ids).await?;
            record_change(&mut tx, cottage, change).await?;
            tx.commit().await.context("cannot commit prices")?;
            Ok(())
        })
    }

    fn history(
        &self,
        cottage: CottageId,
        limit: u32,
    ) -> BoxFuture<'_, anyhow::Result<Vec<PriceChange>>> {
        Box::pin(async move {
            sqlx::query_as::<_, ChangeRow>(
                "SELECT r.description, r.before_value, r.after_value, r.author_id, \
                 u.email AS author_email, r.at \
                 FROM rate_changes r LEFT JOIN users u ON u.id = r.author_id \
                 WHERE r.cottage_id = ? ORDER BY r.at DESC, r.id DESC LIMIT ?",
            )
            .bind(cottage.0)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the price history")?
            .into_iter()
            .map(|row| {
                Ok(PriceChange {
                    description: row.description,
                    before: parse_json(row.before_value.as_deref())?,
                    after: parse_json(row.after_value.as_deref())?,
                    author: row.author_id.map(UserId),
                    author_email: row.author_email,
                    at: row.at.assume_utc(),
                })
            })
            .collect()
        })
    }
}

type Tx<'c> = Transaction<'c, MySql>;

/// Cottage settings and channel rates; returns the id of each channel code.
async fn save_settings_and_channels(
    tx: &mut Tx<'_>,
    cottage: CottageId,
    book: &PriceBook,
) -> anyhow::Result<BTreeMap<String, u64>> {
    sqlx::query("UPDATE cottages SET cleaning_fee = ?, default_min_nights = ? WHERE id = ?")
        .bind(book.settings.cleaning_fee)
        .bind(book.settings.default_min_nights)
        .bind(cottage.0)
        .execute(&mut **tx)
        .await
        .context("cannot save price settings")?;
    sqlx::query("UPDATE booking_rules SET commission_alert_points = ? WHERE cottage_id = ?")
        .bind(book.settings.commission_alert_points)
        .bind(cottage.0)
        .execute(&mut **tx)
        .await
        .context("cannot save the alert threshold")?;
    let mut channel_ids = BTreeMap::new();
    for channel in &book.channels {
        sqlx::query(
            "UPDATE channels SET commission_bp = ?, payment_fees_bp = ?, \
             vat_on_commission = ? WHERE code = ?",
        )
        .bind(channel.rates.commission_bp)
        .bind(channel.rates.payment_fees_bp)
        .bind(channel.rates.vat_on_commission)
        .bind(&channel.code)
        .execute(&mut **tx)
        .await
        .context("cannot save channel rates")?;
        let id: (u64,) = sqlx::query_as("SELECT id FROM channels WHERE code = ?")
            .bind(&channel.code)
            .fetch_one(&mut **tx)
            .await
            .context("cannot read channel")?;
        channel_ids.insert(channel.code.clone(), id.0);
    }
    Ok(channel_ids)
}

async fn insert_price(
    tx: &mut Tx<'_>,
    cottage: CottageId,
    range: Option<u64>,
    channel: u64,
    price: ChannelPrice,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO channel_prices (cottage_id, rate_period_id, channel_id, listed, net) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(cottage.0)
    .bind(range)
    .bind(channel)
    .bind(price.listed)
    .bind(price.net)
    .execute(&mut **tx)
    .await
    .context("cannot save a price")?;
    Ok(())
}

/// Replaces the ranges and prices (prices of ranges go with them: ON DELETE CASCADE).
async fn save_prices(
    tx: &mut Tx<'_>,
    cottage: CottageId,
    book: &PriceBook,
    channel_ids: &BTreeMap<String, u64>,
) -> anyhow::Result<()> {
    let channel = |code: &str| {
        channel_ids
            .get(code)
            .copied()
            .with_context(|| format!("unknown channel {code}"))
    };
    sqlx::query("DELETE FROM channel_prices WHERE cottage_id = ?")
        .bind(cottage.0)
        .execute(&mut **tx)
        .await
        .context("cannot clear prices")?;
    sqlx::query("DELETE FROM rate_periods WHERE cottage_id = ?")
        .bind(cottage.0)
        .execute(&mut **tx)
        .await
        .context("cannot clear ranges")?;
    for (code, price) in &book.defaults {
        insert_price(tx, cottage, None, channel(code)?, *price).await?;
    }
    for range in &book.ranges {
        let inserted = sqlx::query(
            "INSERT INTO rate_periods (cottage_id, start_date, end_date, label, min_nights) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(cottage.0)
        .bind(range.range.start)
        .bind(range.range.end)
        .bind(&range.range.label)
        .bind(range.range.min_nights)
        .execute(&mut **tx)
        .await
        .context("cannot save a range")?;
        let range_id = inserted.last_insert_id();
        for (code, price) in &range.prices {
            insert_price(tx, cottage, Some(range_id), channel(code)?, *price).await?;
        }
    }
    Ok(())
}

async fn record_change(
    tx: &mut Tx<'_>,
    cottage: CottageId,
    change: &PriceChange,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO rate_changes \
         (cottage_id, description, before_value, after_value, author_id, at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(cottage.0)
    .bind(&change.description)
    .bind(change.before.as_ref().map(ToString::to_string))
    .bind(change.after.as_ref().map(ToString::to_string))
    .bind(change.author.map(|author| author.0))
    .bind(to_db(change.at))
    .execute(&mut **tx)
    .await
    .context("cannot record the price change")?;
    Ok(())
}
