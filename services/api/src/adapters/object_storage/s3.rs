use std::env;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use url::Url;

use crate::application::ports::StorageError;
use crate::application::ports::object_storage::{
    CompletedUploadPart, ObjectMetadata, ObjectStorage, PresignedUrl,
};

#[derive(Debug, Clone)]
pub struct S3ObjectStorage {
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

impl S3ObjectStorage {
    pub fn from_env() -> Result<Self, StorageError> {
        let endpoint_url =
            Url::parse(&required_env("S3_ENDPOINT_URL")?).map_err(|_| StorageError::Unexpected)?;
        let public_endpoint_url = env::var("S3_PUBLIC_ENDPOINT_URL")
            .ok()
            .map(|value| Url::parse(&value).map_err(|_| StorageError::Unexpected))
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

    fn object_url(&self, base: &Url, object_key: &str) -> Result<Url, StorageError> {
        let mut url = base.clone();
        match self.url_style {
            S3UrlStyle::Path => {
                url.path_segments_mut()
                    .map_err(|_| StorageError::Unexpected)?
                    .clear()
                    .push(&self.bucket)
                    .extend(object_key.split('/'));
            }
            S3UrlStyle::VirtualHost => {
                let host = url.host_str().ok_or(StorageError::Unexpected)?;
                url.set_host(Some(&format!("{}.{}", self.bucket, host)))
                    .map_err(|_| StorageError::Unexpected)?;
                url.path_segments_mut()
                    .map_err(|_| StorageError::Unexpected)?
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
    ) -> Result<PresignedUrl, StorageError> {
        self.presign_with_query(
            method,
            &self.public_endpoint_url,
            object_key,
            ttl_seconds,
            &[],
        )
    }

    fn presign_with_query(
        &self,
        method: &str,
        base_url: &Url,
        object_key: &str,
        ttl_seconds: i64,
        extra_query: &[(&str, String)],
    ) -> Result<PresignedUrl, StorageError> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(ttl_seconds);
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date = now.format("%Y%m%d").to_string();
        let credential_scope = format!("{date}/{}/s3/aws4_request", self.region);
        let credential = format!("{}/{}", self.access_key_id, credential_scope);
        let mut url = self.object_url(base_url, object_key)?;
        let host = url.host_str().ok_or(StorageError::Unexpected)?.to_string();
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
            for (key, value) in extra_query {
                query.append_pair(key, value);
            }
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

    async fn signed_request(
        &self,
        method: &str,
        object_key: &str,
        query: &[(&str, String)],
        body: Option<String>,
        content_type: Option<&str>,
    ) -> Result<reqwest::Response, StorageError> {
        let url = self
            .presign_with_query(method, &self.endpoint_url, object_key, 60, query)?
            .url;
        let client = reqwest::Client::new();
        let method =
            reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| StorageError::Unexpected)?;
        let mut request = client.request(method, url);
        if let Some(content_type) = content_type {
            request = request.header(reqwest::header::CONTENT_TYPE, content_type);
        }
        if let Some(body) = body {
            request = request.body(body);
        }
        request.send().await.map_err(|_| StorageError::Unexpected)
    }
}

#[async_trait]
impl ObjectStorage for S3ObjectStorage {
    async fn presign_put(
        &self,
        object_key: &str,
        _content_type: &str,
        _size_bytes: i64,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError> {
        self.presign("PUT", object_key, ttl_seconds)
    }

    async fn presign_get(
        &self,
        object_key: &str,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError> {
        self.presign("GET", object_key, ttl_seconds)
    }

    async fn head_object(&self, object_key: &str) -> Result<ObjectMetadata, StorageError> {
        let url = self
            .presign_with_query("HEAD", &self.endpoint_url, object_key, 60, &[])?
            .url;
        let response = reqwest::Client::new()
            .head(url)
            .send()
            .await
            .map_err(|_| StorageError::Unexpected)?;
        if !response.status().is_success() {
            return Err(StorageError::Unexpected);
        }
        let content_length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or(StorageError::Unexpected)?;
        Ok(ObjectMetadata { content_length })
    }

    async fn create_multipart_upload(
        &self,
        object_key: &str,
        content_type: &str,
    ) -> Result<String, StorageError> {
        let response = self
            .signed_request(
                "POST",
                object_key,
                &[("uploads", String::new())],
                None,
                Some(content_type),
            )
            .await?;
        if !response.status().is_success() {
            return Err(StorageError::Unexpected);
        }
        let body = response
            .text()
            .await
            .map_err(|_| StorageError::Unexpected)?;
        extract_xml_text(&body, "UploadId").ok_or(StorageError::Unexpected)
    }

    async fn presign_upload_part(
        &self,
        object_key: &str,
        upload_id: &str,
        part_number: i32,
        ttl_seconds: i64,
    ) -> Result<PresignedUrl, StorageError> {
        self.presign_with_query(
            "PUT",
            &self.public_endpoint_url,
            object_key,
            ttl_seconds,
            &[
                ("partNumber", part_number.to_string()),
                ("uploadId", upload_id.to_string()),
            ],
        )
    }

    async fn complete_multipart_upload(
        &self,
        object_key: &str,
        upload_id: &str,
        parts: &[CompletedUploadPart],
    ) -> Result<(), StorageError> {
        let body = complete_multipart_xml(parts);
        let response = self
            .signed_request(
                "POST",
                object_key,
                &[("uploadId", upload_id.to_string())],
                Some(body),
                Some("application/xml"),
            )
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(StorageError::Unexpected)
        }
    }

    async fn abort_multipart_upload(
        &self,
        object_key: &str,
        upload_id: &str,
    ) -> Result<(), StorageError> {
        let response = self
            .signed_request(
                "DELETE",
                object_key,
                &[("uploadId", upload_id.to_string())],
                None,
                None,
            )
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(StorageError::Unexpected)
        }
    }
}

fn extract_xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].to_string())
}

fn complete_multipart_xml(parts: &[CompletedUploadPart]) -> String {
    let mut xml = String::from("<CompleteMultipartUpload>");
    for part in parts {
        xml.push_str("<Part>");
        xml.push_str(&format!("<PartNumber>{}</PartNumber>", part.part_number));
        xml.push_str("<ETag>");
        xml.push_str(&escape_xml(&part.etag));
        xml.push_str("</ETag>");
        xml.push_str("</Part>");
    }
    xml.push_str("</CompleteMultipartUpload>");
    xml
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

impl S3UrlStyle {
    fn from_env() -> Result<Self, StorageError> {
        match env::var("S3_URL_STYLE")
            .unwrap_or_else(|_| "path".to_string())
            .as_str()
        {
            "path" => Ok(Self::Path),
            "virtual-host" => Ok(Self::VirtualHost),
            _ => Err(StorageError::Unexpected),
        }
    }
}

fn required_env(name: &str) -> Result<String, StorageError> {
    env::var(name).map_err(|_| StorageError::Unexpected)
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

fn signing_key(secret: &str, date: &str, region: &str) -> Result<Vec<u8>, StorageError> {
    let date_key = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes())?;
    let region_key = hmac_sha256(&date_key, region.as_bytes())?;
    let service_key = hmac_sha256(&region_key, b"s3")?;
    hmac_sha256(&service_key, b"aws4_request")
}

fn hmac_sha256(key: &[u8], bytes: &[u8]) -> Result<Vec<u8>, StorageError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).map_err(|_| StorageError::Unexpected)?;
    mac.update(bytes);
    Ok(mac.finalize().into_bytes().to_vec())
}

#[cfg(test)]
fn parse_head_response(response: &[u8]) -> Result<ObjectMetadata, StorageError> {
    let text = String::from_utf8_lossy(response);
    let mut lines = text.lines();
    let status = lines.next().ok_or(StorageError::Unexpected)?;
    if !status.contains(" 200 ") {
        return Err(StorageError::Unexpected);
    }

    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                let content_length = value
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| StorageError::Unexpected)?;
                return Ok(ObjectMetadata { content_length });
            }
        }
    }

    Err(StorageError::Unexpected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage(url_style: S3UrlStyle) -> S3ObjectStorage {
        S3ObjectStorage {
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
