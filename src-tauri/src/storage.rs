use std::{path::{Path, PathBuf}, sync::Arc};

use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::{primitives::ByteStream, Client as S3Client};
use aws_types::region::Region;
use thiserror::Error;

use crate::config::AppConfig;

pub type SharedStorageAdapter = Arc<dyn StorageAdapter>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("object storage operation failed: {0}")]
    Backend(String),
    #[error("object key is invalid")]
    InvalidKey,
}

#[async_trait]
pub trait StorageAdapter: Send + Sync {
    async fn put(&self, file_name: &str, data: Vec<u8>) -> Result<StoredObject, StorageError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
    fn provider_name(&self) -> &'static str;
}

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

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, std::io::Error> {
        if key.contains("..") || key.starts_with('/') || key.starts_with('\\') {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid object key"));
        }
        tokio::fs::read(self.root.join(key)).await
    }
}

#[async_trait]
impl StorageAdapter for LocalObjectStore {
    async fn put(&self, file_name: &str, data: Vec<u8>) -> Result<StoredObject, StorageError> {
        LocalObjectStore::put(self, file_name, &data)
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        LocalObjectStore::get(self, key)
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        if key.contains("..") || Path::new(key).is_absolute() {
            return Err(StorageError::InvalidKey);
        }
        tokio::fs::remove_file(self.root.join(key))
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))
    }

    fn provider_name(&self) -> &'static str { "local-development" }
}

/// Cloudflare R2 implementation using its S3-compatible API. `force_path_style`
/// keeps bucket routing compatible with R2 endpoints and custom local endpoints.
pub struct R2ObjectStore {
    client: S3Client,
    bucket: String,
}

impl R2ObjectStore {
    pub fn new(
        endpoint: String,
        bucket: String,
        access_key_id: String,
        secret_access_key: String,
    ) -> Self {
        let credentials = Credentials::new(access_key_id, secret_access_key, None, None, "klasync-r2");
        let configuration = aws_sdk_s3::config::Builder::new()
            .region(Region::new("auto"))
            .endpoint_url(endpoint)
            .credentials_provider(credentials)
            .force_path_style(true)
            .build();
        Self { client: S3Client::from_conf(configuration), bucket }
    }

    fn object_key(file_name: &str) -> String {
        let extension = Path::new(file_name)
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .map(|value| format!(".{value}"))
            .unwrap_or_default();
        format!("uploads/{}{}", uuid::Uuid::new_v4(), extension)
    }
}

#[async_trait]
impl StorageAdapter for R2ObjectStore {
    async fn put(&self, file_name: &str, data: Vec<u8>) -> Result<StoredObject, StorageError> {
        let key = Self::object_key(file_name);
        let bytes = data.len();
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        Ok(StoredObject { key, bytes })
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        if key.is_empty() || key.contains("..") {
            return Err(StorageError::InvalidKey);
        }
        let object = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        let body = object.body.collect().await.map_err(|error| StorageError::Backend(error.to_string()))?;
        Ok(body.into_bytes().to_vec())
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        if key.is_empty() || key.contains("..") {
            return Err(StorageError::InvalidKey);
        }
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        Ok(())
    }

    fn provider_name(&self) -> &'static str { "cloudflare-r2" }
}

pub fn adapter_from_config(config: &AppConfig) -> SharedStorageAdapter {
    if config.r2_ready() {
        // r2_ready guarantees every one of these values is populated.
        return Arc::new(R2ObjectStore::new(
            config.resolved_r2_endpoint().expect("validated R2 endpoint"),
            config.r2_bucket.clone().expect("validated R2 bucket"),
            config.r2_access_key_id.clone().expect("validated R2 access key"),
            config.r2_secret_access_key.clone().expect("validated R2 secret key"),
        ));
    }
    Arc::new(LocalObjectStore::new(&config.object_storage_dir))
}
