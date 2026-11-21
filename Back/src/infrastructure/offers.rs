//! Promotions and options in `MariaDB` (F026); their history goes to the price history of F006.

use anyhow::Context;
use sqlx::{FromRow, MySql, MySqlPool, Transaction};
use time::{Date, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::application::calendar::CottageId;
use crate::application::health::BoxFuture;
use crate::application::offers::{
    OfferChange, OfferWrite, OffersRepository, StoredOption, StoredPromotion,
};
use crate::domain::offers::{Discount, NightBounds, OptionUnit, Promotion, StayOption};
use crate::domain::page::Localized;

/// Stored names of the discount kinds.
const PERCENT: &str = "percent";
const PER_STAY: &str = "per_stay";
const PER_NIGHT: &str = "per_night";

type Tx<'c> = Transaction<'c, MySql>;

/// Promotions and options in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbOffers {
    pool: MySqlPool,
}

impl MariaDbOffers {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct PromotionRow {
    id: u64,
    name_fr: String,
    name_en: String,
    kind: String,
    value: u64,
    min_nights: Option<u16>,
    max_nights: Option<u16>,
    valid_from: Option<Date>,
    valid_until: Option<Date>,
    active: bool,
}

impl TryFrom<PromotionRow> for StoredPromotion {
    type Error = anyhow::Error;

    fn try_from(row: PromotionRow) -> anyhow::Result<Self> {
        let discount = match row.kind.as_str() {
            PERCENT => Discount::Percent {
                bp: u16::try_from(row.value).context("invalid percent stored")?,
            },
            PER_STAY => Discount::PerStay { amount: row.value },
            PER_NIGHT => Discount::PerNight { amount: row.value },
            other => anyhow::bail!("unknown discount kind stored: {other}"),
        };
        Ok(Self {
            id: row.id,
            promotion: Promotion {
                name: Localized {
                    fr: row.name_fr,
                    en: row.name_en,
                },
                discount,
                bounds: NightBounds {
                    min_nights: row.min_nights,
                    max_nights: row.max_nights,
                },
                valid_from: row.valid_from,
                valid_until: row.valid_until,
                active: row.active,
            },
        })
    }
}

#[derive(FromRow)]
struct OptionRow {
    id: u64,
    name_fr: String,
    name_en: String,
    description_fr: String,
    description_en: String,
    unit: String,
    price: u64,
    min_nights: Option<u16>,
    max_nights: Option<u16>,
    position: u16,
    active: bool,
}

impl TryFrom<OptionRow> for StoredOption {
    type Error = anyhow::Error;

    fn try_from(row: OptionRow) -> anyhow::Result<Self> {
        Ok(Self {
            id: row.id,
            option: StayOption {
                name: Localized {
                    fr: row.name_fr,
                    en: row.name_en,
                },
                description: Localized {
                    fr: row.description_fr,
                    en: row.description_en,
                },
                unit: OptionUnit::parse(&row.unit).context("unknown option unit stored")?,
                price: row.price,
                bounds: NightBounds {
                    min_nights: row.min_nights,
                    max_nights: row.max_nights,
                },
                position: row.position,
                active: row.active,
            },
        })
    }
}

fn to_db(value: OffsetDateTime) -> PrimitiveDateTime {
    let utc = value.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(utc.date(), utc.time())
}

fn discount_columns(discount: Discount) -> (&'static str, u64) {
    match discount {
        Discount::Percent { bp } => (PERCENT, u64::from(bp)),
        Discount::PerStay { amount } => (PER_STAY, amount),
        Discount::PerNight { amount } => (PER_NIGHT, amount),
    }
}

/// History line in the price history of F006.
async fn record(tx: &mut Tx<'_>, cottage: CottageId, change: &OfferChange) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO rate_changes (cottage_id, description, author_id, at) VALUES (?, ?, ?, ?)",
    )
    .bind(cottage.0)
    .bind(&change.description)
    .bind(change.author.0)
    .bind(to_db(change.at))
    .execute(&mut **tx)
    .await
    .context("cannot record the change")?;
    Ok(())
}

async fn write_promotion(
    tx: &mut Tx<'_>,
    cottage: CottageId,
    write: &OfferWrite<'_, Promotion>,
) -> anyhow::Result<u64> {
    let Some(promotion) = write.value else {
        let id = write.id.context("nothing to delete")?;
        sqlx::query("DELETE FROM promotions WHERE id = ? AND cottage_id = ?")
            .bind(id)
            .bind(cottage.0)
            .execute(&mut **tx)
            .await
            .context("cannot delete the promotion")?;
        return Ok(id);
    };
    let (kind, value) = discount_columns(promotion.discount);
    if let Some(id) = write.id {
        sqlx::query(
            "UPDATE promotions SET name_fr = ?, name_en = ?, kind = ?, value = ?, min_nights = ?, \
             max_nights = ?, valid_from = ?, valid_until = ?, active = ? \
             WHERE id = ? AND cottage_id = ?",
        )
        .bind(&promotion.name.fr)
        .bind(&promotion.name.en)
        .bind(kind)
        .bind(value)
        .bind(promotion.bounds.min_nights)
        .bind(promotion.bounds.max_nights)
        .bind(promotion.valid_from)
        .bind(promotion.valid_until)
        .bind(promotion.active)
        .bind(id)
        .bind(cottage.0)
        .execute(&mut **tx)
        .await
        .context("cannot update the promotion")?;
        return Ok(id);
    }
    let done = sqlx::query(
        "INSERT INTO promotions (cottage_id, name_fr, name_en, kind, value, min_nights, \
         max_nights, valid_from, valid_until, active) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(cottage.0)
    .bind(&promotion.name.fr)
    .bind(&promotion.name.en)
    .bind(kind)
    .bind(value)
    .bind(promotion.bounds.min_nights)
    .bind(promotion.bounds.max_nights)
    .bind(promotion.valid_from)
    .bind(promotion.valid_until)
    .bind(promotion.active)
    .execute(&mut **tx)
    .await
    .context("cannot create the promotion")?;
    Ok(done.last_insert_id())
}

async fn write_option(
    tx: &mut Tx<'_>,
    cottage: CottageId,
    write: &OfferWrite<'_, StayOption>,
) -> anyhow::Result<u64> {
    let Some(option) = write.value else {
        let id = write.id.context("nothing to delete")?;
        sqlx::query("DELETE FROM stay_options WHERE id = ? AND cottage_id = ?")
            .bind(id)
            .bind(cottage.0)
            .execute(&mut **tx)
            .await
            .context("cannot delete the option")?;
        return Ok(id);
    };
    if let Some(id) = write.id {
        sqlx::query(
            "UPDATE stay_options SET name_fr = ?, name_en = ?, description_fr = ?, \
             description_en = ?, unit = ?, price = ?, min_nights = ?, max_nights = ?, \
             position = ?, active = ? WHERE id = ? AND cottage_id = ?",
        )
        .bind(&option.name.fr)
        .bind(&option.name.en)
        .bind(&option.description.fr)
        .bind(&option.description.en)
        .bind(option.unit.as_str())
        .bind(option.price)
        .bind(option.bounds.min_nights)
        .bind(option.bounds.max_nights)
        .bind(option.position)
        .bind(option.active)
        .bind(id)
        .bind(cottage.0)
        .execute(&mut **tx)
        .await
        .context("cannot update the option")?;
        return Ok(id);
    }
    let done = sqlx::query(
        "INSERT INTO stay_options (cottage_id, name_fr, name_en, description_fr, \
         description_en, unit, price, min_nights, max_nights, position, active) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(cottage.0)
    .bind(&option.name.fr)
    .bind(&option.name.en)
    .bind(&option.description.fr)
    .bind(&option.description.en)
    .bind(option.unit.as_str())
    .bind(option.price)
    .bind(option.bounds.min_nights)
    .bind(option.bounds.max_nights)
    .bind(option.position)
    .bind(option.active)
    .execute(&mut **tx)
    .await
    .context("cannot create the option")?;
    Ok(done.last_insert_id())
}

impl OffersRepository for MariaDbOffers {
    fn promotions(
        &self,
        cottage: CottageId,
    ) -> BoxFuture<'_, anyhow::Result<Vec<StoredPromotion>>> {
        Box::pin(async move {
            sqlx::query_as::<_, PromotionRow>(
                "SELECT id, name_fr, name_en, kind, value, min_nights, max_nights, valid_from, \
                 valid_until, active FROM promotions WHERE cottage_id = ? ORDER BY id",
            )
            .bind(cottage.0)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the promotions")?
            .into_iter()
            .map(StoredPromotion::try_from)
            .collect()
        })
    }

    fn options(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<Vec<StoredOption>>> {
        Box::pin(async move {
            sqlx::query_as::<_, OptionRow>(
                "SELECT id, name_fr, name_en, description_fr, description_en, unit, price, \
                 min_nights, max_nights, position, active FROM stay_options \
                 WHERE cottage_id = ? ORDER BY position, id",
            )
            .bind(cottage.0)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the options")?
            .into_iter()
            .map(StoredOption::try_from)
            .collect()
        })
    }

    fn save_promotion<'a>(
        &'a self,
        cottage: CottageId,
        write: OfferWrite<'a, Promotion>,
    ) -> BoxFuture<'a, anyhow::Result<u64>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start a transaction")?;
            let id = write_promotion(&mut tx, cottage, &write).await?;
            record(&mut tx, cottage, write.change).await?;
            tx.commit().await.context("cannot commit the promotion")?;
            Ok(id)
        })
    }

    fn save_option<'a>(
        &'a self,
        cottage: CottageId,
        write: OfferWrite<'a, StayOption>,
    ) -> BoxFuture<'a, anyhow::Result<u64>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start a transaction")?;
            let id = write_option(&mut tx, cottage, &write).await?;
            record(&mut tx, cottage, write.change).await?;
            tx.commit().await.context("cannot commit the option")?;
            Ok(id)
        })
    }
}
