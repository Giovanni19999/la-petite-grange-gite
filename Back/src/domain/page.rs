//! Pages built from blocks (F002, ADR-0009): content model, validation, sanitisation,
//! language fallback and the summary of changes shown before publishing.

use std::collections::{BTreeSet, HashMap};
use std::fmt;

use serde::{Deserialize, Serialize};

/// Language of the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Fr,
    En,
}

impl Lang {
    pub const ALL: [Self; 2] = [Self::Fr, Self::En];
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Fr => "fr",
            Self::En => "en",
        })
    }
}

/// A text in French and English. English falls back to French when empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Localized {
    #[serde(default)]
    pub fr: String,
    #[serde(default)]
    pub en: String,
}

impl Localized {
    /// Text to show in `lang`, falling back to French.
    #[must_use]
    pub fn get(&self, lang: Lang) -> &str {
        match lang {
            Lang::En if !self.en.trim().is_empty() => &self.en,
            _ => &self.fr,
        }
    }

    fn raw(&self, lang: Lang) -> &str {
        match lang {
            Lang::Fr => &self.fr,
            Lang::En => &self.en,
        }
    }

    fn map(&self, transform: impl Fn(&str) -> String) -> Self {
        Self {
            fr: transform(&self.fr),
            en: transform(&self.en),
        }
    }
}

/// Identifier of an image of the media library (F003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MediaId(pub u64);

/// Background of a block: a role of the theme, never a raw color.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    #[default]
    Background,
    Muted,
    Primary,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroHeight {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextWidth {
    #[default]
    Narrow,
    Wide,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImagePosition {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ratio {
    #[default]
    #[serde(rename = "50-50")]
    Half,
    #[serde(rename = "33-67")]
    Third,
}

/// Hero background: one fixed photo or several photos that scroll (F024).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroMode {
    #[default]
    Image,
    Carousel,
}

/// Width of an image block (F024).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageWidth {
    Narrow,
    #[default]
    Wide,
    Full,
}

/// One item of a `columns` block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnItem {
    #[serde(default)]
    pub title: Localized,
    #[serde(default)]
    pub body: Localized,
}

/// Type-specific content and options of a block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockKind {
    Hero {
        #[serde(default)]
        image: Option<MediaId>,
        #[serde(default)]
        title: Localized,
        #[serde(default)]
        subtitle: Localized,
        #[serde(default)]
        button_label: Localized,
        #[serde(default)]
        button_link: Option<String>,
        #[serde(default)]
        align: Align,
        #[serde(default)]
        height: HeroHeight,
        /// Darkening of the image, in percent.
        #[serde(default)]
        darken: u8,
        /// Fixed photo (`image`) or carousel (`images`), F024.
        #[serde(default)]
        mode: HeroMode,
        #[serde(default)]
        images: Vec<MediaId>,
        #[serde(default = "default_true")]
        autoplay: bool,
        /// Seconds between two photos of the carousel.
        #[serde(default = "default_interval")]
        interval: u8,
    },
    Text {
        #[serde(default)]
        body: Localized,
        #[serde(default)]
        width: TextWidth,
        #[serde(default)]
        align: Align,
    },
    TextImage {
        #[serde(default)]
        body: Localized,
        #[serde(default)]
        image: Option<MediaId>,
        #[serde(default)]
        image_position: ImagePosition,
        #[serde(default)]
        ratio: Ratio,
    },
    Columns {
        items: Vec<ColumnItem>,
    },
    Gallery {
        #[serde(default)]
        images: Vec<MediaId>,
        #[serde(default = "default_gallery_columns")]
        columns: u8,
    },
    /// One photo with its caption (F024).
    Image {
        #[serde(default)]
        image: Option<MediaId>,
        #[serde(default)]
        width: ImageWidth,
    },
    /// Photos that scroll (F024).
    Carousel {
        #[serde(default)]
        images: Vec<MediaId>,
        #[serde(default)]
        height: HeroHeight,
        #[serde(default)]
        autoplay: bool,
        /// Seconds between two photos.
        #[serde(default = "default_interval")]
        interval: u8,
    },
}

fn default_gallery_columns() -> u8 {
    3
}

fn default_true() -> bool {
    true
}

fn default_interval() -> u8 {
    DEFAULT_INTERVAL
}

/// A block of a page. Unknown fields are rejected by [`BlockKind`], which receives every
/// field not used here (`deny_unknown_fields` does not combine with `flatten`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Stable identifier inside the page (used by the summary of changes).
    pub id: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub background: Background,
    #[serde(default)]
    pub spacing: Spacing,
    #[serde(default)]
    pub hide_on_mobile: bool,
    #[serde(flatten)]
    pub kind: BlockKind,
}

/// Settings of a page (per-language slugs, titles, meta descriptions, menu).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSettings {
    pub slug: Localized,
    pub title: Localized,
    #[serde(default)]
    pub meta_description: Localized,
    #[serde(default)]
    pub in_menu: bool,
    #[serde(default)]
    pub menu_order: u32,
}

/// Full content of a page: what is saved as draft, published and versioned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageContent {
    pub settings: PageSettings,
    #[serde(default)]
    pub blocks: Vec<Block>,
}

/// A rule broken by submitted page content.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContentError {
    #[error("contenu illisible : {0}")]
    Malformed(String),
    #[error("le titre en français est obligatoire")]
    MissingFrenchTitle,
    #[error("adresse de page invalide en {lang} : « {slug} » (minuscules, chiffres et tirets)")]
    InvalidSlug { lang: Lang, slug: String },
    #[error("identifiant de bloc invalide ou en double : « {0} »")]
    InvalidBlockId(String),
    #[error("le bloc « {0} » doit contenir de 2 à 4 colonnes")]
    ColumnCount(String),
    #[error("la galerie « {0} » doit avoir 2, 3 ou 4 colonnes")]
    GalleryColumns(String),
    #[error("l'assombrissement du bloc « {0} » doit être entre 0 et {max} %", max = MAX_DARKEN)]
    Darken(String),
    #[error("lien du bouton invalide dans le bloc « {0} »")]
    ButtonLink(String),
    #[error("le carrousel « {block} » doit contenir de {min} à {max} photos")]
    CarouselPhotos {
        block: String,
        min: usize,
        max: usize,
    },
    #[error("le délai du carrousel « {0} » doit être entre 3 et 15 secondes")]
    CarouselInterval(String),
}

/// Maximum darkening of a hero image (readability of the text on top).
pub const MAX_DARKEN: u8 = 80;
/// Number of photos of a hero carousel (F024).
pub const HERO_CAROUSEL_PHOTOS: std::ops::RangeInclusive<usize> = 2..=10;
/// Number of photos of a carousel block (F024).
pub const CAROUSEL_PHOTOS: std::ops::RangeInclusive<usize> = 2..=20;
/// Seconds between two photos of a carousel (F024).
pub const CAROUSEL_INTERVAL: std::ops::RangeInclusive<u8> = 3..=15;
const DEFAULT_INTERVAL: u8 = 6;
const SLUG_MAX_LEN: usize = 80;
const BLOCK_ID_MAX_LEN: usize = 40;

fn carousel_photos(block: &str, range: &std::ops::RangeInclusive<usize>) -> ContentError {
    ContentError::CarouselPhotos {
        block: block.to_owned(),
        min: *range.start(),
        max: *range.end(),
    }
}

/// Checks a slug: lower-case letters, digits and single hyphens.
#[must_use]
pub fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= SLUG_MAX_LEN
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
}

/// Removes everything but basic formatting and safe links from rich text.
#[must_use]
pub fn sanitize_rich_text(html: &str) -> String {
    ammonia::Builder::default()
        .tags(
            [
                "p", "br", "strong", "b", "em", "i", "ul", "ol", "li", "h2", "h3", "h4", "a",
            ]
            .into_iter()
            .collect(),
        )
        .url_schemes(["http", "https", "mailto", "tel"].into_iter().collect())
        .link_rel(Some("noopener noreferrer"))
        .clean(html)
        .to_string()
}

/// Removes every tag from plain-text fields (titles, labels).
fn strip_tags(text: &str) -> String {
    ammonia::Builder::empty().clean(text).to_string()
}

impl PageContent {
    /// Every media of the media library used by the blocks, without duplicates, in order.
    #[must_use]
    pub fn media_ids(&self) -> Vec<MediaId> {
        let mut ids = Vec::new();
        for block in &self.blocks {
            let used: Vec<MediaId> = match &block.kind {
                BlockKind::Hero { image, images, .. } => {
                    image.iter().chain(images.iter()).copied().collect()
                }
                BlockKind::TextImage { image, .. } | BlockKind::Image { image, .. } => {
                    image.iter().copied().collect()
                }
                BlockKind::Gallery { images, .. } | BlockKind::Carousel { images, .. } => {
                    images.clone()
                }
                BlockKind::Text { .. } | BlockKind::Columns { .. } => Vec::new(),
            };
            for id in used {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        ids
    }

    /// Parses, validates and sanitises submitted page content.
    ///
    /// # Errors
    ///
    /// Returns every broken rule found.
    pub fn parse(json: &serde_json::Value) -> Result<Self, Vec<ContentError>> {
        let content: Self = serde_json::from_value(json.clone())
            .map_err(|error| vec![ContentError::Malformed(error.to_string())])?;
        content.validate()?;
        Ok(content.sanitized())
    }

    fn validate(&self) -> Result<(), Vec<ContentError>> {
        let mut errors = Vec::new();
        let settings = &self.settings;
        if settings.title.fr.trim().is_empty() {
            errors.push(ContentError::MissingFrenchTitle);
        }
        for lang in Lang::ALL {
            let slug = settings.slug.raw(lang);
            if !is_valid_slug(slug) {
                errors.push(ContentError::InvalidSlug {
                    lang,
                    slug: slug.to_owned(),
                });
            }
        }
        let mut seen = BTreeSet::new();
        for block in &self.blocks {
            let id_ok = !block.id.is_empty()
                && block.id.len() <= BLOCK_ID_MAX_LEN
                && block
                    .id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
            if !id_ok || !seen.insert(block.id.as_str()) {
                errors.push(ContentError::InvalidBlockId(block.id.clone()));
            }
            match &block.kind {
                BlockKind::Columns { items } if !(2..=4).contains(&items.len()) => {
                    errors.push(ContentError::ColumnCount(block.id.clone()));
                }
                BlockKind::Gallery { columns, .. } if !(2..=4).contains(columns) => {
                    errors.push(ContentError::GalleryColumns(block.id.clone()));
                }
                BlockKind::Carousel {
                    images, interval, ..
                } => {
                    if !CAROUSEL_PHOTOS.contains(&images.len()) {
                        errors.push(carousel_photos(&block.id, &CAROUSEL_PHOTOS));
                    }
                    if !CAROUSEL_INTERVAL.contains(interval) {
                        errors.push(ContentError::CarouselInterval(block.id.clone()));
                    }
                }
                BlockKind::Hero {
                    darken,
                    button_link,
                    mode,
                    images,
                    interval,
                    ..
                } => {
                    if *darken > MAX_DARKEN {
                        errors.push(ContentError::Darken(block.id.clone()));
                    }
                    if *mode == HeroMode::Carousel {
                        if !HERO_CAROUSEL_PHOTOS.contains(&images.len()) {
                            errors.push(carousel_photos(&block.id, &HERO_CAROUSEL_PHOTOS));
                        }
                        if !CAROUSEL_INTERVAL.contains(interval) {
                            errors.push(ContentError::CarouselInterval(block.id.clone()));
                        }
                    }
                    if button_link
                        .as_deref()
                        .is_some_and(|link| !is_safe_link(link))
                    {
                        errors.push(ContentError::ButtonLink(block.id.clone()));
                    }
                }
                _ => {}
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn sanitized(mut self) -> Self {
        let plain = |text: &Localized| text.map(strip_tags);
        let rich = |text: &Localized| text.map(sanitize_rich_text);
        let settings = &mut self.settings;
        settings.title = plain(&settings.title);
        settings.meta_description = plain(&settings.meta_description);
        for block in &mut self.blocks {
            match &mut block.kind {
                BlockKind::Hero {
                    title,
                    subtitle,
                    button_label,
                    ..
                } => {
                    *title = plain(title);
                    *subtitle = plain(subtitle);
                    *button_label = plain(button_label);
                }
                BlockKind::Text { body, .. } | BlockKind::TextImage { body, .. } => {
                    *body = rich(body);
                }
                BlockKind::Columns { items } => {
                    for item in items {
                        item.title = plain(&item.title);
                        item.body = rich(&item.body);
                    }
                }
                BlockKind::Gallery { .. }
                | BlockKind::Image { .. }
                | BlockKind::Carousel { .. } => {}
            }
        }
        self
    }
}

/// Internal links (`/…`, `#…`) and http(s), mailto and tel links.
fn is_safe_link(link: &str) -> bool {
    (link.starts_with('/') && !link.starts_with("//"))
        || link.starts_with('#')
        || ["https://", "http://", "mailto:", "tel:"]
            .iter()
            .any(|prefix| link.starts_with(prefix))
}

impl Block {
    /// Languages in which the block has at least one non-empty text.
    fn languages_with_text(&self) -> impl Iterator<Item = Lang> + '_ {
        Lang::ALL
            .into_iter()
            .filter(|lang| self.texts(*lang).iter().any(|text| !text.trim().is_empty()))
    }

    /// Texts of the block in one language (stored value, without fallback).
    fn texts(&self, lang: Lang) -> Vec<&str> {
        match &self.kind {
            BlockKind::Hero {
                title,
                subtitle,
                button_label,
                ..
            } => vec![title.raw(lang), subtitle.raw(lang), button_label.raw(lang)],
            BlockKind::Text { body, .. } | BlockKind::TextImage { body, .. } => {
                vec![body.raw(lang)]
            }
            BlockKind::Columns { items } => items
                .iter()
                .flat_map(|item| [item.title.raw(lang), item.body.raw(lang)])
                .collect(),
            BlockKind::Gallery { .. } | BlockKind::Image { .. } | BlockKind::Carousel { .. } => {
                Vec::new()
            }
        }
    }
}

/// What changes between the published page and the draft (shown before publishing).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ChangeSummary {
    pub settings_changed: bool,
    pub blocks_added: Vec<String>,
    pub blocks_removed: Vec<String>,
    pub blocks_changed: Vec<String>,
    pub blocks_moved: bool,
    /// Languages whose texts changed (page settings or blocks).
    pub languages: BTreeSet<Lang>,
}

impl ChangeSummary {
    /// Compares the published content (if any) with the draft.
    #[must_use]
    pub fn between(published: Option<&PageContent>, draft: &PageContent) -> Self {
        let Some(published) = published else {
            return Self {
                settings_changed: true,
                blocks_added: draft.blocks.iter().map(|block| block.id.clone()).collect(),
                languages: Lang::ALL.into_iter().collect(),
                ..Self::default()
            };
        };
        let mut summary = Self {
            settings_changed: published.settings != draft.settings,
            ..Self::default()
        };
        for lang in Lang::ALL {
            let before = &published.settings;
            let after = &draft.settings;
            if before.title.raw(lang) != after.title.raw(lang)
                || before.meta_description.raw(lang) != after.meta_description.raw(lang)
                || before.slug.raw(lang) != after.slug.raw(lang)
            {
                summary.languages.insert(lang);
            }
        }

        let old: HashMap<&str, &Block> = published
            .blocks
            .iter()
            .map(|block| (block.id.as_str(), block))
            .collect();
        let new: HashMap<&str, &Block> = draft
            .blocks
            .iter()
            .map(|block| (block.id.as_str(), block))
            .collect();
        for block in &draft.blocks {
            match old.get(block.id.as_str()) {
                None => {
                    summary.blocks_added.push(block.id.clone());
                    summary.languages.extend(block.languages_with_text());
                }
                Some(previous) if *previous != block => {
                    summary.blocks_changed.push(block.id.clone());
                    for lang in Lang::ALL {
                        if previous.texts(lang) != block.texts(lang) {
                            summary.languages.insert(lang);
                        }
                    }
                }
                Some(_) => {}
            }
        }
        for block in &published.blocks {
            if !new.contains_key(block.id.as_str()) {
                summary.blocks_removed.push(block.id.clone());
                summary.languages.extend(block.languages_with_text());
            }
        }
        let kept_order = |blocks: &[Block], other: &HashMap<&str, &Block>| -> Vec<String> {
            blocks
                .iter()
                .filter(|block| other.contains_key(block.id.as_str()))
                .map(|block| block.id.clone())
                .collect()
        };
        summary.blocks_moved =
            kept_order(&published.blocks, &new) != kept_order(&draft.blocks, &old);
        summary
    }

    /// `true` when publishing would change nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.settings_changed
            && self.blocks_added.is_empty()
            && self.blocks_removed.is_empty()
            && self.blocks_changed.is_empty()
            && !self.blocks_moved
    }
}

#[cfg(test)]
#[allow(clippy::needless_pass_by_value)] // test helpers take owned JSON for readability
mod tests {
    use serde_json::json;

    use super::*;

    fn page(blocks: serde_json::Value) -> serde_json::Value {
        json!({
            "settings": {
                "slug": { "fr": "le-gite", "en": "the-cottage" },
                "title": { "fr": "Le gîte", "en": "The cottage" },
                "in_menu": true,
                "menu_order": 1
            },
            "blocks": blocks
        })
    }

    fn text(id: &str, fr: &str, en: &str) -> serde_json::Value {
        json!({ "id": id, "type": "text", "body": { "fr": fr, "en": en } })
    }

    #[test]
    fn parses_every_block_type_with_defaults() {
        let content = PageContent::parse(&page(json!([
            { "id": "h", "type": "hero", "title": { "fr": "Bienvenue" }, "darken": 30 },
            text("t", "<p>Bonjour</p>", ""),
            { "id": "ti", "type": "text_image", "image": 4, "image_position": "left", "ratio": "33-67" },
            { "id": "c", "type": "columns", "items": [ {}, {} ] },
            { "id": "g", "type": "gallery", "images": [1, 2], "columns": 4 }
        ])))
        .unwrap();

        assert_eq!(content.blocks.len(), 5);
        assert!(content.blocks[0].visible);
        assert_eq!(content.blocks[0].background, Background::Background);
    }

    #[test]
    fn english_falls_back_to_french() {
        let text = Localized {
            fr: "Bonjour".into(),
            en: "  ".into(),
        };
        assert_eq!(text.get(Lang::En), "Bonjour");
        assert_eq!(text.get(Lang::Fr), "Bonjour");
    }

    #[test]
    fn f002_s09_unknown_block_type_is_rejected() {
        let errors =
            PageContent::parse(&page(json!([{ "id": "x", "type": "video" }]))).unwrap_err();

        assert!(matches!(errors[0], ContentError::Malformed(_)));
    }

    #[test]
    fn f002_s09_unknown_option_is_rejected() {
        let errors = PageContent::parse(&page(json!([
            { "id": "t", "type": "text", "color": "#ff0000" }
        ])))
        .unwrap_err();

        assert!(matches!(errors[0], ContentError::Malformed(_)));
    }

    #[test]
    fn f002_s09_scripts_and_unsafe_links_are_removed() {
        let content = PageContent::parse(&page(json!([text(
            "t",
            "<p onclick=\"x()\">Hi<script>alert(1)</script> <a href=\"javascript:alert(1)\">x</a> <a href=\"https://example.test\">ok</a></p>",
            ""
        )])))
        .unwrap();

        let BlockKind::Text { body, .. } = &content.blocks[0].kind else {
            panic!("text block expected");
        };
        assert!(!body.fr.contains("script"));
        assert!(!body.fr.contains("onclick"));
        assert!(!body.fr.contains("javascript"));
        assert!(body.fr.contains("href=\"https://example.test\""));
    }

    #[test]
    fn titles_are_plain_text() {
        let mut raw = page(json!([]));
        raw["settings"]["title"]["fr"] = json!("<b>Le</b> gîte<script>x</script>");

        let content = PageContent::parse(&raw).unwrap();

        assert_eq!(content.settings.title.fr, "Le gîte");
    }

    #[test]
    fn invalid_settings_and_blocks_are_all_reported() {
        let mut raw = page(json!([
            { "id": "c", "type": "columns", "items": [ {} ] },
            { "id": "c", "type": "gallery", "columns": 6 },
            { "id": "h", "type": "hero", "darken": 95, "button_link": "javascript:x" }
        ]));
        raw["settings"]["title"]["fr"] = json!(" ");
        raw["settings"]["slug"]["en"] = json!("The Cottage");

        let errors = PageContent::parse(&raw).unwrap_err();

        assert!(errors.contains(&ContentError::MissingFrenchTitle));
        assert!(errors.contains(&ContentError::InvalidSlug {
            lang: Lang::En,
            slug: "The Cottage".into()
        }));
        assert!(errors.contains(&ContentError::ColumnCount("c".into())));
        assert!(errors.contains(&ContentError::InvalidBlockId("c".into())));
        assert!(errors.contains(&ContentError::GalleryColumns("c".into())));
        assert!(errors.contains(&ContentError::Darken("h".into())));
        assert!(errors.contains(&ContentError::ButtonLink("h".into())));
    }

    #[test]
    fn slugs() {
        for slug in ["accueil", "la-region", "acces-2"] {
            assert!(is_valid_slug(slug), "{slug}");
        }
        for slug in ["", "Accès", "-a", "a-", "a--b", "a b", "a/b"] {
            assert!(!is_valid_slug(slug), "{slug}");
        }
    }

    #[test]
    fn f002_s07_summary_lists_added_removed_and_changed_blocks() {
        let published = PageContent::parse(&page(json!([
            text("a", "Un", "One"),
            text("b", "Deux", "Two"),
            text("c", "Trois", "Three")
        ])))
        .unwrap();
        let draft = PageContent::parse(&page(json!([
            text("a", "Un", "One, changed"),
            text("c", "Trois", "Three"),
            text("d", "Quatre", "")
        ])))
        .unwrap();

        let summary = ChangeSummary::between(Some(&published), &draft);

        assert_eq!(summary.blocks_added, vec!["d"]);
        assert_eq!(summary.blocks_removed, vec!["b"]);
        assert_eq!(summary.blocks_changed, vec!["a"]);
        assert!(!summary.blocks_moved);
        assert!(!summary.settings_changed);
        // "a" changed in English, "d" added with French text, "b" removed (French and English).
        assert_eq!(summary.languages, Lang::ALL.into_iter().collect());
    }

    #[test]
    fn f002_s07_summary_detects_moves_and_settings() {
        let published =
            PageContent::parse(&page(json!([text("a", "Un", ""), text("b", "Deux", "")]))).unwrap();
        let mut raw = page(json!([text("b", "Deux", ""), text("a", "Un", "")]));
        raw["settings"]["title"]["fr"] = json!("Notre gîte");
        let draft = PageContent::parse(&raw).unwrap();

        let summary = ChangeSummary::between(Some(&published), &draft);

        assert!(summary.blocks_moved);
        assert!(summary.settings_changed);
        assert_eq!(summary.languages, [Lang::Fr].into_iter().collect());
    }

    #[test]
    fn identical_content_has_an_empty_summary() {
        let content = PageContent::parse(&page(json!([text("a", "Un", "")]))).unwrap();

        assert!(ChangeSummary::between(Some(&content), &content).is_empty());
    }

    #[test]
    fn never_published_page_is_all_new() {
        let content = PageContent::parse(&page(json!([text("a", "Un", "")]))).unwrap();

        let summary = ChangeSummary::between(None, &content);

        assert_eq!(summary.blocks_added, vec!["a"]);
        assert!(summary.settings_changed);
    }

    #[test]
    fn f024_s01_existing_hero_is_a_fixed_image() {
        let content = PageContent::parse(&page(json!([
            { "id": "h", "type": "hero", "image": 3, "title": { "fr": "Titre" } }
        ])))
        .unwrap();
        let BlockKind::Hero {
            mode,
            images,
            autoplay,
            interval,
            ..
        } = &content.blocks[0].kind
        else {
            panic!("hero expected");
        };
        assert_eq!(*mode, HeroMode::Image);
        assert_eq!(*images, Vec::<MediaId>::new());
        assert!(*autoplay, "a hero carousel scrolls by default");
        assert_eq!(*interval, 6);
    }

    #[test]
    fn f024_s04_s05_image_and_carousel_blocks() {
        let content = PageContent::parse(&page(json!([
            { "id": "i", "type": "image", "image": 4, "width": "full" },
            { "id": "c", "type": "carousel", "images": [5, 6, 7, 8], "height": "large" }
        ])))
        .unwrap();
        assert_eq!(
            content.blocks[0].kind,
            BlockKind::Image {
                image: Some(MediaId(4)),
                width: ImageWidth::Full
            }
        );
        let BlockKind::Carousel {
            autoplay, interval, ..
        } = &content.blocks[1].kind
        else {
            panic!("carousel expected");
        };
        assert!(!autoplay, "a carousel block does not scroll by default");
        assert_eq!(*interval, 6);
        assert_eq!(
            content.media_ids(),
            vec![MediaId(4), MediaId(5), MediaId(6), MediaId(7), MediaId(8)]
        );
    }

    #[test]
    fn f024_s06_invalid_carousels_are_refused() {
        let errors = PageContent::parse(&page(json!([
            { "id": "c", "type": "carousel", "images": [1] },
            { "id": "h", "type": "hero", "mode": "carousel",
              "images": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], "interval": 20 }
        ])))
        .unwrap_err();
        assert!(errors.contains(&ContentError::CarouselPhotos {
            block: "c".into(),
            min: 2,
            max: 20
        }));
        assert!(errors.contains(&ContentError::CarouselPhotos {
            block: "h".into(),
            min: 2,
            max: 10
        }));
        assert!(errors.contains(&ContentError::CarouselInterval("h".into())));
        assert_eq!(
            errors
                .iter()
                .find(|error| matches!(error, ContentError::CarouselPhotos { block, .. } if block == "c"))
                .unwrap()
                .to_string(),
            "le carrousel « c » doit contenir de 2 à 20 photos"
        );
    }

    #[test]
    fn f024_hero_in_image_mode_ignores_the_carousel_rules() {
        assert!(
            PageContent::parse(&page(json!([
                { "id": "h", "type": "hero", "mode": "image", "images": [1], "interval": 99 }
            ])))
            .is_ok()
        );
    }
}
