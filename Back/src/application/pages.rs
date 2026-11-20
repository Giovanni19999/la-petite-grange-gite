//! Page use cases (F002): public reading, draft editing, publication, versions.

use std::fmt;
use std::sync::Arc;

use time::OffsetDateTime;

use crate::application::auth::Clock;
use crate::application::health::BoxFuture;
use crate::domain::auth::UserId;
use crate::domain::page::{ChangeSummary, ContentError, Lang, MediaId, PageContent};

/// Identifier of a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageId(pub u64);

/// A page as stored: draft, published content and versioning data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPage {
    pub id: PageId,
    pub is_home: bool,
    pub draft: PageContent,
    /// Incremented at every draft save; publishing requires the confirmed value.
    pub draft_version: u32,
    pub published: Option<PageContent>,
    pub published_at: Option<OffsetDateTime>,
}

/// A published version of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageVersion {
    pub number: u32,
    pub published_at: OffsetDateTime,
    pub author: Option<UserId>,
}

/// Port: page storage.
pub trait PageRepository: Send + Sync {
    fn all(&self) -> BoxFuture<'_, anyhow::Result<Vec<StoredPage>>>;
    fn find(&self, id: PageId) -> BoxFuture<'_, anyhow::Result<Option<StoredPage>>>;
    /// Page whose **published** slug in `lang` is `slug`.
    fn find_published<'a>(
        &'a self,
        lang: Lang,
        slug: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<Option<StoredPage>>>;
    fn home(&self) -> BoxFuture<'_, anyhow::Result<Option<StoredPage>>>;
    /// Creates an unpublished page with `content` as draft.
    fn create<'a>(&'a self, content: &'a PageContent) -> BoxFuture<'a, anyhow::Result<PageId>>;
    /// Replaces the draft if its version is still `expected_version`; returns the new version,
    /// or `None` when the version changed meanwhile.
    fn save_draft<'a>(
        &'a self,
        id: PageId,
        expected_version: u32,
        content: &'a PageContent,
    ) -> BoxFuture<'a, anyhow::Result<Option<u32>>>;
    /// Publishes the draft if its version is still `expected_version` and records a version;
    /// returns `false` when the draft version changed meanwhile.
    fn publish(
        &self,
        id: PageId,
        expected_version: u32,
        author: UserId,
        at: OffsetDateTime,
    ) -> BoxFuture<'_, anyhow::Result<bool>>;
    fn versions(&self, id: PageId) -> BoxFuture<'_, anyhow::Result<Vec<PageVersion>>>;
    fn version_content(
        &self,
        id: PageId,
        number: u32,
    ) -> BoxFuture<'_, anyhow::Result<Option<PageContent>>>;
    fn delete(&self, id: PageId) -> BoxFuture<'_, anyhow::Result<()>>;
}

/// Port: which media of the library exist (F003), to refuse unknown images in a draft.
pub trait MediaCatalog: Send + Sync {
    /// The ids among `ids` that exist in the media library.
    fn existing<'a>(&'a self, ids: &'a [MediaId]) -> BoxFuture<'a, anyhow::Result<Vec<MediaId>>>;
}

/// Why a page operation failed.
#[derive(Debug, thiserror::Error)]
pub enum PageError {
    #[error("page introuvable")]
    NotFound,
    #[error("contenu invalide")]
    Invalid(Vec<ContentError>),
    #[error("la page a été modifiée entre-temps (version actuelle : {current})")]
    VersionConflict { current: u32 },
    #[error("l'adresse « {slug} » ({lang}) est déjà utilisée par une autre page")]
    SlugTaken { lang: Lang, slug: String },
    #[error("la page d'accueil ne peut pas être supprimée")]
    HomeCannotBeDeleted,
    #[error("version introuvable")]
    VersionNotFound,
    #[error("image introuvable dans la médiathèque : {}", ids.iter().map(|id| format!("n° {}", id.0)).collect::<Vec<_>>().join(", "))]
    UnknownMedia { ids: Vec<MediaId> },
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// A page and the menu, as shown to visitors in one language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicPage {
    pub page: StoredPage,
    pub content: PageContent,
}

/// An entry of the public menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub is_home: bool,
    pub content: PageContent,
}

/// Page use cases.
#[derive(Clone)]
pub struct PageService {
    pages: Arc<dyn PageRepository>,
    clock: Arc<dyn Clock>,
    media: Arc<dyn MediaCatalog>,
}

impl fmt::Debug for PageService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageService").finish_non_exhaustive()
    }
}

impl PageService {
    #[must_use]
    pub fn new(
        pages: Arc<dyn PageRepository>,
        clock: Arc<dyn Clock>,
        media: Arc<dyn MediaCatalog>,
    ) -> Self {
        Self {
            pages,
            clock,
            media,
        }
    }

    // ----- Public (visitors): published content only -----

    /// Published home page.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn public_home(&self) -> anyhow::Result<Option<PublicPage>> {
        Ok(self.pages.home().await?.and_then(Self::public))
    }

    /// Published page by its slug in `lang`.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn public_page(&self, lang: Lang, slug: &str) -> anyhow::Result<Option<PublicPage>> {
        Ok(self
            .pages
            .find_published(lang, slug)
            .await?
            .and_then(Self::public))
    }

    fn public(page: StoredPage) -> Option<PublicPage> {
        let content = page.published.clone()?;
        Some(PublicPage { page, content })
    }

    /// Published pages shown in the menu: home first, then by menu order.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn menu(&self) -> anyhow::Result<Vec<MenuEntry>> {
        let mut entries: Vec<MenuEntry> = self
            .pages
            .all()
            .await?
            .into_iter()
            .filter_map(|page| {
                let content = page.published?;
                (page.is_home || content.settings.in_menu).then_some(MenuEntry {
                    is_home: page.is_home,
                    content,
                })
            })
            .collect();
        entries.sort_by_key(|entry| (!entry.is_home, entry.content.settings.menu_order));
        Ok(entries)
    }

    // ----- Admin -----

    /// All pages with their draft and published content.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn list(&self) -> anyhow::Result<Vec<StoredPage>> {
        let mut pages = self.pages.all().await?;
        pages.sort_by_key(|page| (!page.is_home, page.draft.settings.menu_order, page.id.0));
        Ok(pages)
    }

    /// One page.
    ///
    /// # Errors
    ///
    /// [`PageError::NotFound`] or infrastructure errors.
    pub async fn get(&self, id: PageId) -> Result<StoredPage, PageError> {
        self.pages.find(id).await?.ok_or(PageError::NotFound)
    }

    /// Creates an unpublished page.
    ///
    /// # Errors
    ///
    /// Invalid content, slug already used, or infrastructure errors.
    pub async fn create(&self, raw: &serde_json::Value) -> Result<PageId, PageError> {
        let content = PageContent::parse(raw).map_err(PageError::Invalid)?;
        self.check_media_exist(&content).await?;
        self.check_slugs_free(None, &content).await?;
        Ok(self.pages.create(&content).await?)
    }

    /// Saves the draft; returns the new draft version.
    ///
    /// # Errors
    ///
    /// Not found, invalid content, slug already used, version conflict, infrastructure.
    pub async fn save_draft(
        &self,
        id: PageId,
        expected_version: u32,
        raw: &serde_json::Value,
    ) -> Result<u32, PageError> {
        let content = PageContent::parse(raw).map_err(PageError::Invalid)?;
        let page = self.get(id).await?;
        self.check_media_exist(&content).await?;
        self.check_slugs_free(Some(id), &content).await?;
        self.pages
            .save_draft(id, expected_version, &content)
            .await?
            .ok_or(PageError::VersionConflict {
                current: page.draft_version,
            })
    }

    /// Summary of changes between the published page and the draft.
    ///
    /// # Errors
    ///
    /// Not found or infrastructure errors.
    pub async fn changes(&self, id: PageId) -> Result<ChangeSummary, PageError> {
        let page = self.get(id).await?;
        Ok(ChangeSummary::between(page.published.as_ref(), &page.draft))
    }

    /// Publishes the draft confirmed by the owner (its version must still be current).
    ///
    /// # Errors
    ///
    /// Not found, version conflict, slug already used by another published page,
    /// infrastructure errors.
    pub async fn publish(
        &self,
        id: PageId,
        confirmed_version: u32,
        author: UserId,
    ) -> Result<(), PageError> {
        let page = self.get(id).await?;
        if page.draft_version != confirmed_version {
            return Err(PageError::VersionConflict {
                current: page.draft_version,
            });
        }
        self.check_slugs_free(Some(id), &page.draft).await?;
        let published = self
            .pages
            .publish(id, confirmed_version, author, self.clock.now())
            .await?;
        if published {
            Ok(())
        } else {
            let current = self.get(id).await?.draft_version;
            Err(PageError::VersionConflict { current })
        }
    }

    /// Published versions, most recent first.
    ///
    /// # Errors
    ///
    /// Not found or infrastructure errors.
    pub async fn versions(&self, id: PageId) -> Result<Vec<PageVersion>, PageError> {
        self.get(id).await?;
        let mut versions = self.pages.versions(id).await?;
        versions.sort_by_key(|version| std::cmp::Reverse(version.number));
        Ok(versions)
    }

    /// Puts a published version back into the draft; returns the new draft version.
    ///
    /// # Errors
    ///
    /// Page or version not found, infrastructure errors.
    pub async fn restore(&self, id: PageId, number: u32) -> Result<u32, PageError> {
        let page = self.get(id).await?;
        let content = self
            .pages
            .version_content(id, number)
            .await?
            .ok_or(PageError::VersionNotFound)?;
        self.pages
            .save_draft(id, page.draft_version, &content)
            .await?
            .ok_or(PageError::VersionConflict {
                current: page.draft_version,
            })
    }

    /// Deletes a page (never the home page).
    ///
    /// # Errors
    ///
    /// Not found, home page, infrastructure errors.
    pub async fn delete(&self, id: PageId) -> Result<(), PageError> {
        if self.get(id).await?.is_home {
            return Err(PageError::HomeCannotBeDeleted);
        }
        Ok(self.pages.delete(id).await?)
    }

    /// Every image of the content must exist in the media library.
    async fn check_media_exist(&self, content: &PageContent) -> Result<(), PageError> {
        let used = content.media_ids();
        if used.is_empty() {
            return Ok(());
        }
        let existing = self.media.existing(&used).await?;
        let unknown: Vec<MediaId> = used
            .into_iter()
            .filter(|id| !existing.contains(id))
            .collect();
        if unknown.is_empty() {
            Ok(())
        } else {
            Err(PageError::UnknownMedia { ids: unknown })
        }
    }

    /// A slug must not be used by the draft or published content of another page.
    async fn check_slugs_free(
        &self,
        id: Option<PageId>,
        content: &PageContent,
    ) -> Result<(), PageError> {
        let others = self.pages.all().await?;
        for lang in Lang::ALL {
            let slug = slug_of(content, lang);
            let taken = others
                .iter()
                .filter(|page| Some(page.id) != id)
                .any(|page| {
                    slug_of(&page.draft, lang) == slug
                        || page
                            .published
                            .as_ref()
                            .is_some_and(|published| slug_of(published, lang) == slug)
                });
            if taken {
                return Err(PageError::SlugTaken {
                    lang,
                    slug: slug.to_owned(),
                });
            }
        }
        Ok(())
    }
}

fn slug_of(content: &PageContent, lang: Lang) -> &str {
    match lang {
        Lang::Fr => &content.settings.slug.fr,
        Lang::En => &content.settings.slug.en,
    }
}

#[cfg(test)]
#[allow(clippy::missing_panics_doc)] // test doubles
pub mod fakes {
    //! In-memory page repository, for tests.

    use std::sync::Mutex;

    use super::*;

    #[derive(Debug, Default)]
    pub struct InMemoryPages {
        pub pages: Mutex<Vec<StoredPage>>,
        pub versions: Mutex<Vec<(PageId, PageVersion, PageContent)>>,
    }

    impl InMemoryPages {
        pub fn insert(&self, page: StoredPage) {
            self.pages.lock().unwrap().push(page);
        }
    }

    /// Media library containing exactly the given ids.
    #[derive(Debug, Default)]
    pub struct KnownMedia(pub Vec<MediaId>);

    impl MediaCatalog for KnownMedia {
        fn existing<'a>(
            &'a self,
            ids: &'a [MediaId],
        ) -> BoxFuture<'a, anyhow::Result<Vec<MediaId>>> {
            Box::pin(async move {
                Ok(ids
                    .iter()
                    .copied()
                    .filter(|id| self.0.contains(id))
                    .collect())
            })
        }
    }

    impl PageRepository for InMemoryPages {
        fn all(&self) -> BoxFuture<'_, anyhow::Result<Vec<StoredPage>>> {
            let pages = self.pages.lock().unwrap().clone();
            Box::pin(async move { Ok(pages) })
        }

        fn find(&self, id: PageId) -> BoxFuture<'_, anyhow::Result<Option<StoredPage>>> {
            let page = self
                .pages
                .lock()
                .unwrap()
                .iter()
                .find(|p| p.id == id)
                .cloned();
            Box::pin(async move { Ok(page) })
        }

        fn find_published<'a>(
            &'a self,
            lang: Lang,
            slug: &'a str,
        ) -> BoxFuture<'a, anyhow::Result<Option<StoredPage>>> {
            let page = self
                .pages
                .lock()
                .unwrap()
                .iter()
                .find(|p| {
                    p.published
                        .as_ref()
                        .is_some_and(|c| slug_of(c, lang) == slug)
                })
                .cloned();
            Box::pin(async move { Ok(page) })
        }

        fn home(&self) -> BoxFuture<'_, anyhow::Result<Option<StoredPage>>> {
            let page = self
                .pages
                .lock()
                .unwrap()
                .iter()
                .find(|p| p.is_home)
                .cloned();
            Box::pin(async move { Ok(page) })
        }

        fn create<'a>(&'a self, content: &'a PageContent) -> BoxFuture<'a, anyhow::Result<PageId>> {
            let mut pages = self.pages.lock().unwrap();
            let id = PageId(pages.iter().map(|p| p.id.0).max().unwrap_or(0) + 1);
            pages.push(StoredPage {
                id,
                is_home: false,
                draft: content.clone(),
                draft_version: 1,
                published: None,
                published_at: None,
            });
            Box::pin(async move { Ok(id) })
        }

        fn save_draft<'a>(
            &'a self,
            id: PageId,
            expected_version: u32,
            content: &'a PageContent,
        ) -> BoxFuture<'a, anyhow::Result<Option<u32>>> {
            let mut pages = self.pages.lock().unwrap();
            let result = pages
                .iter_mut()
                .find(|p| p.id == id && p.draft_version == expected_version)
                .map(|page| {
                    page.draft = content.clone();
                    page.draft_version += 1;
                    page.draft_version
                });
            Box::pin(async move { Ok(result) })
        }

        fn publish(
            &self,
            id: PageId,
            expected_version: u32,
            author: UserId,
            at: OffsetDateTime,
        ) -> BoxFuture<'_, anyhow::Result<bool>> {
            let mut pages = self.pages.lock().unwrap();
            let mut versions = self.versions.lock().unwrap();
            let done = match pages
                .iter_mut()
                .find(|p| p.id == id && p.draft_version == expected_version)
            {
                Some(page) => {
                    page.published = Some(page.draft.clone());
                    page.published_at = Some(at);
                    let number = u32::try_from(versions.iter().filter(|(p, ..)| *p == id).count())
                        .unwrap()
                        + 1;
                    versions.push((
                        id,
                        PageVersion {
                            number,
                            published_at: at,
                            author: Some(author),
                        },
                        page.draft.clone(),
                    ));
                    true
                }
                None => false,
            };
            Box::pin(async move { Ok(done) })
        }

        fn versions(&self, id: PageId) -> BoxFuture<'_, anyhow::Result<Vec<PageVersion>>> {
            let list = self
                .versions
                .lock()
                .unwrap()
                .iter()
                .filter(|(p, ..)| *p == id)
                .map(|(_, v, _)| v.clone())
                .collect();
            Box::pin(async move { Ok(list) })
        }

        fn version_content(
            &self,
            id: PageId,
            number: u32,
        ) -> BoxFuture<'_, anyhow::Result<Option<PageContent>>> {
            let content = self
                .versions
                .lock()
                .unwrap()
                .iter()
                .find(|(p, v, _)| *p == id && v.number == number)
                .map(|(.., c)| c.clone());
            Box::pin(async move { Ok(content) })
        }

        fn delete(&self, id: PageId) -> BoxFuture<'_, anyhow::Result<()>> {
            self.pages.lock().unwrap().retain(|p| p.id != id);
            Box::pin(async { Ok(()) })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use serde_json::json;
    use time::macros::datetime;

    use super::fakes::{InMemoryPages, KnownMedia};
    use super::*;
    use crate::application::auth::fakes::FixedClock;

    const OWNER: UserId = UserId(1);

    fn content(slug_fr: &str, slug_en: &str, title: &str) -> serde_json::Value {
        json!({
            "settings": {
                "slug": { "fr": slug_fr, "en": slug_en },
                "title": { "fr": title, "en": "" },
                "in_menu": true,
                "menu_order": 2
            },
            "blocks": [ { "id": "t", "type": "text", "body": { "fr": "<p>Texte</p>" } } ]
        })
    }

    async fn service() -> (PageService, Arc<InMemoryPages>) {
        let pages = Arc::new(InMemoryPages::default());
        let clock = Arc::new(FixedClock(Mutex::new(datetime!(2026-11-20 10:00 UTC))));
        let media = Arc::new(KnownMedia(vec![MediaId(7)]));
        let service = PageService::new(pages.clone(), clock, media);
        let home = PageContent::parse(&content("accueil", "home", "Accueil")).unwrap();
        pages.insert(StoredPage {
            id: PageId(1),
            is_home: true,
            draft: home.clone(),
            draft_version: 1,
            published: None,
            published_at: None,
        });
        service.publish(PageId(1), 1, OWNER).await.unwrap();
        (service, pages)
    }

    #[tokio::test]
    async fn f003_s08_draft_with_unknown_media_is_refused() {
        let (service, _) = service().await;
        let mut with_images = content("accueil", "home", "Accueil");
        with_images["blocks"] = json!([
            { "id": "g", "type": "gallery", "images": [7, 99] }
        ]);

        let error = service
            .save_draft(PageId(1), 1, &with_images)
            .await
            .unwrap_err();
        assert!(
            matches!(&error, PageError::UnknownMedia { ids } if ids == &vec![MediaId(99)]),
            "{error:?}"
        );

        with_images["blocks"] = json!([{ "id": "g", "type": "gallery", "images": [7] }]);
        service
            .save_draft(PageId(1), 1, &with_images)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn f002_s04_draft_is_not_public() {
        let (service, _) = service().await;

        service
            .save_draft(PageId(1), 1, &content("accueil", "home", "Nouveau titre"))
            .await
            .unwrap();

        let public = service.public_home().await.unwrap().unwrap();
        assert_eq!(public.content.settings.title.fr, "Accueil");
    }

    #[tokio::test]
    async fn f002_s05_publish_makes_the_draft_public_and_keeps_versions() {
        let (service, _) = service().await;
        let version = service
            .save_draft(PageId(1), 1, &content("accueil", "home", "Nouveau titre"))
            .await
            .unwrap();

        service.publish(PageId(1), version, OWNER).await.unwrap();

        let public = service.public_home().await.unwrap().unwrap();
        assert_eq!(public.content.settings.title.fr, "Nouveau titre");
        assert_eq!(service.versions(PageId(1)).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn f002_s06_stale_draft_version_is_refused() {
        let (service, _) = service().await;
        service
            .save_draft(PageId(1), 1, &content("accueil", "home", "Version A"))
            .await
            .unwrap();
        service
            .save_draft(PageId(1), 2, &content("accueil", "home", "Version B"))
            .await
            .unwrap();

        let result = service.publish(PageId(1), 2, OWNER).await;

        assert!(matches!(
            result,
            Err(PageError::VersionConflict { current: 3 })
        ));
        let public = service.public_home().await.unwrap().unwrap();
        assert_eq!(public.content.settings.title.fr, "Accueil");
    }

    #[tokio::test]
    async fn saving_with_an_old_version_is_refused() {
        let (service, _) = service().await;
        service
            .save_draft(PageId(1), 1, &content("accueil", "home", "A"))
            .await
            .unwrap();

        let result = service
            .save_draft(PageId(1), 1, &content("accueil", "home", "B"))
            .await;

        assert!(matches!(
            result,
            Err(PageError::VersionConflict { current: 2 })
        ));
    }

    #[tokio::test]
    async fn f002_s08_restore_puts_a_version_back_into_the_draft_only() {
        let (service, _) = service().await;
        let v = service
            .save_draft(PageId(1), 1, &content("accueil", "home", "Nouveau"))
            .await
            .unwrap();
        service.publish(PageId(1), v, OWNER).await.unwrap();

        service.restore(PageId(1), 1).await.unwrap();

        let page = service.get(PageId(1)).await.unwrap();
        assert_eq!(page.draft.settings.title.fr, "Accueil");
        assert_eq!(
            page.published.unwrap().settings.title.fr,
            "Nouveau",
            "restoring does not publish"
        );
    }

    #[tokio::test]
    async fn f002_s02_menu_lists_published_pages_in_order_home_first() {
        let (service, _) = service().await;
        let mut second = content("la-region", "the-area", "La région");
        second["settings"]["menu_order"] = json!(5);
        let mut first = content("le-gite", "the-cottage", "Le gîte");
        first["settings"]["menu_order"] = json!(3);
        let hidden = content("brouillon", "draft", "Jamais publiée");
        for raw in [&second, &first] {
            let id = service.create(raw).await.unwrap();
            service.publish(id, 1, OWNER).await.unwrap();
        }
        service.create(&hidden).await.unwrap();

        let titles: Vec<String> = service
            .menu()
            .await
            .unwrap()
            .into_iter()
            .map(|entry| entry.content.settings.title.fr)
            .collect();

        assert_eq!(titles, ["Accueil", "Le gîte", "La région"]);
    }

    #[tokio::test]
    async fn slugs_must_be_unique() {
        let (service, _) = service().await;

        let result = service
            .create(&content("accueil", "other", "Doublon"))
            .await;

        assert!(matches!(
            result,
            Err(PageError::SlugTaken { lang: Lang::Fr, .. })
        ));
    }

    #[tokio::test]
    async fn home_page_cannot_be_deleted() {
        let (service, _) = service().await;

        assert!(matches!(
            service.delete(PageId(1)).await,
            Err(PageError::HomeCannotBeDeleted)
        ));
    }

    #[tokio::test]
    async fn unpublished_page_is_not_public() {
        let (service, _) = service().await;
        service
            .create(&content("brouillon", "draft", "Jamais publiée"))
            .await
            .unwrap();

        assert!(
            service
                .public_page(Lang::Fr, "brouillon")
                .await
                .unwrap()
                .is_none()
        );
    }
}
