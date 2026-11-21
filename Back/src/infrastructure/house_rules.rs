//! House rules in `MariaDB` (F004): the `booking_rules` row of the cottage and its history.

use anyhow::Context;
use sqlx::{FromRow, MySqlPool};
use time::{Date, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::application::calendar::CottageId;
use crate::application::health::BoxFuture;
use crate::application::house_rules::{HouseRulesRepository, RuleChange};
use crate::domain::auth::UserId;
use crate::domain::calendar::BookingHorizon;
use crate::domain::house_rules::{Deposit, HouseRules, TouristTax, Weekdays};

/// Stored names of the deposit modes.
const PERCENTAGE: &str = "percentage";
const FIXED: &str = "fixed";

/// House rules in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbHouseRules {
    pool: MySqlPool,
}

impl MariaDbHouseRules {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct RulesRow {
    reservable_months: u16,
    reservable_until: Option<Date>,
    capacity: u8,
    min_delay_days: u16,
    arrival_weekdays: u8,
    departure_weekdays: u8,
    deposit_mode: String,
    deposit_percent: u8,
    deposit_fixed: u64,
    balance_days: u16,
    tourist_tax_per_adult_night: u64,
    tourist_tax_additional_bp: u16,
    tourist_tax_exempt_under: u8,
}

impl TryFrom<RulesRow> for HouseRules {
    type Error = anyhow::Error;

    fn try_from(row: RulesRow) -> anyhow::Result<Self> {
        let weekdays = |bits| Weekdays::from_bits(bits).context("invalid weekdays stored");
        Ok(Self {
            capacity: row.capacity,
            min_delay_days: row.min_delay_days,
            arrival_weekdays: weekdays(row.arrival_weekdays)?,
            departure_weekdays: weekdays(row.departure_weekdays)?,
            deposit: match row.deposit_mode.as_str() {
                PERCENTAGE => Deposit::Percentage {
                    percent: row.deposit_percent,
                },
                FIXED => Deposit::Fixed {
                    amount: row.deposit_fixed,
                },
                other => anyhow::bail!("unknown deposit mode stored: {other}"),
            },
            balance_days: row.balance_days,
            tourist_tax: TouristTax {
                per_adult_night: row.tourist_tax_per_adult_night,
                additional_bp: row.tourist_tax_additional_bp,
                exempt_under: row.tourist_tax_exempt_under,
            },
            horizon: BookingHorizon {
                months: row.reservable_months,
                until: row.reservable_until,
            },
        })
    }
}

#[derive(FromRow)]
struct ChangeRow {
    description: String,
    before_value: Option<String>,
    after_value: String,
    author_id: Option<u64>,
    author_email: Option<String>,
    at: PrimitiveDateTime,
}

fn to_db(value: OffsetDateTime) -> PrimitiveDateTime {
    let utc = value.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(utc.date(), utc.time())
}

fn parse_rules(json: &str) -> anyhow::Result<HouseRules> {
    serde_json::from_str(json).context("invalid house rules history stored")
}

impl HouseRulesRepository for MariaDbHouseRules {
    fn load(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<HouseRules>> {
        Box::pin(async move {
            let row: RulesRow = sqlx::query_as(
                "SELECT reservable_months, reservable_until, capacity, min_delay_days, \
                 arrival_weekdays, departure_weekdays, deposit_mode, deposit_percent, \
                 deposit_fixed, balance_days, tourist_tax_per_adult_night, \
                 tourist_tax_additional_bp, tourist_tax_exempt_under \
                 FROM booking_rules WHERE cottage_id = ?",
            )
            .bind(cottage.0)
            .fetch_one(&self.pool)
            .await
            .context("cannot read the house rules")?;
            HouseRules::try_from(row)
        })
    }

    fn save<'a>(
        &'a self,
        cottage: CottageId,
        change: &'a RuleChange,
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let rules = &change.after;
            let (mode, percent, fixed) = match rules.deposit {
                // The other mode keeps its stored value for when the owners switch back.
                Deposit::Percentage { percent } => (PERCENTAGE, Some(percent), None),
                Deposit::Fixed { amount } => (FIXED, None, Some(amount)),
            };
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start a transaction")?;
            sqlx::query(
                "UPDATE booking_rules SET reservable_months = ?, reservable_until = ?, \
                 capacity = ?, min_delay_days = ?, arrival_weekdays = ?, departure_weekdays = ?, \
                 deposit_mode = ?, deposit_percent = COALESCE(?, deposit_percent), \
                 deposit_fixed = COALESCE(?, deposit_fixed), balance_days = ?, \
                 tourist_tax_per_adult_night = ?, tourist_tax_additional_bp = ?, \
                 tourist_tax_exempt_under = ? WHERE cottage_id = ?",
            )
            .bind(rules.horizon.months)
            .bind(rules.horizon.until)
            .bind(rules.capacity)
            .bind(rules.min_delay_days)
            .bind(rules.arrival_weekdays.bits())
            .bind(rules.departure_weekdays.bits())
            .bind(mode)
            .bind(percent)
            .bind(fixed)
            .bind(rules.balance_days)
            .bind(rules.tourist_tax.per_adult_night)
            .bind(rules.tourist_tax.additional_bp)
            .bind(rules.tourist_tax.exempt_under)
            .bind(cottage.0)
            .execute(&mut *tx)
            .await
            .context("cannot save the house rules")?;
            sqlx::query(
                "INSERT INTO house_rule_changes \
                 (cottage_id, description, before_value, after_value, author_id, at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(cottage.0)
            .bind(&change.description)
            .bind(
                change
                    .before
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?,
            )
            .bind(serde_json::to_string(&change.after)?)
            .bind(change.author.map(|author| author.0))
            .bind(to_db(change.at))
            .execute(&mut *tx)
            .await
            .context("cannot record the house rules change")?;
            tx.commit().await.context("cannot commit the house rules")?;
            Ok(())
        })
    }

    fn history(
        &self,
        cottage: CottageId,
        limit: u32,
    ) -> BoxFuture<'_, anyhow::Result<Vec<RuleChange>>> {
        Box::pin(async move {
            sqlx::query_as::<_, ChangeRow>(
                "SELECT h.description, h.before_value, h.after_value, h.author_id, \
                 u.email AS author_email, h.at \
                 FROM house_rule_changes h LEFT JOIN users u ON u.id = h.author_id \
                 WHERE h.cottage_id = ? ORDER BY h.at DESC, h.id DESC LIMIT ?",
            )
            .bind(cottage.0)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the house rules history")?
            .into_iter()
            .map(|row| {
                Ok(RuleChange {
                    description: row.description,
                    before: row.before_value.as_deref().map(parse_rules).transpose()?,
                    after: parse_rules(&row.after_value)?,
                    author: row.author_id.map(UserId),
                    author_email: row.author_email,
                    at: row.at.assume_utc(),
                })
            })
            .collect()
        })
    }
}
