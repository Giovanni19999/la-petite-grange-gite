//! `MariaDB` adapters of the calendar (F005): cottage and house rules, blocked periods.

use anyhow::Context;
use sqlx::{FromRow, MySqlPool};
use time::{Date, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::application::calendar::{
    BlockedPeriodRepository, Cottage, CottageId, CottageRepository, PeriodAction, PeriodChange,
    PeriodEvent, PeriodId, StoredPeriod,
};
use crate::application::health::BoxFuture;
use crate::domain::auth::UserId;
use crate::domain::calendar::{BlockedKind, BlockedPeriodInput, BookingHorizon};

fn to_db(value: OffsetDateTime) -> PrimitiveDateTime {
    let utc = value.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(utc.date(), utc.time())
}

/// The cottage and its house rules in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbCottages {
    pool: MySqlPool,
}

impl MariaDbCottages {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct CottageRow {
    id: u64,
    slug: String,
    name: String,
    reservable_months: u16,
    reservable_until: Option<Date>,
}

impl CottageRepository for MariaDbCottages {
    fn default_cottage(&self) -> BoxFuture<'_, anyhow::Result<Cottage>> {
        Box::pin(async move {
            let row: CottageRow = sqlx::query_as(
                "SELECT c.id, c.slug, c.name, r.reservable_months, r.reservable_until \
                 FROM cottages c JOIN booking_rules r ON r.cottage_id = c.id \
                 ORDER BY c.id LIMIT 1",
            )
            .fetch_one(&self.pool)
            .await
            .context("cannot read the cottage")?;
            Ok(Cottage {
                id: CottageId(row.id),
                slug: row.slug,
                name: row.name,
                horizon: BookingHorizon {
                    months: row.reservable_months,
                    until: row.reservable_until,
                },
            })
        })
    }
}

/// Blocked periods and their history in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbBlockedPeriods {
    pool: MySqlPool,
}

impl MariaDbBlockedPeriods {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct PeriodRow {
    id: u64,
    cottage_id: u64,
    start_date: Date,
    end_date: Option<Date>,
    kind: String,
    label: Option<String>,
    people: Option<u8>,
    reason: Option<String>,
    updated_at: PrimitiveDateTime,
}

impl PeriodRow {
    fn into_period(self) -> anyhow::Result<StoredPeriod> {
        Ok(StoredPeriod {
            id: PeriodId(self.id),
            cottage: CottageId(self.cottage_id),
            period: BlockedPeriodInput {
                start: self.start_date,
                end: self.end_date,
                kind: BlockedKind::parse(&self.kind)
                    .with_context(|| format!("unknown period kind {}", self.kind))?,
                label: self.label,
                people: self.people,
                reason: self.reason,
            },
            updated_at: self.updated_at.assume_utc(),
        })
    }
}

#[derive(FromRow)]
struct EventRow {
    period_id: u64,
    action: String,
    before_value: Option<String>,
    after_value: Option<String>,
    author_id: Option<u64>,
    author_email: Option<String>,
    at: PrimitiveDateTime,
}

fn parse_snapshot(json: Option<&str>) -> anyhow::Result<Option<BlockedPeriodInput>> {
    json.map(|value| {
        serde_json::from_str(value).context("invalid period snapshot stored in the database")
    })
    .transpose()
}

impl BlockedPeriodRepository for MariaDbBlockedPeriods {
    fn list(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<Vec<StoredPeriod>>> {
        Box::pin(async move {
            sqlx::query_as::<_, PeriodRow>(
                "SELECT id, cottage_id, start_date, end_date, kind, label, people, reason, \
                 updated_at FROM blocked_periods WHERE cottage_id = ? ORDER BY start_date, id",
            )
            .bind(cottage.0)
            .fetch_all(&self.pool)
            .await
            .context("cannot read blocked periods")?
            .into_iter()
            .map(PeriodRow::into_period)
            .collect()
        })
    }

    fn find(&self, id: PeriodId) -> BoxFuture<'_, anyhow::Result<Option<StoredPeriod>>> {
        Box::pin(async move {
            sqlx::query_as::<_, PeriodRow>(
                "SELECT id, cottage_id, start_date, end_date, kind, label, people, reason, \
                 updated_at FROM blocked_periods WHERE id = ?",
            )
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await
            .context("cannot read blocked period")?
            .map(PeriodRow::into_period)
            .transpose()
        })
    }

    fn save<'a>(&'a self, change: &'a PeriodChange) -> BoxFuture<'a, anyhow::Result<PeriodId>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start transaction")?;
            let id = match (change.id, &change.after) {
                (None, Some(after)) => {
                    let result = sqlx::query(
                        "INSERT INTO blocked_periods \
                         (cottage_id, start_date, end_date, kind, label, people, reason) \
                         VALUES (?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(change.cottage.0)
                    .bind(after.start)
                    .bind(after.end)
                    .bind(after.kind.as_str())
                    .bind(&after.label)
                    .bind(after.people)
                    .bind(&after.reason)
                    .execute(&mut *tx)
                    .await
                    .context("cannot create blocked period")?;
                    PeriodId(result.last_insert_id())
                }
                (Some(id), Some(after)) => {
                    sqlx::query(
                        "UPDATE blocked_periods SET start_date = ?, end_date = ?, kind = ?, \
                         label = ?, people = ?, reason = ? WHERE id = ?",
                    )
                    .bind(after.start)
                    .bind(after.end)
                    .bind(after.kind.as_str())
                    .bind(&after.label)
                    .bind(after.people)
                    .bind(&after.reason)
                    .bind(id.0)
                    .execute(&mut *tx)
                    .await
                    .context("cannot update blocked period")?;
                    id
                }
                (Some(id), None) => {
                    sqlx::query("DELETE FROM blocked_periods WHERE id = ?")
                        .bind(id.0)
                        .execute(&mut *tx)
                        .await
                        .context("cannot delete blocked period")?;
                    id
                }
                (None, None) => anyhow::bail!("nothing to save"),
            };
            let before = change
                .before
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?;
            let after = change
                .after
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?;
            sqlx::query(
                "INSERT INTO blocked_period_events \
                 (period_id, cottage_id, action, before_value, after_value, author_id, at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(id.0)
            .bind(change.cottage.0)
            .bind(change.action.as_str())
            .bind(before)
            .bind(after)
            .bind(change.author.0)
            .bind(to_db(change.at))
            .execute(&mut *tx)
            .await
            .context("cannot record the period history")?;
            tx.commit().await.context("cannot commit blocked period")?;
            Ok(id)
        })
    }

    fn history(
        &self,
        cottage: CottageId,
        limit: u32,
    ) -> BoxFuture<'_, anyhow::Result<Vec<PeriodEvent>>> {
        Box::pin(async move {
            sqlx::query_as::<_, EventRow>(
                "SELECT e.period_id, e.action, e.before_value, e.after_value, e.author_id, \
                 u.email AS author_email, e.at \
                 FROM blocked_period_events e LEFT JOIN users u ON u.id = e.author_id \
                 WHERE e.cottage_id = ? ORDER BY e.at DESC, e.id DESC LIMIT ?",
            )
            .bind(cottage.0)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .context("cannot read the period history")?
            .into_iter()
            .map(|row| {
                Ok(PeriodEvent {
                    period: PeriodId(row.period_id),
                    action: PeriodAction::parse(&row.action)
                        .with_context(|| format!("unknown action {}", row.action))?,
                    before: parse_snapshot(row.before_value.as_deref())?,
                    after: parse_snapshot(row.after_value.as_deref())?,
                    author: row.author_id.map(UserId),
                    author_email: row.author_email,
                    at: row.at.assume_utc(),
                })
            })
            .collect()
        })
    }
}
