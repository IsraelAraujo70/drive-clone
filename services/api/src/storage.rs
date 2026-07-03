use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use url::Url;

use crate::auth::ApiError;

#[derive(Debug, Clone)]
pub struct PresignedUrl {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ObjectMetadata {
    pub content_length: i64,
}

#[async_trait]
pub trait ObjectStorage: Send + Sync {
    async fn presign_put(
        &self,
        object_key: &str,
        content_type: &str,
        size_bytes: i64,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError>;

    async fn presign_get(
        &self,
        object_key: &str,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError>;

    async fn head_object(&self, object_key: &str) -> Result<ObjectMetadata, ApiError>;
}

#[derive(Debug, Clone)]
pub struct S3Storage {
    endpoint_url: Url,
    public_endpoint_url: Url,
    bucket: String,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    url_style: S3UrlStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum S3UrlStyle {
    Path,
    VirtualHost,
}

impl S3Storage {
    pub fn from_env() -> Result<Self, ApiError> {
        let endpoint_url =
            Url::parse(&required_env("S3_ENDPOINT_URL")?).map_err(|_| ApiError::StorageError)?;
        let public_endpoint_url = env::var("S3_PUBLIC_ENDPOINT_URL")
            .ok()
            .map(|value| Url::parse(&value).map_err(|_| ApiError::StorageError))
            .transpose()?
            .unwrap_or_else(|| endpoint_url.clone());

        Ok(Self {
            endpoint_url,
            public_endpoint_url,
            bucket: required_env("S3_BUCKET")?,
            region: required_env("S3_REGION")?,
            access_key_id: required_env("S3_ACCESS_KEY_ID")?,
            secret_access_key: required_env("S3_SECRET_ACCESS_KEY")?,
            url_style: S3UrlStyle::from_env()?,
        })
    }

    fn object_url(&self, base: &Url, object_key: &str) -> Result<Url, ApiError> {
        let mut url = base.clone();
        match self.url_style {
            S3UrlStyle::Path => {
                url.path_segments_mut()
                    .map_err(|_| ApiError::StorageError)?
                    .clear()
                    .push(&self.bucket)
                    .extend(object_key.split('/'));
            }
            S3UrlStyle::VirtualHost => {
                let host = url.host_str().ok_or(ApiError::StorageError)?;
                url.set_host(Some(&format!("{}.{}", self.bucket, host)))
                    .map_err(|_| ApiError::StorageError)?;
                url.path_segments_mut()
                    .map_err(|_| ApiError::StorageError)?
                    .clear()
                    .extend(object_key.split('/'));
            }
        }
        Ok(url)
    }

    fn presign(
        &self,
        method: &str,
        object_key: &str,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError> {
        self.presign_with_base(method, &self.public_endpoint_url, object_key, ttl_seconds)
    }

    fn presign_with_base(
        &self,
        method: &str,
        base_url: &Url,
        object_key: &str,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(ttl_seconds);
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date = now.format("%Y%m%d").to_string();
        let credential_scope = format!("{date}/{}/s3/aws4_request", self.region);
        let credential = format!("{}/{}", self.access_key_id, credential_scope);
        let mut url = self.object_url(base_url, object_key)?;
        let host = url.host_str().ok_or(ApiError::StorageError)?.to_string();
        let host = match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host,
        };

        {
            let mut query = url.query_pairs_mut();
            query.append_pair("X-Amz-Algorithm", "AWS4-HMAC-SHA256");
            query.append_pair("X-Amz-Credential", &credential);
            query.append_pair("X-Amz-Date", &amz_date);
            query.append_pair("X-Amz-Expires", &ttl_seconds.to_string());
            query.append_pair("X-Amz-SignedHeaders", "host");
        }

        let canonical_query = canonical_query(&url);
        let canonical_request = format!(
            "{method}\n{}\n{canonical_query}\nhost:{host}\n\nhost\nUNSIGNED-PAYLOAD",
            canonical_uri(&url)
        );
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );
        let signing_key = signing_key(&self.secret_access_key, &date, &self.region)?;
        let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes())?);
        url.query_pairs_mut()
            .append_pair("X-Amz-Signature", &signature);

        Ok(PresignedUrl {
            url: url.to_string(),
            expires_at,
        })
    }
}

#[async_trait]
impl ObjectStorage for S3Storage {
    async fn presign_put(
        &self,
        object_key: &str,
        _content_type: &str,
        _size_bytes: i64,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError> {
        self.presign("PUT", object_key, ttl_seconds)
    }

    async fn presign_get(
        &self,
        object_key: &str,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError> {
        self.presign("GET", object_key, ttl_seconds)
    }

    async fn head_object(&self, object_key: &str) -> Result<ObjectMetadata, ApiError> {
        let url = self
            .presign_with_base("HEAD", &self.endpoint_url, object_key, 60)?
            .url;
        let response = reqwest::Client::new()
            .head(url)
            .send()
            .await
            .map_err(|_| ApiError::StorageError)?;
        if !response.status().is_success() {
            return Err(ApiError::StorageError);
        }
        let content_length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or(ApiError::StorageError)?;
        Ok(ObjectMetadata { content_length })
    }
}

impl S3UrlStyle {
    fn from_env() -> Result<Self, ApiError> {
        match env::var("S3_URL_STYLE")
            .unwrap_or_else(|_| "path".to_string())
            .as_str()
        {
            "path" => Ok(Self::Path),
            "virtual-host" => Ok(Self::VirtualHost),
            _ => Err(ApiError::StorageError),
        }
    }
}

#[derive(Debug, Default)]
pub struct DisabledStorage;

#[async_trait]
impl ObjectStorage for DisabledStorage {
    async fn presign_put(
        &self,
        _object_key: &str,
        _content_type: &str,
        _size_bytes: i64,
        _ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError> {
        Err(ApiError::StorageError)
    }

    async fn presign_get(
        &self,
        _object_key: &str,
        _ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError> {
        Err(ApiError::StorageError)
    }

    async fn head_object(&self, _object_key: &str) -> Result<ObjectMetadata, ApiError> {
        Err(ApiError::StorageError)
    }
}

#[derive(Debug, Clone, Default)]
pub struct FakeStorage {
    objects: Arc<Mutex<HashMap<String, ObjectMetadata>>>,
}

impl FakeStorage {
    pub fn put_object(&self, object_key: &str, content_length: i64) {
        self.objects
            .lock()
            .expect("fake storage mutex poisoned")
            .insert(object_key.to_string(), ObjectMetadata { content_length });
    }
}

#[async_trait]
impl ObjectStorage for FakeStorage {
    async fn presign_put(
        &self,
        object_key: &str,
        _content_type: &str,
        _size_bytes: i64,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError> {
        presigned_fake_url("PUT", object_key, ttl_seconds)
    }

    async fn presign_get(
        &self,
        object_key: &str,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, ApiError> {
        presigned_fake_url("GET", object_key, ttl_seconds)
    }

    async fn head_object(&self, object_key: &str) -> Result<ObjectMetadata, ApiError> {
        self.objects
            .lock()
            .expect("fake storage mutex poisoned")
            .get(object_key)
            .cloned()
            .ok_or(ApiError::StorageError)
    }
}

fn presigned_fake_url(
    method: &str,
    object_key: &str,
    ttl_seconds: i64,
) -> Result<PresignedUrl, ApiError> {
    let expires_at = Utc::now() + Duration::seconds(ttl_seconds);
    let mut url = Url::parse("http://storage.test/").map_err(|_| ApiError::StorageError)?;
    url.path_segments_mut()
        .map_err(|_| ApiError::StorageError)?
        .extend(object_key.split('/'));
    url.query_pairs_mut().append_pair("method", method);
    Ok(PresignedUrl {
        url: url.to_string(),
        expires_at,
    })
}

fn required_env(name: &str) -> Result<String, ApiError> {
    env::var(name).map_err(|_| ApiError::StorageError)
}

fn canonical_uri(url: &Url) -> String {
    url.path()
        .split('/')
        .map(percent_encode_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn canonical_query(url: &Url) -> String {
    let mut pairs = url
        .query_pairs()
        .map(|(key, value)| (percent_encode_query(&key), percent_encode_query(&value)))
        .collect::<Vec<_>>();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode_path_segment(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn percent_encode_query(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn signing_key(secret: &str, date: &str, region: &str) -> Result<Vec<u8>, ApiError> {
    let date_key = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes())?;
    let region_key = hmac_sha256(&date_key, region.as_bytes())?;
    let service_key = hmac_sha256(&region_key, b"s3")?;
    hmac_sha256(&service_key, b"aws4_request")
}

fn hmac_sha256(key: &[u8], bytes: &[u8]) -> Result<Vec<u8>, ApiError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).map_err(|_| ApiError::StorageError)?;
    mac.update(bytes);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn parse_head_response(response: &[u8]) -> Result<ObjectMetadata, ApiError> {
    let text = String::from_utf8_lossy(response);
    let mut lines = text.lines();
    let status = lines.next().ok_or(ApiError::StorageError)?;
    if !status.contains(" 200 ") {
        return Err(ApiError::StorageError);
    }

    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                let content_length = value
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| ApiError::StorageError)?;
                return Ok(ObjectMetadata { content_length });
            }
        }
    }

    Err(ApiError::StorageError)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage(url_style: S3UrlStyle) -> S3Storage {
        S3Storage {
            endpoint_url: Url::parse("https://storage.example").unwrap(),
            public_endpoint_url: Url::parse("https://public-storage.example").unwrap(),
            bucket: "bucket-name".to_string(),
            region: "auto".to_string(),
            access_key_id: "access".to_string(),
            secret_access_key: "secret".to_string(),
            url_style,
        }
    }

    #[test]
    fn path_style_urls_include_bucket_in_path() {
        let url = storage(S3UrlStyle::Path)
            .object_url(
                &Url::parse("https://storage.example").unwrap(),
                "user-id/object-id",
            )
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://storage.example/bucket-name/user-id/object-id"
        );
    }

    #[test]
    fn virtual_host_urls_include_bucket_in_host() {
        let url = storage(S3UrlStyle::VirtualHost)
            .object_url(
                &Url::parse("https://storage.example").unwrap(),
                "user-id/object-id",
            )
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://bucket-name.storage.example/user-id/object-id"
        );
    }

    #[test]
    fn parse_head_response_reads_content_length() {
        let metadata =
            parse_head_response(b"HTTP/1.1 200 OK\r\ncontent-length: 42\r\n\r\n").unwrap();
        assert_eq!(metadata.content_length, 42);
    }
}
