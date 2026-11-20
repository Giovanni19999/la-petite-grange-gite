//! Media library routes (F003): admin only, mounted under `/api/admin/media`.

use std::collections::HashMap;

use axum::extract::multipart::Field;
use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;

use crate::app_state::AppState;
use crate::application::media::{MediaError, MediaService, MediaUse, PublicImage, StoredMedia};
use crate::domain::media::MediaDetails;
use crate::domain::page::{Lang, MediaId};
use crate::http::auth::AdminUser;
use crate::http::error::ApiError;

/// Room for the multipart envelope around the files.
const MULTIPART_OVERHEAD_BYTES: usize = 1024 * 1024;

/// Admin routes; the upload body limit follows the configured limits.
pub fn admin_routes(state: &AppState) -> Router<AppState> {
    let limits = state.media.settings().limits;
    let body_limit = limits
        .max_file_bytes
        .saturating_add(1)
        .saturating_mul(limits.max_files)
        .saturating_add(MULTIPART_OVERHEAD_BYTES);
    Router::new()
        .route(
            "/",
            get(list)
                .post(upload)
                .layer(DefaultBodyLimit::max(body_limit)),
        )
        .route("/{id}", get(find).put(update).delete(remove))
}

impl From<MediaError> for ApiError {
    fn from(error: MediaError) -> Self {
        match error {
            MediaError::NotFound => Self::NotFound,
            MediaError::Invalid(errors) => {
                Self::Invalid(errors.iter().map(ToString::to_string).collect())
            }
            MediaError::InUse { .. } => Self::Conflict {
                code: "media_in_use",
                message: error.to_string(),
            },
            MediaError::Internal(error) => Self::Internal(error),
        }
    }
}

#[derive(Debug, Serialize)]
struct Source {
    width: u32,
    url: String,
}

#[derive(Debug, Serialize)]
struct PageUse {
    page_id: u64,
    title: String,
}

#[derive(Debug, Serialize)]
pub struct MediaResponse {
    id: u64,
    width: u32,
    height: u32,
    sources: Vec<Source>,
    alt: Value,
    caption: Value,
    focal: Value,
    missing_alt: bool,
    used_in: Vec<PageUse>,
    created_at: String,
}

fn media_response(service: &MediaService, media: &StoredMedia, uses: &[MediaUse]) -> MediaResponse {
    MediaResponse {
        id: media.id.0,
        width: media.width,
        height: media.height,
        sources: service
            .sources(media)
            .into_iter()
            .map(|(width, url)| Source { width, url })
            .collect(),
        alt: json!(media.details.alt),
        caption: json!(media.details.caption),
        focal: json!(media.details.focal),
        missing_alt: media.details.missing_alt(),
        used_in: uses
            .iter()
            .map(|page| PageUse {
                page_id: page.page.0,
                title: page.title.clone(),
            })
            .collect(),
        created_at: media.created_at.format(&Rfc3339).unwrap_or_default(),
    }
}

async fn list(
    _user: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<MediaResponse>>, ApiError> {
    let usage = state.media.usage().await?;
    let all = state.media.list().await?;
    Ok(Json(
        all.iter()
            .map(|media| {
                media_response(
                    &state.media,
                    media,
                    usage.get(&media.id).map_or(&[], Vec::as_slice),
                )
            })
            .collect(),
    ))
}

async fn find(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<MediaResponse>, ApiError> {
    let media = state.media.get(MediaId(id)).await?;
    let usage = state.media.usage().await?;
    let uses = usage.get(&media.id).map_or(&[][..], Vec::as_slice);
    Ok(Json(media_response(&state.media, &media, uses)))
}

async fn update(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    body: Result<Json<MediaDetails>, JsonRejection>,
) -> Result<Json<MediaResponse>, ApiError> {
    let Json(details) = body.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let media = state.media.update_details(MediaId(id), details).await?;
    let usage = state.media.usage().await?;
    let uses = usage.get(&media.id).map_or(&[][..], Vec::as_slice);
    Ok(Json(media_response(&state.media, &media, uses)))
}

async fn remove(
    _user: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    state.media.delete(MediaId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
struct Refused {
    file_name: String,
    reason: String,
}

/// Reads a file field, keeping at most `max + 1` bytes (enough to know it is too heavy).
async fn read_bounded(field: &mut Field<'_>, max: usize) -> Result<Vec<u8>, ApiError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|error| ApiError::BadRequest(error.body_text()))?
    {
        if bytes.len() <= max {
            let room = max + 1 - bytes.len();
            bytes.extend_from_slice(&chunk[..chunk.len().min(room)]);
        }
    }
    Ok(bytes)
}

async fn upload(
    _user: AdminUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let max_file_bytes = state.media.settings().limits.max_file_bytes;
    let mut files = Vec::new();
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::BadRequest(error.body_text()))?
    {
        if field.name() != Some("files") {
            continue;
        }
        let file_name = field.file_name().unwrap_or("photo").to_owned();
        files.push((file_name, read_bounded(&mut field, max_file_bytes).await?));
    }
    if files.is_empty() {
        return Err(ApiError::BadRequest("aucun fichier reçu".to_owned()));
    }

    let outcomes = state.media.upload(files).await?;
    let usage = state.media.usage().await?;
    let mut uploaded = Vec::new();
    let mut refused = Vec::new();
    for outcome in outcomes {
        match outcome.result {
            Ok(media) => {
                let uses = usage.get(&media.id).map_or(&[][..], Vec::as_slice);
                uploaded.push(media_response(&state.media, &media, uses));
            }
            Err(reason) => refused.push(Refused {
                file_name: outcome.file_name,
                reason: reason.to_string(),
            }),
        }
    }
    Ok(Json(json!({ "uploaded": uploaded, "refused": refused })))
}

/// JSON of an image for the public pages, in `lang` (English falls back to French).
#[must_use]
pub fn public_image(image: &PublicImage, lang: Lang) -> Value {
    json!({
        "id": image.id.0,
        "width": image.width,
        "height": image.height,
        "sources": image.sources.iter().map(|(width, url)| json!({ "width": width, "url": url })).collect::<Vec<_>>(),
        "alt": image.alt.get(lang),
        "caption": image.caption.get(lang),
        "focal_x": image.focal_x,
        "focal_y": image.focal_y,
    })
}

/// Images of a page, by id, ready to be put in the blocks.
pub type ImageMap = HashMap<MediaId, PublicImage>;
