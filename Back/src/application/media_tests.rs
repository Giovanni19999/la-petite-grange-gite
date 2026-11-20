//! Unit tests of the media service (F003), with in-memory ports.
#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]

use std::sync::Mutex;

use serde_json::json;
use time::macros::datetime;

use super::*;
use crate::application::pages::StoredPage;
use crate::application::pages::fakes::InMemoryPages;
use crate::domain::media::{FocalPoint, variant_widths};
use crate::domain::page::PageContent;

struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        datetime!(2026-11-20 10:00 UTC)
    }
}

/// Pretends every image is 2000 x 1500 and produces one byte per version.
struct FakeProcessor;
impl ImageProcessor for FakeProcessor {
    fn process(
        &self,
        bytes: &[u8],
        limits: &UploadLimits,
        settings: &ImageSettings,
    ) -> Result<ProcessedImage, UploadRefusal> {
        if bytes.ends_with(b"broken") {
            return Err(UploadRefusal::Unreadable);
        }
        limits.check_dimensions(2000, 1500)?;
        Ok(ProcessedImage {
            width: 2000,
            height: 1500,
            variants: variant_widths(2000, &settings.widths)
                .into_iter()
                .map(|width| (width, vec![1]))
                .collect(),
        })
    }
}

#[derive(Default)]
struct FakeFiles {
    saved: Mutex<Vec<String>>,
    removed: Mutex<Vec<String>>,
}
impl MediaFiles for FakeFiles {
    fn save<'a>(
        &'a self,
        hash: &'a ContentHash,
        _extension: &'a str,
        _original: &'a [u8],
        _variants: &'a [(u32, Vec<u8>)],
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        self.saved.lock().unwrap().push(hash.to_string());
        Box::pin(async { Ok(()) })
    }
    fn remove<'a>(&'a self, media: &'a StoredMedia) -> BoxFuture<'a, anyhow::Result<()>> {
        self.removed.lock().unwrap().push(media.hash.to_string());
        Box::pin(async { Ok(()) })
    }
}

#[derive(Default)]
struct InMemoryMedia(Mutex<Vec<StoredMedia>>);
impl MediaRepository for InMemoryMedia {
    fn all(&self) -> BoxFuture<'_, anyhow::Result<Vec<StoredMedia>>> {
        let mut all = self.0.lock().unwrap().clone();
        all.reverse();
        Box::pin(async move { Ok(all) })
    }
    fn find(&self, id: MediaId) -> BoxFuture<'_, anyhow::Result<Option<StoredMedia>>> {
        let found = self.0.lock().unwrap().iter().find(|m| m.id == id).cloned();
        Box::pin(async move { Ok(found) })
    }
    fn find_by_hash<'a>(
        &'a self,
        hash: &'a ContentHash,
    ) -> BoxFuture<'a, anyhow::Result<Option<StoredMedia>>> {
        let found = self
            .0
            .lock()
            .unwrap()
            .iter()
            .find(|m| &m.hash == hash)
            .cloned();
        Box::pin(async move { Ok(found) })
    }
    fn find_many<'a>(
        &'a self,
        ids: &'a [MediaId],
    ) -> BoxFuture<'a, anyhow::Result<Vec<StoredMedia>>> {
        let found = self
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|m| ids.contains(&m.id))
            .cloned()
            .collect();
        Box::pin(async move { Ok(found) })
    }
    fn create<'a>(&'a self, media: &'a NewMedia) -> BoxFuture<'a, anyhow::Result<MediaId>> {
        let mut all = self.0.lock().unwrap();
        let id = MediaId(all.len() as u64 + 1);
        all.push(StoredMedia {
            id,
            hash: media.hash.clone(),
            original_extension: media.original_extension.clone(),
            width: media.width,
            height: media.height,
            widths: media.widths.clone(),
            details: MediaDetails::default(),
            created_at: media.created_at,
        });
        Box::pin(async move { Ok(id) })
    }
    fn update_details<'a>(
        &'a self,
        id: MediaId,
        details: &'a MediaDetails,
    ) -> BoxFuture<'a, anyhow::Result<bool>> {
        let mut all = self.0.lock().unwrap();
        let found = all.iter_mut().find(|m| m.id == id);
        let updated = found.is_some();
        if let Some(media) = found {
            media.details = details.clone();
        }
        Box::pin(async move { Ok(updated) })
    }
    fn delete(&self, id: MediaId) -> BoxFuture<'_, anyhow::Result<()>> {
        self.0.lock().unwrap().retain(|m| m.id != id);
        Box::pin(async { Ok(()) })
    }
}

struct Fixture {
    service: MediaService,
    files: Arc<FakeFiles>,
    pages: Arc<InMemoryPages>,
}

fn fixture() -> Fixture {
    let files = Arc::new(FakeFiles::default());
    let pages = Arc::new(InMemoryPages::default());
    let service = MediaService::new(
        MediaPorts {
            media: Arc::new(InMemoryMedia::default()),
            files: files.clone(),
            processor: Arc::new(FakeProcessor),
            pages: pages.clone(),
            clock: Arc::new(FixedClock),
        },
        MediaSettings {
            limits: UploadLimits {
                max_file_bytes: 1024,
                max_files: 3,
                max_pixels: 50_000_000,
            },
            images: ImageSettings {
                widths: vec![480, 960, 1440, 1920],
                quality: 80.0,
            },
            public_url: "/uploads/".into(),
        },
    );
    Fixture {
        service,
        files,
        pages,
    }
}

fn jpeg(tag: &str) -> Vec<u8> {
    [&[0xFF, 0xD8, 0xFF, 0xE0][..], tag.as_bytes()].concat()
}

fn page_using(id: u64, title: &str, image: u64) -> StoredPage {
    let content = PageContent::parse(&json!({
        "settings": { "slug": { "fr": "p", "en": "p" }, "title": { "fr": title, "en": "" } },
        "blocks": [ { "id": "h", "type": "hero", "image": image } ]
    }))
    .unwrap();
    StoredPage {
        id: PageId(id),
        is_home: id == 1,
        draft: content.clone(),
        draft_version: 1,
        published: Some(content),
        published_at: None,
    }
}

async fn upload_one(service: &MediaService, name: &str, bytes: Vec<u8>) -> StoredMedia {
    service.upload(vec![(name.into(), bytes)]).await.unwrap()[0]
        .result
        .as_ref()
        .unwrap()
        .clone()
}

#[tokio::test]
async fn f003_s01_s03_each_file_is_accepted_or_refused_on_its_own() {
    let f = fixture();
    let outcomes = f
        .service
        .upload(vec![
            ("a.jpg".into(), jpeg("a")),
            ("anim.gif".into(), b"GIF89a".to_vec()),
            ("big.jpg".into(), [jpeg("b"), vec![0; 2048]].concat()),
            ("extra.jpg".into(), jpeg("c")),
        ])
        .await
        .unwrap();

    assert_eq!(
        outcomes[0].result.as_ref().unwrap().widths,
        vec![480, 960, 1440, 1920]
    );
    assert_eq!(
        outcomes[1].result.as_ref().unwrap_err(),
        &UploadRefusal::UnsupportedFormat
    );
    assert_eq!(
        outcomes[2].result.as_ref().unwrap_err(),
        &UploadRefusal::TooHeavy { max_mb: 0 }
    );
    assert_eq!(
        outcomes[3].result.as_ref().unwrap_err(),
        &UploadRefusal::TooManyFiles { max: 3 }
    );
    assert_eq!(f.files.saved.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn f003_s01_the_same_photo_twice_is_one_media() {
    let f = fixture();
    let first = upload_one(&f.service, "a.jpg", jpeg("a")).await;
    let second = upload_one(&f.service, "copie.jpg", jpeg("a")).await;
    assert_eq!(first.id, second.id);
    assert_eq!(f.files.saved.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn f003_s03_unreadable_image_is_refused() {
    let f = fixture();
    let outcomes = f
        .service
        .upload(vec![("x.jpg".into(), jpeg("broken"))])
        .await
        .unwrap();
    assert_eq!(
        outcomes[0].result.as_ref().unwrap_err(),
        &UploadRefusal::Unreadable
    );
}

#[tokio::test]
async fn f003_s05_details_are_saved() {
    let f = fixture();
    let id = upload_one(&f.service, "a.jpg", jpeg("a")).await.id;
    let details = MediaDetails {
        alt: Localized {
            fr: "La grange".into(),
            en: String::new(),
        },
        caption: Localized {
            fr: String::new(),
            en: "The barn".into(),
        },
        focal: FocalPoint { x: 30, y: 70 },
    };
    let media = f.service.update_details(id, details.clone()).await.unwrap();
    assert_eq!(media.details, details);

    let invalid = MediaDetails {
        focal: FocalPoint { x: 120, y: 0 },
        ..details
    };
    assert!(matches!(
        f.service.update_details(id, invalid).await,
        Err(MediaError::Invalid(_))
    ));
}

#[tokio::test]
async fn f003_s06_s07_delete_only_unused_media() {
    let f = fixture();
    let used = upload_one(&f.service, "a.jpg", jpeg("a")).await.id;
    let unused = upload_one(&f.service, "b.jpg", jpeg("b")).await.id;
    f.pages.insert(page_using(1, "Accueil", used.0));

    let refused = f.service.delete(used).await.unwrap_err();
    assert_eq!(refused.to_string(), "image utilisée dans : « Accueil »");

    f.service.delete(unused).await.unwrap();
    assert!(matches!(
        f.service.get(unused).await,
        Err(MediaError::NotFound)
    ));
    assert_eq!(f.files.removed.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn f003_s09_public_images_have_urls_per_width() {
    let f = fixture();
    let media = upload_one(&f.service, "a.jpg", jpeg("a")).await;
    let images = f
        .service
        .public_images(&[media.id, MediaId(99)])
        .await
        .unwrap();
    assert_eq!(images.len(), 1);
    let image = &images[&media.id];
    assert_eq!(image.sources.len(), 4);
    assert_eq!(
        image.sources[0],
        (480, format!("/uploads/{}-480.webp", media.hash))
    );
    assert_eq!((image.focal_x, image.focal_y), (50, 50));
}
