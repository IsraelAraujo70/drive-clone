use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use url::Url;

use crate::application::ports::StorageError;
use crate::application::ports::object_storage::{
    CompletedUploadPart, ObjectMetadata, ObjectStorage, PresignedUrl, StoredObject,
};

#[derive(Debug, Clone, Default)]
pub struct FakeObjectStorage {
    objects: Arc<Mutex<HashMap<String, StoredObject>>>,
    multipart_uploads: Arc<Mutex<HashMap<String, String>>>,
    completed_multipart: Arc<Mutex<Vec<(String, String, Vec<CompletedUploadPart>)>>>,
    aborted_multipart: Arc<Mutex<Vec<(String, String)>>>,
    delete_failures: Arc<Mutex<HashSet<String>>>,
    deleted_objects: Arc<Mutex<Vec<String>>>,
}

impl FakeObjectStorage {
    pub fn put_object(&self, object_key: &str, content_length: i64) {
        self.put_object_with_last_modified(object_key, content_length, Utc::now());
    }

    pub fn put_object_with_last_modified(
        &self,
        object_key: &str,
        content_length: i64,
        last_modified: DateTime<Utc>,
    ) {
        self.objects
            .lock()
            .expect("fake storage mutex poisoned")
            .insert(
                object_key.to_string(),
                StoredObject {
                    key: object_key.to_string(),
                    last_modified,
                    size_bytes: content_length,
                },
            );
    }

    pub fn fail_delete(&self, object_key: &str) {
        self.delete_failures
            .lock()
            .expect("fake storage mutex poisoned")
            .insert(object_key.to_string());
    }

    pub fn has_object(&self, object_key: &str) -> bool {
        self.objects
            .lock()
            .expect("fake storage mutex poisoned")
            .contains_key(object_key)
    }

    pub fn deleted_objects(&self) -> Vec<String> {
        self.deleted_objects
            .lock()
            .expect("fake storage mutex poisoned")
            .clone()
    }

    pub fn multipart_completions(&self) -> Vec<(String, String, Vec<CompletedUploadPart>)> {
        self.completed_multipart
            .lock()
            .expect("fake storage mutex poisoned")
            .clone()
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
            .map(|object| ObjectMetadata {
                content_length: object.size_bytes,
            })
            .ok_or(StorageError::NotFound)
    }

    async fn create_multipart_upload(
        &self,
        object_key: &str,
        _content_type: &str,
    ) -> Result<String, StorageError> {
        let upload_id = format!("fake-upload-{object_key}");
        self.multipart_uploads
            .lock()
            .expect("fake storage mutex poisoned")
            .insert(object_key.to_string(), upload_id.clone());
        Ok(upload_id)
    }

    async fn presign_upload_part(
        &self,
        object_key: &str,
        upload_id: &str,
        part_number: i32,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError> {
        let mut presigned = presigned_fake_url("PUT", object_key, ttl_seconds)?;
        let mut url = Url::parse(&presigned.url).map_err(|_| StorageError::Unexpected)?;
        url.query_pairs_mut()
            .append_pair("uploadId", upload_id)
            .append_pair("partNumber", &part_number.to_string());
        presigned.url = url.to_string();
        Ok(presigned)
    }

    async fn complete_multipart_upload(
        &self,
        object_key: &str,
        upload_id: &str,
        parts: &[CompletedUploadPart],
    ) -> Result<(), StorageError> {
        self.completed_multipart
            .lock()
            .expect("fake storage mutex poisoned")
            .push((
                object_key.to_string(),
                upload_id.to_string(),
                parts.to_vec(),
            ));
        Ok(())
    }

    async fn abort_multipart_upload(
        &self,
        object_key: &str,
        upload_id: &str,
    ) -> Result<(), StorageError> {
        self.aborted_multipart
            .lock()
            .expect("fake storage mutex poisoned")
            .push((object_key.to_string(), upload_id.to_string()));
        Ok(())
    }

    async fn delete_object(&self, object_key: &str) -> Result<(), StorageError> {
        if self
            .delete_failures
            .lock()
            .expect("fake storage mutex poisoned")
            .contains(object_key)
        {
            return Err(StorageError::Unexpected);
        }

        self.deleted_objects
            .lock()
            .expect("fake storage mutex poisoned")
            .push(object_key.to_string());

        if self
            .objects
            .lock()
            .expect("fake storage mutex poisoned")
            .remove(object_key)
            .is_some()
        {
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    async fn list_objects(&self) -> Result<Vec<StoredObject>, StorageError> {
        let mut objects = self
            .objects
            .lock()
            .expect("fake storage mutex poisoned")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        objects.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(objects)
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
