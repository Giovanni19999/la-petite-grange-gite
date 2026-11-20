//! Media library use cases (F003, ADR-0017): upload, details, deletion, usage, and the image
//! information the public pages need.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::application::auth::Clock;
use crate::application::health::BoxFuture;
use crate::application::pages::{MediaCatalog, PageId, PageRepository};
use crate::domain::media::{
    ContentHash, DetailsError, ImageSettings, MediaDetails, SourceFormat, UploadLimits,
    UploadRefusal,
};
use crate::domain::page::{Localized, MediaId};

/// A media of the library, as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMedia {
    pub id: MediaId,
    pub hash: ContentHash,
    /// Extension of the private original (`jpg`, `png`, `webp`).
    pub original_extension: String,
    pub width: u32,
    pub height: u32,
    /// Widths of the published WebP versions, ascending.
    pub widths: Vec<u32>,
    pub details: MediaDetails,
    pub created_at: OffsetDateTime,
}

/// A media to record after its files were written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMedia {
    pub hash: ContentHash,
    pub original_extension: String,
    pub width: u32,
    pub height: u32,
    pub widths: Vec<u32>,
    pub created_at: OffsetDateTime,
}

/// Result of the image processing: dimensions after orientation and the WebP versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedImage {
    pub width: u32,
    pub height: u32,
    /// `(width, WebP bytes)`, ascending widths.
    pub variants: Vec<(u32, Vec<u8>)>,
}

/// Port: decoding, orientation, resizing and WebP encoding (CPU bound, synchronous).
pub trait ImageProcessor: Send + Sync {
    /// # Errors
    ///
    /// The reason why the image is refused (too many pixels, unreadable…).
    fn process(
        &self,
        bytes: &[u8],
        limits: &UploadLimits,
        settings: &ImageSettings,
    ) -> Result<ProcessedImage, UploadRefusal>;
}

/// Port: storage of the media files (public WebP versions and private originals).
pub trait MediaFiles: Send + Sync {
    fn save<'a>(
        &'a self,
        hash: &'a ContentHash,
        original_extension: &'a str,
        original: &'a [u8],
        variants: &'a [(u32, Vec<u8>)],
    ) -> BoxFuture<'a, anyhow::Result<()>>;
    fn remove<'a>(&'a self, media: &'a StoredMedia) -> BoxFuture<'a, anyhow::Result<()>>;
}

/// Port: media records.
pub trait MediaRepository: Send + Sync {
    /// Every media, newest first.
    fn all(&self) -> BoxFuture<'_, anyhow::Result<Vec<StoredMedia>>>;
    fn find(&self, id: MediaId) -> BoxFuture<'_, anyhow::Result<Option<StoredMedia>>>;
    fn find_by_hash<'a>(
        &'a self,
        hash: &'a ContentHash,
    ) -> BoxFuture<'a, anyhow::Result<Option<StoredMedia>>>;
    fn find_many<'a>(
        &'a self,
        ids: &'a [MediaId],
    ) -> BoxFuture<'a, anyhow::Result<Vec<StoredMedia>>>;
    fn create<'a>(&'a self, media: &'a NewMedia) -> BoxFuture<'a, anyhow::Result<MediaId>>;
    /// Returns `false` when the media does not exist.
    fn update_details<'a>(
        &'a self,
        id: MediaId,
        details: &'a MediaDetails,
    ) -> BoxFuture<'a, anyhow::Result<bool>>;
    fn delete(&self, id: MediaId) -> BoxFuture<'_, anyhow::Result<()>>;
}

/// Settings of the media library (technical, with defaults: ADR-0010).
#[derive(Debug, Clone, PartialEq)]
pub struct MediaSettings {
    pub limits: UploadLimits,
    pub images: ImageSettings,
    /// URL prefix under which the public files are served, e.g. `/uploads`.
    pub public_url: String,
}

/// A page using a media.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaUse {
    pub page: PageId,
    pub title: String,
}

/// One uploaded file and what became of it.
#[derive(Debug)]
pub struct UploadOutcome {
    pub file_name: String,
    pub result: Result<StoredMedia, UploadRefusal>,
}

/// Why a media operation failed.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("image introuvable")]
    NotFound,
    #[error("réglages invalides")]
    Invalid(Vec<DetailsError>),
    #[error("image utilisée dans : {}", pages.iter().map(|page| format!("« {} »", page.title)).collect::<Vec<_>>().join(", "))]
    InUse { pages: Vec<MediaUse> },
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Image information for the public pages: URLs of the versions and descriptive texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicImage {
    pub id: MediaId,
    pub width: u32,
    pub height: u32,
    /// `(width, url)`, ascending.
    pub sources: Vec<(u32, String)>,
    pub alt: Localized,
    pub caption: Localized,
    pub focal_x: u8,
    pub focal_y: u8,
}

/// Ports of the media service.
pub struct MediaPorts {
    pub media: Arc<dyn MediaRepository>,
    pub files: Arc<dyn MediaFiles>,
    pub processor: Arc<dyn ImageProcessor>,
    pub pages: Arc<dyn PageRepository>,
    pub clock: Arc<dyn Clock>,
}

impl fmt::Debug for MediaPorts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MediaPorts").finish_non_exhaustive()
    }
}

/// Media library use cases.
#[derive(Clone)]
pub struct MediaService {
    media: Arc<dyn MediaRepository>,
    files: Arc<dyn MediaFiles>,
    processor: Arc<dyn ImageProcessor>,
    pages: Arc<dyn PageRepository>,
    clock: Arc<dyn Clock>,
    settings: Arc<MediaSettings>,
}

impl fmt::Debug for MediaService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MediaService")
            .field("settings", &self.settings)
            .finish_non_exhaustive()
    }
}

impl MediaService {
    #[must_use]
    pub fn new(ports: MediaPorts, settings: MediaSettings) -> Self {
        Self {
            media: ports.media,
            files: ports.files,
            processor: ports.processor,
            pages: ports.pages,
            clock: ports.clock,
            settings: Arc::new(settings),
        }
    }

    #[must_use]
    pub fn settings(&self) -> &MediaSettings {
        &self.settings
    }

    /// Every media, newest first.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn list(&self) -> anyhow::Result<Vec<StoredMedia>> {
        self.media.all().await
    }

    /// # Errors
    ///
    /// Not found or infrastructure errors.
    pub async fn get(&self, id: MediaId) -> Result<StoredMedia, MediaError> {
        self.media.find(id).await?.ok_or(MediaError::NotFound)
    }

    /// Saves the uploaded files; each one is accepted or refused on its own.
    ///
    /// # Errors
    ///
    /// Infrastructure errors (a refused file is not an error, see [`UploadOutcome`]).
    pub async fn upload(
        &self,
        files: Vec<(String, Vec<u8>)>,
    ) -> anyhow::Result<Vec<UploadOutcome>> {
        let max_files = self.settings.limits.max_files;
        let mut outcomes = Vec::with_capacity(files.len());
        for (index, (file_name, bytes)) in files.into_iter().enumerate() {
            let result = if index < max_files {
                self.upload_one(bytes).await?
            } else {
                Err(UploadRefusal::TooManyFiles { max: max_files })
            };
            outcomes.push(UploadOutcome { file_name, result });
        }
        Ok(outcomes)
    }

    async fn upload_one(
        &self,
        bytes: Vec<u8>,
    ) -> anyhow::Result<Result<StoredMedia, UploadRefusal>> {
        let format = match self.settings.limits.check_file(&bytes) {
            Ok(format) => format,
            Err(refusal) => return Ok(Err(refusal)),
        };
        let hash = ContentHash::parse(&hex::encode(Sha256::digest(&bytes)))
            .ok_or_else(|| anyhow::anyhow!("invalid content hash"))?;
        // The same photo sent twice is the same media (its files are named after the content).
        if let Some(existing) = self.media.find_by_hash(&hash).await? {
            return Ok(Ok(existing));
        }

        let processor = self.processor.clone();
        let settings = self.settings.clone();
        let (bytes, processed) = tokio::task::spawn_blocking(move || {
            let processed = processor.process(&bytes, &settings.limits, &settings.images);
            (bytes, processed)
        })
        .await?;
        let processed = match processed {
            Ok(processed) => processed,
            Err(refusal) => return Ok(Err(refusal)),
        };

        let extension = format.extension();
        self.files
            .save(&hash, extension, &bytes, &processed.variants)
            .await?;
        let new = NewMedia {
            hash,
            original_extension: extension.to_owned(),
            width: processed.width,
            height: processed.height,
            widths: processed.variants.iter().map(|(width, _)| *width).collect(),
            created_at: self.clock.now(),
        };
        let id = self.media.create(&new).await?;
        Ok(Ok(self
            .get(id)
            .await
            .map_err(|error| anyhow::anyhow!(error))?))
    }

    /// Updates the texts and focal point.
    ///
    /// # Errors
    ///
    /// Invalid details, not found, infrastructure errors.
    pub async fn update_details(
        &self,
        id: MediaId,
        details: MediaDetails,
    ) -> Result<StoredMedia, MediaError> {
        let details = details.validated().map_err(MediaError::Invalid)?;
        if !self.media.update_details(id, &details).await? {
            return Err(MediaError::NotFound);
        }
        self.get(id).await
    }

    /// Deletes a media and its files, unless a page (draft or published) uses it.
    ///
    /// # Errors
    ///
    /// Not found, in use, infrastructure errors.
    pub async fn delete(&self, id: MediaId) -> Result<(), MediaError> {
        let media = self.get(id).await?;
        let uses = self.usage().await?.remove(&id).unwrap_or_default();
        if !uses.is_empty() {
            return Err(MediaError::InUse { pages: uses });
        }
        self.media.delete(id).await?;
        self.files.remove(&media).await?;
        Ok(())
    }

    /// Pages using each media, in their draft or published content.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn usage(&self) -> anyhow::Result<HashMap<MediaId, Vec<MediaUse>>> {
        let mut usage: HashMap<MediaId, Vec<MediaUse>> = HashMap::new();
        for page in self.pages.all().await? {
            let title = page.draft.settings.title.fr.clone();
            let mut ids = page.draft.media_ids();
            if let Some(published) = &page.published {
                ids.extend(published.media_ids());
            }
            ids.sort_by_key(|id| id.0);
            ids.dedup();
            for id in ids {
                usage.entry(id).or_default().push(MediaUse {
                    page: page.id,
                    title: title.clone(),
                });
            }
        }
        Ok(usage)
    }

    /// Public URLs of the versions of a media.
    #[must_use]
    pub fn sources(&self, media: &StoredMedia) -> Vec<(u32, String)> {
        let base = self.settings.public_url.trim_end_matches('/');
        media
            .widths
            .iter()
            .map(|width| {
                (
                    *width,
                    format!("{base}/{}", media.hash.variant_file(*width)),
                )
            })
            .collect()
    }

    /// Image information for the public pages, for the ids that still exist.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn public_images(
        &self,
        ids: &[MediaId],
    ) -> anyhow::Result<HashMap<MediaId, PublicImage>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        Ok(self
            .media
            .find_many(ids)
            .await?
            .into_iter()
            .map(|media| {
                let image = PublicImage {
                    id: media.id,
                    width: media.width,
                    height: media.height,
                    sources: self.sources(&media),
                    alt: media.details.alt.clone(),
                    caption: media.details.caption.clone(),
                    focal_x: media.details.focal.x,
                    focal_y: media.details.focal.y,
                };
                (media.id, image)
            })
            .collect())
    }
}

impl MediaCatalog for MediaService {
    fn existing<'a>(&'a self, ids: &'a [MediaId]) -> BoxFuture<'a, anyhow::Result<Vec<MediaId>>> {
        Box::pin(async move {
            Ok(self
                .media
                .find_many(ids)
                .await?
                .into_iter()
                .map(|media| media.id)
                .collect())
        })
    }
}

/// Source format kept for the original, from its extension.
#[must_use]
pub fn is_known_extension(extension: &str) -> bool {
    [SourceFormat::Jpeg, SourceFormat::Png, SourceFormat::Webp]
        .iter()
        .any(|format| format.extension() == extension)
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod tests;
