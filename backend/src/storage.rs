use std::{path::Path, sync::Arc};

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
    #[error("object storage is not configured")]
    Unconfigured,
}

pub struct StoredObject {
    pub key: String,
    pub bytes: usize,
}

#[async_trait]
pub trait StorageAdapter: Send + Sync {
    async fn put(&self, file_name: &str, data: Vec<u8>) -> Result<StoredObject, StorageError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
    fn provider_name(&self) -> &'static str;
}

/// Cloudflare R2 implementation using its S3-compatible API. `force_path_style`
/// keeps bucket routing compatible with R2 endpoints.
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
        let credentials =
            Credentials::new(access_key_id, secret_access_key, None, None, "klasync-r2");
        let configuration = aws_sdk_s3::config::Builder::new()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(endpoint)
            .credentials_provider(credentials)
            .force_path_style(true)
            .build();
        Self {
            client: S3Client::from_conf(configuration),
            bucket,
        }
    }

    fn object_key(file_name: &str) -> String {
        let extension = Path::new(file_name)
            .extension()
            .and_then(|value| value.to_str())
            .map(|ext| {
                ext.chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .take(10)
                    .collect::<String>()
                    .to_lowercase()
            })
            .filter(|ext| !ext.is_empty())
            .map(|ext| format!(".{ext}"))
            .unwrap_or_default();
        format!("uploads/{}{}", uuid::Uuid::now_v7(), extension)
    }
}

fn content_type_from_name(file_name: &str) -> &'static str {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".mp3") {
        "audio/mpeg"
    } else if lower.ends_with(".wav") {
        "audio/wav"
    } else if lower.ends_with(".m4a") || lower.ends_with(".aac") {
        "audio/mp4"
    } else if lower.ends_with(".ogg") || lower.ends_with(".opus") {
        "audio/ogg"
    } else if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".txt") {
        "text/plain"
    } else if lower.ends_with(".json") {
        "application/json"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

#[async_trait]
impl StorageAdapter for R2ObjectStore {
    async fn put(&self, file_name: &str, data: Vec<u8>) -> Result<StoredObject, StorageError> {
        let key = Self::object_key(file_name);
        let bytes = data.len();
        let content_type = content_type_from_name(file_name);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type(content_type)
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
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        let body = object
            .body
            .collect()
            .await
            .map_err(|error| StorageError::Backend(error.to_string()))?;
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

    fn provider_name(&self) -> &'static str {
        "cloudflare-r2"
    }
}

pub struct UnconfiguredStorageAdapter;

#[async_trait]
impl StorageAdapter for UnconfiguredStorageAdapter {
    async fn put(&self, _: &str, _: Vec<u8>) -> Result<StoredObject, StorageError> {
        Err(StorageError::Unconfigured)
    }

    async fn get(&self, _: &str) -> Result<Vec<u8>, StorageError> {
        Err(StorageError::Unconfigured)
    }

    async fn delete(&self, _: &str) -> Result<(), StorageError> {
        Err(StorageError::Unconfigured)
    }

    fn provider_name(&self) -> &'static str {
        "unconfigured"
    }
}


pub fn adapter_from_config(config: &AppConfig) -> SharedStorageAdapter {
    if config.r2_ready() {
        return Arc::new(R2ObjectStore::new(
            config
                .resolved_r2_endpoint()
                .expect("validated R2 endpoint"),
            config.r2_bucket.clone().expect("validated R2 bucket"),
            config
                .r2_access_key_id
                .clone()
                .expect("validated R2 access key"),
            config
                .r2_secret_access_key
                .clone()
                .expect("validated R2 secret key"),
        ));
    }
    Arc::new(UnconfiguredStorageAdapter)
}
