use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UploadRequest {
    pub filename: String,
    pub parent_folder_id: Option<Uuid>,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
}
