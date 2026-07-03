use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    Pending,
    Complete,
}

impl FileState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Complete => "complete",
        }
    }
}

impl From<String> for FileState {
    fn from(value: String) -> Self {
        match value.as_str() {
            "complete" => Self::Complete,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DriveFile {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub object_key: String,
    pub state: FileState,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct PendingFile {
    pub id: Uuid,
    pub size_bytes: i64,
    pub object_key: String,
    pub state: FileState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct FileShare {
    pub file_id: Uuid,
    pub grantee: FileUser,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SharedFile {
    pub file: DriveFile,
    pub owner: FileUser,
}
