//! `MariaDB` adapter of the page repository (F002).

use anyhow::Context;
use sqlx::MySqlPool;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::application::health::BoxFuture;
use crate::application::pages::{PageId, PageRepository, PageVersion, StoredPage};
use crate::domain::auth::UserId;
use crate::domain::page::{Lang, PageContent};

fn to_db(value: OffsetDateTime) -> PrimitiveDateTime {
    let utc = value.to_offset(UtcOffset::UTC);
    PrimitiveDateTime::new(utc.date(), utc.time())
}

fn parse_content(json: &str) -> anyhow::Result<PageContent> {
    serde_json::from_str(json).context("invalid page content stored in the database")
}

fn to_json(content: &PageContent) -> anyhow::Result<String> {
    serde_json::to_string(content).context("cannot serialise page content")
}

/// Raw row of the `pages` table.
struct PageRow {
    id: u64,
    is_home: i8,
    draft: String,
    draft_version: u32,
    published: Option<String>,
    published_at: Option<PrimitiveDateTime>,
}

impl PageRow {
    fn into_page(self) -> anyhow::Result<StoredPage> {
        Ok(StoredPage {
            id: PageId(self.id),
            is_home: self.is_home != 0,
            draft: parse_content(&self.draft)?,
            draft_version: self.draft_version,
            published: self.published.as_deref().map(parse_content).transpose()?,
            published_at: self.published_at.map(PrimitiveDateTime::assume_utc),
        })
    }
}

/// Pages in `MariaDB`.
#[derive(Debug, Clone)]
pub struct MariaDbPages {
    pool: MySqlPool,
}

impl MariaDbPages {
    #[must_use]
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl PageRepository for MariaDbPages {
    fn all(&self) -> BoxFuture<'_, anyhow::Result<Vec<StoredPage>>> {
        Box::pin(async move {
            sqlx::query_as!(
                PageRow,
                "SELECT id, is_home, draft, draft_version, published, published_at FROM pages"
            )
            .fetch_all(&self.pool)
            .await
            .context("cannot read pages")?
            .into_iter()
            .map(PageRow::into_page)
            .collect()
        })
    }

    fn find(&self, id: PageId) -> BoxFuture<'_, anyhow::Result<Option<StoredPage>>> {
        Box::pin(async move {
            sqlx::query_as!(
                PageRow,
                "SELECT id, is_home, draft, draft_version, published, published_at \
                 FROM pages WHERE id = ?",
                id.0
            )
            .fetch_optional(&self.pool)
            .await
            .context("cannot read page")?
            .map(PageRow::into_page)
            .transpose()
        })
    }

    fn find_published<'a>(
        &'a self,
        lang: Lang,
        slug: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<Option<StoredPage>>> {
        Box::pin(async move {
            let row = match lang {
                Lang::Fr => {
                    sqlx::query_as!(
                        PageRow,
                        "SELECT id, is_home, draft, draft_version, published, published_at \
                     FROM pages WHERE published_slug_fr = ?",
                        slug
                    )
                    .fetch_optional(&self.pool)
                    .await
                }
                Lang::En => {
                    sqlx::query_as!(
                        PageRow,
                        "SELECT id, is_home, draft, draft_version, published, published_at \
                     FROM pages WHERE published_slug_en = ?",
                        slug
                    )
                    .fetch_optional(&self.pool)
                    .await
                }
            };
            row.context("cannot read page")?
                .map(PageRow::into_page)
                .transpose()
        })
    }

    fn home(&self) -> BoxFuture<'_, anyhow::Result<Option<StoredPage>>> {
        Box::pin(async move {
            sqlx::query_as!(
                PageRow,
                "SELECT id, is_home, draft, draft_version, published, published_at \
                 FROM pages WHERE is_home = TRUE ORDER BY id LIMIT 1"
            )
            .fetch_optional(&self.pool)
            .await
            .context("cannot read the home page")?
            .map(PageRow::into_page)
            .transpose()
        })
    }

    fn create<'a>(&'a self, content: &'a PageContent) -> BoxFuture<'a, anyhow::Result<PageId>> {
        Box::pin(async move {
            let draft = to_json(content)?;
            let done = sqlx::query!(
                "INSERT INTO pages (is_home, draft, draft_version) VALUES (FALSE, ?, 1)",
                draft
            )
            .execute(&self.pool)
            .await
            .context("cannot create page")?;
            Ok(PageId(done.last_insert_id()))
        })
    }

    fn save_draft<'a>(
        &'a self,
        id: PageId,
        expected_version: u32,
        content: &'a PageContent,
    ) -> BoxFuture<'a, anyhow::Result<Option<u32>>> {
        Box::pin(async move {
            let draft = to_json(content)?;
            let done = sqlx::query!(
                "UPDATE pages SET draft = ?, draft_version = draft_version + 1 \
                 WHERE id = ? AND draft_version = ?",
                draft,
                id.0,
                expected_version
            )
            .execute(&self.pool)
            .await
            .context("cannot save draft")?;
            Ok((done.rows_affected() == 1).then_some(expected_version + 1))
        })
    }

    fn publish(
        &self,
        id: PageId,
        expected_version: u32,
        author: UserId,
        at: OffsetDateTime,
    ) -> BoxFuture<'_, anyhow::Result<bool>> {
        Box::pin(async move {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("cannot start transaction")?;
            let draft = sqlx::query_scalar!(
                "SELECT draft FROM pages WHERE id = ? AND draft_version = ? FOR UPDATE",
                id.0,
                expected_version
            )
            .fetch_optional(&mut *tx)
            .await
            .context("cannot lock page")?;
            let Some(draft) = draft else {
                return Ok(false);
            };
            let content = parse_content(&draft)?;
            let at = to_db(at);
            sqlx::query!(
                "UPDATE pages SET published = draft, published_at = ?, \
                 published_slug_fr = ?, published_slug_en = ? WHERE id = ?",
                at,
                content.settings.slug.fr,
                content.settings.slug.en,
                id.0
            )
            .execute(&mut *tx)
            .await
            .context("cannot publish page")?;
            sqlx::query!(
                "INSERT INTO page_versions (page_id, number, content, published_at, author_id) \
                 SELECT ?, COALESCE(MAX(number), 0) + 1, ?, ?, ? \
                 FROM page_versions WHERE page_id = ?",
                id.0,
                draft,
                at,
                author.0,
                id.0
            )
            .execute(&mut *tx)
            .await
            .context("cannot record page version")?;
            tx.commit().await.context("cannot commit publication")?;
            Ok(true)
        })
    }

    fn versions(&self, id: PageId) -> BoxFuture<'_, anyhow::Result<Vec<PageVersion>>> {
        Box::pin(async move {
            let rows = sqlx::query!(
                "SELECT number, published_at, author_id FROM page_versions WHERE page_id = ?",
                id.0
            )
            .fetch_all(&self.pool)
            .await
            .context("cannot read page versions")?;
            Ok(rows
                .into_iter()
                .map(|row| PageVersion {
                    number: row.number,
                    published_at: row.published_at.assume_utc(),
                    author: row.author_id.map(UserId),
                })
                .collect())
        })
    }

    fn version_content(
        &self,
        id: PageId,
        number: u32,
    ) -> BoxFuture<'_, anyhow::Result<Option<PageContent>>> {
        Box::pin(async move {
            sqlx::query_scalar!(
                "SELECT content FROM page_versions WHERE page_id = ? AND number = ?",
                id.0,
                number
            )
            .fetch_optional(&self.pool)
            .await
            .context("cannot read page version")?
            .as_deref()
            .map(parse_content)
            .transpose()
        })
    }

    fn delete(&self, id: PageId) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM pages WHERE id = ? AND is_home = FALSE", id.0)
                .execute(&self.pool)
                .await
                .context("cannot delete page")?;
            Ok(())
        })
    }
}
