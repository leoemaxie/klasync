use std::path::{Path, PathBuf};

use uuid::Uuid;

#[derive(Clone)]
pub struct LocalObjectStore {
    root: PathBuf,
}

pub struct StoredObject {
    pub key: String,
    pub bytes: usize,
}

impl LocalObjectStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub async fn put(&self, file_name: &str, data: &[u8]) -> Result<StoredObject, std::io::Error> {
        let extension = Path::new(file_name)
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| extension.len() <= 16)
            .unwrap_or("bin");
        let key = format!("uploads/{}.{}", Uuid::new_v4(), extension.to_ascii_lowercase());
        let destination = self.root.join(&key);
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(destination, data).await?;
        Ok(StoredObject { key, bytes: data.len() })
    }
}
