//! Private documents (F007, ADR-0020): `<media root>/documents/<random>.<ext>`, never mounted
//! in Caddy; read by the API to attach them to emails.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::Context;

use crate::application::health::BoxFuture;
use crate::application::messaging::DocumentStore;

/// Extensions a private document may have.
const EXTENSIONS: [&str; 4] = ["pdf", "png", "jpg", "webp"];

/// A stored name: 32 hexadecimal digits, a dot and a known extension (no path).
fn valid_name(file: &str) -> bool {
    file.split_once('.').is_some_and(|(stem, extension)| {
        stem.len() == 32
            && stem.bytes().all(|b| b.is_ascii_hexdigit())
            && EXTENSIONS.contains(&extension)
    })
}

fn new_name(extension: &str) -> anyhow::Result<String> {
    anyhow::ensure!(
        EXTENSIONS.contains(&extension),
        "unexpected document extension {extension}"
    );
    let mut random = [0_u8; 16];
    rand::fill(&mut random);
    Ok(format!("{}.{extension}", hex::encode(random)))
}

/// Documents on the local file system (the media volume in production).
#[derive(Debug, Clone)]
pub struct FsDocuments {
    dir: PathBuf,
}

impl FsDocuments {
    /// Creates `<root>/documents` if needed.
    ///
    /// # Errors
    ///
    /// The directory cannot be created.
    pub fn new(root: &Path) -> anyhow::Result<Self> {
        let dir = root.join("documents");
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create documents directory {}", dir.display()))?;
        Ok(Self { dir })
    }

    fn path(&self, file: &str) -> anyhow::Result<PathBuf> {
        anyhow::ensure!(valid_name(file), "unexpected document name");
        Ok(self.dir.join(file))
    }
}

impl DocumentStore for FsDocuments {
    fn save<'a>(
        &'a self,
        bytes: &'a [u8],
        extension: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<String>> {
        Box::pin(async move {
            let name = new_name(extension)?;
            let path = self.path(&name)?;
            let temporary = path.with_extension("part");
            tokio::fs::write(&temporary, bytes)
                .await
                .with_context(|| format!("cannot write {}", temporary.display()))?;
            tokio::fs::rename(&temporary, &path)
                .await
                .with_context(|| format!("cannot rename {}", temporary.display()))?;
            Ok(name)
        })
    }

    fn read<'a>(&'a self, file: &'a str) -> BoxFuture<'a, anyhow::Result<Option<Vec<u8>>>> {
        Box::pin(async move {
            match tokio::fs::read(self.path(file)?).await {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error).context("cannot read a document"),
            }
        })
    }

    fn delete<'a>(&'a self, file: &'a str) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            match tokio::fs::remove_file(self.path(file)?).await {
                Err(error) if error.kind() != ErrorKind::NotFound => {
                    Err(error).context("cannot remove a document")
                }
                _ => Ok(()),
            }
        })
    }
}

/// Documents kept in memory: tests only.
#[derive(Debug, Clone, Default)]
pub struct MemoryDocuments {
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl DocumentStore for MemoryDocuments {
    fn save<'a>(
        &'a self,
        bytes: &'a [u8],
        extension: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<String>> {
        Box::pin(async move {
            let name = new_name(extension)?;
            self.files
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(name.clone(), bytes.to_vec());
            Ok(name)
        })
    }

    fn read<'a>(&'a self, file: &'a str) -> BoxFuture<'a, anyhow::Result<Option<Vec<u8>>>> {
        Box::pin(async move {
            Ok(self
                .files
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(file)
                .cloned())
        })
    }

    fn delete<'a>(&'a self, file: &'a str) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            self.files
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(file);
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn documents_are_saved_read_and_deleted_by_their_random_name() {
        let root = tempfile::tempdir().unwrap();
        let documents = FsDocuments::new(root.path()).unwrap();
        let name = documents.save(b"%PDF-1.4", "pdf").await.unwrap();
        assert!(valid_name(&name));
        assert_eq!(documents.read(&name).await.unwrap().unwrap(), b"%PDF-1.4");
        documents.delete(&name).await.unwrap();
        assert_eq!(documents.read(&name).await.unwrap(), None);
    }

    #[tokio::test]
    async fn names_with_a_path_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let documents = FsDocuments::new(root.path()).unwrap();
        assert!(documents.read("../secret.pdf").await.is_err());
        assert!(documents.save(b"x", "exe").await.is_err());
    }
}
