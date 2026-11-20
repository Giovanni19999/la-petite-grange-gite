//! File storage of the media library (F003, ADR-0017).
//!
//! `<root>/public/<hash>-<width>.webp` is served by the proxy; `<root>/originals/<hash>.<ext>`
//! is private. Files are written to a temporary name then renamed, so the proxy never serves
//! a half-written file, and an existing file is never overwritten.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::application::health::BoxFuture;
use crate::application::media::{MediaFiles, StoredMedia, is_known_extension};
use crate::domain::media::ContentHash;

/// Media files on the local file system (a Docker volume in production).
#[derive(Debug, Clone)]
pub struct FsMediaFiles {
    public: PathBuf,
    originals: PathBuf,
}

impl FsMediaFiles {
    /// Creates the `public` and `originals` directories under `root` if needed.
    ///
    /// # Errors
    ///
    /// The directories cannot be created.
    pub fn new(root: &Path) -> anyhow::Result<Self> {
        let public = root.join("public");
        let originals = root.join("originals");
        for dir in [&public, &originals] {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("cannot create media directory {}", dir.display()))?;
        }
        Ok(Self { public, originals })
    }

    /// Directory of the public files (served under the public URL).
    #[must_use]
    pub fn public_dir(&self) -> &Path {
        &self.public
    }

    fn original_path(&self, hash: &ContentHash, extension: &str) -> anyhow::Result<PathBuf> {
        anyhow::ensure!(
            is_known_extension(extension),
            "unexpected extension {extension}"
        );
        Ok(self.originals.join(format!("{hash}.{extension}")))
    }
}

async fn write_new(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if tokio::fs::try_exists(path).await? {
        return Ok(());
    }
    let temporary = path.with_extension("part");
    tokio::fs::write(&temporary, bytes)
        .await
        .with_context(|| format!("cannot write {}", temporary.display()))?;
    tokio::fs::rename(&temporary, path)
        .await
        .with_context(|| format!("cannot rename {}", temporary.display()))
}

async fn remove_if_present(path: &Path) -> anyhow::Result<()> {
    match tokio::fs::remove_file(path).await {
        Err(error) if error.kind() != ErrorKind::NotFound => {
            Err(error).with_context(|| format!("cannot remove {}", path.display()))
        }
        _ => Ok(()),
    }
}

impl MediaFiles for FsMediaFiles {
    fn save<'a>(
        &'a self,
        hash: &'a ContentHash,
        original_extension: &'a str,
        original: &'a [u8],
        variants: &'a [(u32, Vec<u8>)],
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            write_new(&self.original_path(hash, original_extension)?, original).await?;
            for (width, bytes) in variants {
                write_new(&self.public.join(hash.variant_file(*width)), bytes).await?;
            }
            Ok(())
        })
    }

    fn remove<'a>(&'a self, media: &'a StoredMedia) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            for width in &media.widths {
                remove_if_present(&self.public.join(media.hash.variant_file(*width))).await?;
            }
            remove_if_present(&self.original_path(&media.hash, &media.original_extension)?).await
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use time::macros::datetime;

    use super::*;
    use crate::domain::media::MediaDetails;
    use crate::domain::page::MediaId;

    #[tokio::test]
    async fn saves_public_versions_and_a_private_original_then_removes_them() {
        let root = tempfile::tempdir().unwrap();
        let files = FsMediaFiles::new(root.path()).unwrap();
        let hash = ContentHash::parse(&"b".repeat(64)).unwrap();

        files
            .save(
                &hash,
                "jpg",
                b"original",
                &[(480, b"small".to_vec()), (960, b"big".to_vec())],
            )
            .await
            .unwrap();
        let public = root.path().join("public");
        assert_eq!(
            std::fs::read(public.join(hash.variant_file(480))).unwrap(),
            b"small"
        );
        assert!(
            root.path()
                .join("originals")
                .join(format!("{hash}.jpg"))
                .exists()
        );
        assert!(
            !public.join(format!("{hash}.jpg")).exists(),
            "original is never public"
        );

        // An existing file is never overwritten.
        files
            .save(&hash, "jpg", b"other", &[(480, b"changed".to_vec())])
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(public.join(hash.variant_file(480))).unwrap(),
            b"small"
        );

        let media = StoredMedia {
            id: MediaId(1),
            hash: hash.clone(),
            original_extension: "jpg".into(),
            width: 960,
            height: 640,
            widths: vec![480, 960],
            details: MediaDetails::default(),
            created_at: datetime!(2026-11-20 10:00 UTC),
        };
        files.remove(&media).await.unwrap();
        assert_eq!(std::fs::read_dir(&public).unwrap().count(), 0);
        assert_eq!(
            std::fs::read_dir(root.path().join("originals"))
                .unwrap()
                .count(),
            0
        );
        // Removing twice is fine.
        files.remove(&media).await.unwrap();
    }
}
