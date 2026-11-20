//! `MariaDB` adapter of the media repository (F003).

use anyhow::Context;
use sqlx::{FromRow, MySql, MySqlPool, QueryBuilder};
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::application::health::BoxFuture;
use crate::application::media::{MediaRepository, NewMedia, StoredMedia};
use crate::domain::media::{ContentHash, FocalPoint, MediaDetails};
use crate::domain::page::{Localized, MediaId};

const COLUMNS: &str = "id, content_hash, original_extension, width, height, widths, \
     alt_fr, alt_en, caption_fr, caption_en, focal_x, focal_y, created_at";

fn to_db(value: OffsetDateTime) -> PrimitiveDateTime {
    let utc = value.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(utc.date(), utc.time())
}

/// Raw row of the `media` table.
#[derive(FromRow)]
struct MediaRow {
    id: u64,
    content_hash: String,
    original_extension: String,
    width: u32,
    height: u32,
    widths: String,
    alt_fr: String,
    alt_en: String,
    caption_fr: String,
    caption_en: String,
    focal_x: u8,
    focal_y: u8,
    created_at: PrimitiveDateTime,
}

impl MediaRow {
    fn into_media(self) -> anyhow::Result<StoredMedia> {
        Ok(StoredMedia {
            id: MediaId(self.id),
            hash: ContentHash::parse(&self.content_hash)
                .context("invalid content hash stored in the database")?,
            original_extension: self.original_extension,
            width: self.width,
            height: self.height,
            widths: serde_json::from_str(&self.widths)
                .context("invalid media widths stored in the database")?,
            details: MediaDetails {
                alt: Localized {
                    fr: self.alt_fr,
                    en: self.alt_en,
                },
                caption: Localized {
                    fr: self.caption_fr,
                    en: self.caption_en,
                },
                focal: FocalPoint {
                    x: self.focal_x,
                    y: self.focal_y,
                },
            },
            created_at: self.created_at.assume_utc(),
        })
    }
}

fn into_media(rows: Vec<MediaRow>) -> anyhow::Result<Vec<StoredMedia>> {
    rows.into_iter().map(MediaRow::into_media).collect()
}

/// Media records in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbMedia {
    pool: MySqlPool,
}

impl MariaDbMedia {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn select_where(
        &self,
        condition: impl FnOnce(&mut QueryBuilder<MySql>),
    ) -> anyhow::Result<Vec<StoredMedia>> {
        let mut query = QueryBuilder::new(format!("SELECT {COLUMNS} FROM media "));
        condition(&mut query);
        let rows = query
            .build_query_as::<MediaRow>()
            .fetch_all(&self.pool)
            .await
            .context("cannot read media")?;
        into_media(rows)
    }
}

impl MediaRepository for MariaDbMedia {
    fn all(&self) -> BoxFuture<'_, anyhow::Result<Vec<StoredMedia>>> {
        Box::pin(self.select_where(|query| {
            query.push("ORDER BY created_at DESC, id DESC");
        }))
    }

    fn find(&self, id: MediaId) -> BoxFuture<'_, anyhow::Result<Option<StoredMedia>>> {
        Box::pin(async move {
            Ok(self
                .select_where(|query| {
                    query.push("WHERE id = ").push_bind(id.0);
                })
                .await?
                .pop())
        })
    }

    fn find_by_hash<'a>(
        &'a self,
        hash: &'a ContentHash,
    ) -> BoxFuture<'a, anyhow::Result<Option<StoredMedia>>> {
        Box::pin(async move {
            Ok(self
                .select_where(|query| {
                    query
                        .push("WHERE content_hash = ")
                        .push_bind(hash.as_str().to_owned());
                })
                .await?
                .pop())
        })
    }

    fn find_many<'a>(
        &'a self,
        ids: &'a [MediaId],
    ) -> BoxFuture<'a, anyhow::Result<Vec<StoredMedia>>> {
        Box::pin(async move {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            self.select_where(|query| {
                query.push("WHERE id IN (");
                let mut separated = query.separated(", ");
                for id in ids {
                    separated.push_bind(id.0);
                }
                separated.push_unseparated(")");
            })
            .await
        })
    }

    fn create<'a>(&'a self, media: &'a NewMedia) -> BoxFuture<'a, anyhow::Result<MediaId>> {
        Box::pin(async move {
            let widths = serde_json::to_string(&media.widths)?;
            let result = sqlx::query(
                "INSERT INTO media \
                 (content_hash, original_extension, width, height, widths, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(media.hash.as_str())
            .bind(&media.original_extension)
            .bind(media.width)
            .bind(media.height)
            .bind(widths)
            .bind(to_db(media.created_at))
            .execute(&self.pool)
            .await
            .context("cannot record media")?;
            Ok(MediaId(result.last_insert_id()))
        })
    }

    fn update_details<'a>(
        &'a self,
        id: MediaId,
        details: &'a MediaDetails,
    ) -> BoxFuture<'a, anyhow::Result<bool>> {
        Box::pin(async move {
            let exists = sqlx::query("SELECT 1 FROM media WHERE id = ?")
                .bind(id.0)
                .fetch_optional(&self.pool)
                .await
                .context("cannot read media")?
                .is_some();
            if !exists {
                return Ok(false);
            }
            sqlx::query(
                "UPDATE media SET alt_fr = ?, alt_en = ?, caption_fr = ?, caption_en = ?, \
                 focal_x = ?, focal_y = ? WHERE id = ?",
            )
            .bind(&details.alt.fr)
            .bind(&details.alt.en)
            .bind(&details.caption.fr)
            .bind(&details.caption.en)
            .bind(details.focal.x)
            .bind(details.focal.y)
            .bind(id.0)
            .execute(&self.pool)
            .await
            .context("cannot update media")?;
            Ok(true)
        })
    }

    fn delete(&self, id: MediaId) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            sqlx::query("DELETE FROM media WHERE id = ?")
                .bind(id.0)
                .execute(&self.pool)
                .await
                .context("cannot delete media")?;
            Ok(())
        })
    }
}
