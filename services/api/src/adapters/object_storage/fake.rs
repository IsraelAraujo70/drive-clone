use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use url::Url;

use crate::application::ports::StorageError;
use crate::application::ports::object_storage::{ObjectMetadata, ObjectStorage, PresignedUrl};

#[derive(Debug, Clone, Default)]
pub struct FakeObjectStorage {
    objects: Arc<Mutex<HashMap<String, ObjectMetadata>>>,
}

impl FakeObjectStorage {
    pub fn put_object(&self, object_key: &str, content_length: i64) {
        self.objects
            .lock()
            .expect("fake storage mutex poisoned")
            .insert(object_key.to_string(), ObjectMetadata { content_length });
    }
}

#[async_trait]
impl ObjectStorage for FakeObjectStorage {
    async fn presign_put(
        &self,
        object_key: &str,
        _content_type: &str,
        _size_bytes: i64,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError> {
        presigned_fake_url("PUT", object_key, ttl_seconds)
    }

    async fn presign_get(
        &self,
        object_key: &str,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError> {
        presigned_fake_url("GET", object_key, ttl_seconds)
    }

    async fn head_object(&self, object_key: &str) -> Result<ObjectMetadata, StorageError> {
        self.objects
            .lock()
            .expect("fake storage mutex poisoned")
            .get(object_key)
            .cloned()
            .ok_or(StorageError::Unexpected)
    }
}

fn presigned_fake_url(
    method: &str,
    object_key: &str,
    ttl_seconds: i64,
) -> Result<PresignedUrl, StorageError> {
    let expires_at = Utc::now() + Duration::seconds(ttl_seconds);
    let mut url = Url::parse("http://storage.test/").map_err(|_| StorageError::Unexpected)?;
    url.path_segments_mut()
        .map_err(|_| StorageError::Unexpected)?
        .extend(object_key.split('/'));
    url.query_pairs_mut().append_pair("method", method);
    Ok(PresignedUrl {
        url: url.to_string(),
        expires_at,
    })
}
