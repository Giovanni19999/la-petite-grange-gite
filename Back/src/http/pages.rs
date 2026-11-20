//! Page routes (F002, ADR-0013): public reading in one language, admin editing.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::app_state::AppState;
use crate::application::pages::{MenuEntry, PageError, PageId, PageVersion, StoredPage};
use crate::domain::page::{Block, BlockKind, ChangeSummary, Lang, MediaId, PageContent};
use crate::http::auth::AdminUser;
use crate::http::error::ApiError;
use crate::http::media::{ImageMap, public_image};

/// Public routes, mounted under `/api/pages`.
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/menu", get(menu))
        .route("/home", get(home))
        .route("/by-slug/{slug}", get(by_slug))
}

/// Admin routes, mounted under `/api/admin/pages`; every handler requires [`AdminUser`].
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(admin_list).post(admin_create))
        .route("/{id}", get(admin_get).delete(admin_delete))
        .route("/{id}/draft", put(admin_save_draft))
        .route("/{id}/changes", get(admin_changes))
        .route("/{id}/publish", post(admin_publish))
        .route("/{id}/versions", get(admin_versions))
        .route("/{id}/versions/{number}/restore", post(admin_restore))
}

impl From<PageError> for ApiError {
    fn from(error: PageError) -> Self {
        match error {
            PageError::NotFound | PageError::VersionNotFound => Self::NotFound,
            PageError::Invalid(errors) => {
                Self::Invalid(errors.iter().map(ToString::to_string).collect())
            }
            PageError::VersionConflict { .. } => Self::Conflict {
                code: "version_conflict",
                message: error.to_string(),
            },
            PageError::SlugTaken { .. } => Self::Conflict {
                code: "slug_taken",
                message: error.to_string(),
            },
            PageError::HomeCannotBeDeleted => Self::Conflict {
                code: "home_page",
                message: error.to_string(),
            },
            PageError::UnknownMedia { .. } => Self::Invalid(vec![error.to_string()]),
            PageError::Internal(error) => Self::Internal(error),
        }
    }
}

fn json_body<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(value)| value)
        .map_err(|rejection| ApiError::BadRequest(rejection.body_text()))
}

fn timestamp(value: OffsetDateTime) -> String {
    value.format(&Rfc3339).unwrap_or_default()
}

// ----- Public -----

#[derive(Debug, Deserialize)]
pub struct LangQuery {
    #[serde(default = "default_lang")]
    lang: Lang,
}

fn default_lang() -> Lang {
    Lang::Fr
}

/// Public URL of a page in a language (ADR-0013): French at the root, English under `/en`.
fn public_path(content: &PageContent, is_home: bool, lang: Lang) -> String {
    match (lang, is_home) {
        (Lang::Fr, true) => "/".to_owned(),
        (Lang::En, true) => "/en".to_owned(),
        (Lang::Fr, false) => format!("/{}", content.settings.slug.fr),
        (Lang::En, false) => format!("/en/{}", content.settings.slug.en),
    }
}

#[derive(Debug, Serialize)]
pub struct MenuItem {
    title: String,
    path: String,
}

async fn menu(
    State(state): State<AppState>,
    Query(query): Query<LangQuery>,
) -> Result<Json<Vec<MenuItem>>, ApiError> {
    let entries = state.pages.menu().await?;
    Ok(Json(
        entries
            .iter()
            .map(|MenuEntry { is_home, content }| MenuItem {
                title: content.settings.title.get(query.lang).to_owned(),
                path: public_path(content, *is_home, query.lang),
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub struct PublicPageResponse {
    lang: Lang,
    title: String,
    meta_description: String,
    /// URL of the same page in each language (`hreflang`, language switch).
    alternates: Value,
    blocks: Vec<Value>,
}

/// An image of the media library in `lang`, or `null` when it no longer exists.
fn image_json(id: Option<&MediaId>, images: &ImageMap, lang: Lang) -> Value {
    id.and_then(|id| images.get(id))
        .map_or(Value::Null, |image| public_image(image, lang))
}

/// A block with its texts resolved in `lang` (English falls back to French) and its images
/// resolved to their public files.
fn public_block(block: &Block, lang: Lang, images: &ImageMap) -> Value {
    let mut value = match &block.kind {
        BlockKind::Hero {
            image,
            title,
            subtitle,
            button_label,
            button_link,
            align,
            height,
            darken,
        } => json!({
            "type": "hero", "image": image_json(image.as_ref(), images, lang), "title": title.get(lang), "subtitle": subtitle.get(lang),
            "button_label": button_label.get(lang), "button_link": button_link,
            "align": align, "height": height, "darken": darken
        }),
        BlockKind::Text { body, width, align } => {
            json!({ "type": "text", "body": body.get(lang), "width": width, "align": align })
        }
        BlockKind::TextImage {
            body,
            image,
            image_position,
            ratio,
        } => json!({
            "type": "text_image", "body": body.get(lang), "image": image_json(image.as_ref(), images, lang),
            "image_position": image_position, "ratio": ratio
        }),
        BlockKind::Columns { items } => json!({
            "type": "columns",
            "items": items.iter().map(|item| json!({
                "title": item.title.get(lang), "body": item.body.get(lang)
            })).collect::<Vec<_>>()
        }),
        BlockKind::Gallery {
            images: gallery,
            columns,
        } => {
            json!({
                "type": "gallery",
                "images": gallery
                    .iter()
                    .filter_map(|id| images.get(id).map(|image| public_image(image, lang)))
                    .collect::<Vec<_>>(),
                "columns": columns
            })
        }
    };
    if let Value::Object(map) = &mut value {
        map.insert("id".into(), json!(block.id));
        map.insert("background".into(), json!(block.background));
        map.insert("spacing".into(), json!(block.spacing));
        map.insert("hide_on_mobile".into(), json!(block.hide_on_mobile));
    }
    value
}

fn public_page(
    page: &StoredPage,
    content: &PageContent,
    lang: Lang,
    images: &ImageMap,
) -> PublicPageResponse {
    PublicPageResponse {
        lang,
        title: content.settings.title.get(lang).to_owned(),
        meta_description: content.settings.meta_description.get(lang).to_owned(),
        alternates: json!({
            "fr": public_path(content, page.is_home, Lang::Fr),
            "en": public_path(content, page.is_home, Lang::En),
        }),
        blocks: content
            .blocks
            .iter()
            .filter(|block| block.visible)
            .map(|block| public_block(block, lang, images))
            .collect(),
    }
}

async fn home(
    State(state): State<AppState>,
    Query(query): Query<LangQuery>,
) -> Result<Json<PublicPageResponse>, ApiError> {
    let found = state.pages.public_home().await?.ok_or(ApiError::NotFound)?;
    let images = state
        .media
        .public_images(&found.content.media_ids())
        .await?;
    Ok(Json(public_page(
        &found.page,
        &found.content,
        query.lang,
        &images,
    )))
}

async fn by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<LangQuery>,
) -> Result<Json<PublicPageResponse>, ApiError> {
    let found = state
        .pages
        .public_page(query.lang, &slug)
        .await?
        .ok_or(ApiError::NotFound)?;
    let images = state
        .media
        .public_images(&found.content.media_ids())
        .await?;
    Ok(Json(public_page(
        &found.page,
        &found.content,
        query.lang,
        &images,
    )))
}

// ----- Admin -----

#[derive(Debug, Serialize)]
pub struct AdminPageSummary {
    id: u64,
    is_home: bool,
    title: String,
    slug: String,
    draft_version: u32,
    published_at: Option<String>,
    unpublished_changes: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminPage {
    id: u64,
    is_home: bool,
    draft_version: u32,
    draft: PageContent,
    published: Option<PageContent>,
    published_at: Option<String>,
}

impl From<StoredPage> for AdminPage {
    fn from(page: StoredPage) -> Self {
        Self {
            id: page.id.0,
            is_home: page.is_home,
            draft_version: page.draft_version,
            draft: page.draft,
            published: page.published,
            published_at: page.published_at.map(timestamp),
        }
    }
}

async fn admin_list(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminPageSummary>>, ApiError> {
    let pages = state.pages.list().await?;
    Ok(Json(
        pages
            .into_iter()
            .map(|page| AdminPageSummary {
                id: page.id.0,
                is_home: page.is_home,
                title: page.draft.settings.title.fr.clone(),
                slug: page.draft.settings.slug.fr.clone(),
                draft_version: page.draft_version,
                published_at: page.published_at.map(timestamp),
                unpublished_changes: page.published.as_ref() != Some(&page.draft),
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreatePageRequest {
    content: Value,
}

async fn admin_create(
    _user: AdminUser,
    State(state): State<AppState>,
    body: Result<Json<CreatePageRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let request = json_body(body)?;
    let id = state.pages.create(&request.content).await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id.0 }))))
}

async fn admin_get(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<AdminPage>, ApiError> {
    Ok(Json(state.pages.get(PageId(id)).await?.into()))
}

async fn admin_delete(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    state.pages.delete(PageId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct SaveDraftRequest {
    draft_version: u32,
    content: Value,
}

async fn admin_save_draft(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    body: Result<Json<SaveDraftRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let request = json_body(body)?;
    let version = state
        .pages
        .save_draft(PageId(id), request.draft_version, &request.content)
        .await?;
    Ok(Json(json!({ "draft_version": version })))
}

#[derive(Debug, Serialize)]
pub struct ChangesResponse {
    draft_version: u32,
    changes: ChangeSummary,
    nothing_to_publish: bool,
}

async fn admin_changes(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<ChangesResponse>, ApiError> {
    let page = state.pages.get(PageId(id)).await?;
    let changes = state.pages.changes(PageId(id)).await?;
    Ok(Json(ChangesResponse {
        draft_version: page.draft_version,
        nothing_to_publish: changes.is_empty(),
        changes,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    draft_version: u32,
}

async fn admin_publish(
    user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    body: Result<Json<PublishRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let request = json_body(body)?;
    state
        .pages
        .publish(PageId(id), request.draft_version, user.0.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    number: u32,
    published_at: String,
    by_owner: bool,
}

impl From<PageVersion> for VersionResponse {
    fn from(version: PageVersion) -> Self {
        Self {
            number: version.number,
            published_at: timestamp(version.published_at),
            by_owner: version.author.is_some(),
        }
    }
}

async fn admin_versions(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Vec<VersionResponse>>, ApiError> {
    let versions = state.pages.versions(PageId(id)).await?;
    Ok(Json(versions.into_iter().map(Into::into).collect()))
}

async fn admin_restore(
    _user: AdminUser,
    State(state): State<AppState>,
    Path((id, number)): Path<(u64, u32)>,
) -> Result<Json<Value>, ApiError> {
    let version = state.pages.restore(PageId(id), number).await?;
    Ok(Json(json!({ "draft_version": version })))
}
