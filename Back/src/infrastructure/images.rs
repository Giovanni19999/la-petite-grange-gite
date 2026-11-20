//! Image processing adapter (F003, ADR-0017): decoding with the pure Rust `image` crate,
//! EXIF orientation, resizing, lossy WebP with libwebp. Metadata is never copied.

use std::io::Cursor;

use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader, Limits};

use crate::application::media::{ImageProcessor, ProcessedImage};
use crate::domain::media::{ImageSettings, UploadLimits, UploadRefusal, variant_widths};

/// Bytes per pixel of the largest decoded buffer (RGBA, 16 bits per channel).
const MAX_BYTES_PER_PIXEL: u64 = 8;

/// [`ImageProcessor`] built on `image` and `webp`.
#[derive(Debug, Clone, Copy, Default)]
pub struct WebpImageProcessor;

fn reader(bytes: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, UploadRefusal> {
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| UploadRefusal::Unreadable)
}

fn encode(image: &DynamicImage, quality: f32) -> Vec<u8> {
    let (width, height) = (image.width(), image.height());
    if image.color().has_alpha() {
        let rgba = image.to_rgba8();
        webp::Encoder::from_rgba(&rgba, width, height)
            .encode(quality)
            .to_vec()
    } else {
        let rgb = image.to_rgb8();
        webp::Encoder::from_rgb(&rgb, width, height)
            .encode(quality)
            .to_vec()
    }
}

impl ImageProcessor for WebpImageProcessor {
    fn process(
        &self,
        bytes: &[u8],
        limits: &UploadLimits,
        settings: &ImageSettings,
    ) -> Result<ProcessedImage, UploadRefusal> {
        // Dimensions come from the header: oversized images are refused before decoding.
        let (width, height) = reader(bytes)?
            .into_dimensions()
            .map_err(|_| UploadRefusal::Unreadable)?;
        limits.check_dimensions(width, height)?;

        let mut decoding_limits = Limits::default();
        decoding_limits.max_image_width = Some(width);
        decoding_limits.max_image_height = Some(height);
        decoding_limits.max_alloc = Some(limits.max_pixels.saturating_mul(MAX_BYTES_PER_PIXEL));
        let mut reader = reader(bytes)?;
        reader.limits(decoding_limits);
        let mut decoder = reader
            .into_decoder()
            .map_err(|_| UploadRefusal::Unreadable)?;
        let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
        let mut image =
            DynamicImage::from_decoder(decoder).map_err(|_| UploadRefusal::Unreadable)?;
        image.apply_orientation(orientation);

        let (width, height) = (image.width(), image.height());
        let variants = variant_widths(width, &settings.widths)
            .into_iter()
            .map(|target| {
                let resized = if target == width {
                    image.clone()
                } else {
                    let target_height = scaled_height(width, height, target);
                    image.resize_exact(target, target_height, FilterType::Lanczos3)
                };
                (target, encode(&resized, settings.quality))
            })
            .collect();
        Ok(ProcessedImage {
            width,
            height,
            variants,
        })
    }
}

/// Height keeping the aspect ratio, at least one pixel.
fn scaled_height(width: u32, height: u32, target_width: u32) -> u32 {
    let scaled =
        (u64::from(height) * u64::from(target_width) + u64::from(width) / 2) / u64::from(width);
    u32::try_from(scaled.max(1)).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use image::codecs::jpeg::JpegEncoder;
    use image::{ImageEncoder, Rgb, RgbImage};

    use super::*;

    const LIMITS: UploadLimits = UploadLimits {
        max_file_bytes: 20 * 1024 * 1024,
        max_files: 20,
        max_pixels: 50_000_000,
    };

    fn settings() -> ImageSettings {
        ImageSettings {
            widths: vec![20, 40, 1920],
            quality: 80.0,
        }
    }

    /// A JPEG `width` x `height`, red on the left half, blue on the right half.
    fn jpeg(width: u32, height: u32) -> Vec<u8> {
        let image = RgbImage::from_fn(width, height, |x, _| {
            if x < width / 2 {
                Rgb([255, 0, 0])
            } else {
                Rgb([0, 0, 255])
            }
        });
        let mut bytes = Vec::new();
        JpegEncoder::new_with_quality(&mut bytes, 90)
            .write_image(
                image.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        bytes
    }

    /// Inserts an EXIF segment with an orientation and a GPS position, like a phone photo.
    fn with_exif(jpeg: &[u8], orientation: u16) -> Vec<u8> {
        let mut tiff: Vec<u8> = b"MM\0\x2A\0\0\0\x08".to_vec();
        // IFD0: 2 entries (Orientation, GPS IFD pointer), then no next IFD.
        tiff.extend([0, 2]);
        tiff.extend([0x01, 0x12, 0, 3, 0, 0, 0, 1]);
        tiff.extend(orientation.to_be_bytes());
        tiff.extend([0, 0]);
        tiff.extend([0x88, 0x25, 0, 4, 0, 0, 0, 1, 0, 0, 0, 38]);
        tiff.extend([0, 0, 0, 0]);
        // GPS IFD: GPSVersionID only, enough to mark a GPS position.
        tiff.extend([0, 1, 0, 0, 0, 1, 0, 0, 0, 4, 2, 2, 0, 0, 0, 0, 0, 0]);
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend(tiff);
        let length = u16::try_from(payload.len() + 2).unwrap();
        let mut out = jpeg[..2].to_vec();
        out.extend([0xFF, 0xE1]);
        out.extend(length.to_be_bytes());
        out.extend(payload);
        out.extend(&jpeg[2..]);
        out
    }

    fn decode_webp(bytes: &[u8]) -> DynamicImage {
        image::load_from_memory_with_format(bytes, image::ImageFormat::WebP).unwrap()
    }

    #[test]
    fn f003_s01_produces_webp_versions_not_wider_than_the_original() {
        let processed = WebpImageProcessor
            .process(&jpeg(60, 30), &LIMITS, &settings())
            .unwrap();
        assert_eq!((processed.width, processed.height), (60, 30));
        let widths: Vec<u32> = processed.variants.iter().map(|(w, _)| *w).collect();
        assert_eq!(widths, vec![20, 40]);
        let small = decode_webp(&processed.variants[0].1);
        assert_eq!((small.width(), small.height()), (20, 10));
    }

    #[test]
    fn f003_s02_applies_orientation_and_drops_metadata() {
        // Orientation 6: the camera was turned, the picture must be rotated 90° clockwise.
        let processed = WebpImageProcessor
            .process(&with_exif(&jpeg(60, 30), 6), &LIMITS, &settings())
            .unwrap();
        assert_eq!((processed.width, processed.height), (30, 60));
        for (_, webp) in &processed.variants {
            assert!(webp.starts_with(b"RIFF") && &webp[8..12] == b"WEBP");
            assert!(
                !webp
                    .windows(4)
                    .any(|chunk| chunk == b"EXIF" || chunk == b"XMP "),
                "no metadata chunk"
            );
        }
        // Red was on the left: after a clockwise rotation it is at the top.
        let image = decode_webp(&processed.variants[0].1).to_rgb8();
        let top = image.get_pixel(image.width() / 2, 1);
        assert!(top[0] > 200 && top[2] < 80, "top pixel {top:?}");
    }

    #[test]
    fn f003_s04_refuses_too_many_pixels_from_the_header() {
        let limits = UploadLimits {
            max_pixels: 1000,
            ..LIMITS
        };
        assert_eq!(
            WebpImageProcessor.process(&jpeg(60, 30), &limits, &settings()),
            Err(UploadRefusal::TooManyPixels { max_megapixels: 0 })
        );
    }

    #[test]
    fn f003_s03_refuses_unreadable_files() {
        let mut broken = jpeg(60, 30);
        broken.truncate(40);
        assert_eq!(
            WebpImageProcessor.process(&broken, &LIMITS, &settings()),
            Err(UploadRefusal::Unreadable)
        );
    }
}
