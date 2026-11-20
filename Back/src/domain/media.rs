//! Media library rules (F003, ADR-0017): accepted formats, sizes to produce, focal point and
//! descriptive texts. No I/O here; limits and widths are received as settings (ADR-0010).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::page::Localized;

/// Maximum length of an alternative text or a caption, per language.
pub const MAX_TEXT_LEN: usize = 300;

/// Image formats accepted at upload, recognised by their first bytes (not by the file name).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Jpeg,
    Png,
    Webp,
}

impl SourceFormat {
    /// Recognises the format from the file signature.
    #[must_use]
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(Self::Jpeg)
        } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(Self::Webp)
        } else {
            None
        }
    }

    /// Extension of the kept original.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
}

/// Limits of an upload (technical settings with defaults).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadLimits {
    pub max_file_bytes: usize,
    pub max_files: usize,
    pub max_pixels: u64,
}

/// What is produced from an upload (technical settings with defaults).
#[derive(Debug, Clone, PartialEq)]
pub struct ImageSettings {
    /// Widths of the WebP versions, ascending.
    pub widths: Vec<u32>,
    /// WebP quality, 0 to 100.
    pub quality: f32,
}

/// Why an uploaded file is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UploadRefusal {
    #[error("format non accepté : envoyez une photo JPEG, PNG ou WebP")]
    UnsupportedFormat,
    #[error("fichier trop lourd : {max_mb} Mo au maximum")]
    TooHeavy { max_mb: usize },
    #[error("image trop grande : {max_megapixels} mégapixels au maximum")]
    TooManyPixels { max_megapixels: u64 },
    #[error("trop de fichiers envoyés en une fois : {max} au maximum")]
    TooManyFiles { max: usize },
    #[error("image illisible ou abîmée")]
    Unreadable,
}

impl UploadLimits {
    /// Checks what can be checked before decoding: format and weight.
    ///
    /// # Errors
    ///
    /// The reason why the file is refused.
    pub fn check_file(&self, bytes: &[u8]) -> Result<SourceFormat, UploadRefusal> {
        if bytes.len() > self.max_file_bytes {
            return Err(UploadRefusal::TooHeavy {
                max_mb: self.max_file_bytes / (1024 * 1024),
            });
        }
        SourceFormat::sniff(bytes).ok_or(UploadRefusal::UnsupportedFormat)
    }

    /// Checks the dimensions read from the file header, before decoding the pixels.
    ///
    /// # Errors
    ///
    /// [`UploadRefusal::TooManyPixels`] above the limit.
    pub fn check_dimensions(&self, width: u32, height: u32) -> Result<(), UploadRefusal> {
        if u64::from(width) * u64::from(height) > self.max_pixels {
            return Err(UploadRefusal::TooManyPixels {
                max_megapixels: self.max_pixels / 1_000_000,
            });
        }
        Ok(())
    }
}

/// Widths to produce for an image `original_width` wide: the configured widths not larger
/// than the original, or the original width alone when it is smaller than all of them.
#[must_use]
pub fn variant_widths(original_width: u32, configured: &[u32]) -> Vec<u32> {
    let mut widths: Vec<u32> = configured
        .iter()
        .copied()
        .filter(|width| *width <= original_width)
        .collect();
    widths.sort_unstable();
    widths.dedup();
    if widths.is_empty() {
        widths.push(original_width);
    }
    widths
}

/// The important part of a photo, in percent of its width and height (0 to 100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocalPoint {
    pub x: u8,
    pub y: u8,
}

impl Default for FocalPoint {
    fn default() -> Self {
        Self { x: 50, y: 50 }
    }
}

/// Texts and focal point of a media, as edited by the owners.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaDetails {
    #[serde(default)]
    pub alt: Localized,
    #[serde(default)]
    pub caption: Localized,
    #[serde(default)]
    pub focal: FocalPoint,
}

/// A rule broken by submitted media details.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DetailsError {
    #[error("le point focal doit être entre 0 et 100 %")]
    FocalPoint,
    #[error("texte trop long : {max} caractères au maximum", max = MAX_TEXT_LEN)]
    TooLong,
}

impl MediaDetails {
    /// Validates the details and removes any markup from the texts (plain text only).
    ///
    /// # Errors
    ///
    /// Every broken rule.
    pub fn validated(self) -> Result<Self, Vec<DetailsError>> {
        let mut errors = Vec::new();
        if self.focal.x > 100 || self.focal.y > 100 {
            errors.push(DetailsError::FocalPoint);
        }
        let texts = [
            &self.alt.fr,
            &self.alt.en,
            &self.caption.fr,
            &self.caption.en,
        ];
        if texts.iter().any(|text| text.chars().count() > MAX_TEXT_LEN) {
            errors.push(DetailsError::TooLong);
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let plain = |text: &Localized| Localized {
            fr: plain_text(&text.fr),
            en: plain_text(&text.en),
        };
        Ok(Self {
            alt: plain(&self.alt),
            caption: plain(&self.caption),
            focal: self.focal,
        })
    }

    /// The French alternative text is missing (flagged in the library).
    #[must_use]
    pub fn missing_alt(&self) -> bool {
        self.alt.fr.trim().is_empty()
    }
}

fn plain_text(text: &str) -> String {
    ammonia::Builder::empty()
        .clean(text)
        .to_string()
        .trim()
        .to_owned()
}

/// Name of the files of a media: content hash, so that files are never overwritten.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash(String);

impl ContentHash {
    /// Accepts only lower-case hexadecimal digests (they end up in file names).
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        (value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        .then(|| Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// File name of the WebP version `width` pixels wide.
    #[must_use]
    pub fn variant_file(&self, width: u32) -> String {
        format!("{}-{width}.webp", self.0)
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: UploadLimits = UploadLimits {
        max_file_bytes: 20 * 1024 * 1024,
        max_files: 20,
        max_pixels: 50_000_000,
    };

    #[test]
    fn f003_s03_recognises_accepted_formats_by_content() {
        assert_eq!(
            SourceFormat::sniff(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(SourceFormat::Jpeg)
        );
        assert_eq!(
            SourceFormat::sniff(b"\x89PNG\r\n\x1a\n...."),
            Some(SourceFormat::Png)
        );
        assert_eq!(
            SourceFormat::sniff(b"RIFF\0\0\0\0WEBPVP8 "),
            Some(SourceFormat::Webp)
        );
        assert_eq!(SourceFormat::sniff(b"GIF89a"), None);
        assert_eq!(SourceFormat::sniff(b"<svg xmlns"), None);
    }

    #[test]
    fn f003_s03_refuses_heavy_or_unknown_files() {
        assert_eq!(
            LIMITS.check_file(b"GIF89a"),
            Err(UploadRefusal::UnsupportedFormat)
        );
        let heavy = vec![0xFF; LIMITS.max_file_bytes + 1];
        assert_eq!(
            LIMITS.check_file(&heavy),
            Err(UploadRefusal::TooHeavy { max_mb: 20 })
        );
    }

    #[test]
    fn f003_s04_refuses_too_many_pixels() {
        assert!(LIMITS.check_dimensions(8000, 6000).is_ok());
        assert_eq!(
            LIMITS.check_dimensions(10_000, 10_000),
            Err(UploadRefusal::TooManyPixels { max_megapixels: 50 })
        );
    }

    #[test]
    fn f003_s01_widths_never_larger_than_the_original() {
        let configured = [480, 960, 1440, 1920];
        assert_eq!(
            variant_widths(4000, &configured),
            vec![480, 960, 1440, 1920]
        );
        assert_eq!(variant_widths(1200, &configured), vec![480, 960]);
        assert_eq!(variant_widths(300, &configured), vec![300]);
    }

    #[test]
    fn f003_s05_details_are_validated_and_plain_text() {
        let details = MediaDetails {
            alt: Localized {
                fr: " La <b>grange</b><script>x</script> ".into(),
                en: String::new(),
            },
            caption: Localized::default(),
            focal: FocalPoint { x: 30, y: 70 },
        }
        .validated()
        .unwrap();
        assert_eq!(details.alt.fr, "La grange");
        assert!(!details.missing_alt());

        let invalid = MediaDetails {
            focal: FocalPoint { x: 101, y: 0 },
            alt: Localized {
                fr: "x".repeat(MAX_TEXT_LEN + 1),
                en: String::new(),
            },
            ..MediaDetails::default()
        }
        .validated();
        assert_eq!(
            invalid,
            Err(vec![DetailsError::FocalPoint, DetailsError::TooLong])
        );
    }

    #[test]
    fn content_hash_only_accepts_hex_digests() {
        let hash = ContentHash::parse(&"a".repeat(64)).unwrap();
        assert_eq!(
            hash.variant_file(960),
            format!("{}-960.webp", "a".repeat(64))
        );
        assert!(ContentHash::parse("../../etc/passwd").is_none());
        assert!(ContentHash::parse(&"A".repeat(64)).is_none());
    }
}
