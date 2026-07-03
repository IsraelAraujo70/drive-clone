#[derive(Debug, Clone)]
pub struct UploadRequest {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
}
