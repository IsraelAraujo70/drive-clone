use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    Pending,
    Complete,
    Expired,
}

impl FileState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Complete => "complete",
            Self::Expired => "expired",
        }
    }
}

impl From<String> for FileState {
    fn from(value: String) -> Self {
        match value.as_str() {
            "complete" => Self::Complete,
            "expired" => Self::Expired,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DriveFile {
    pub id: Uuid,
    pub filename: String,
    pub parent_folder_id: Option<Uuid>,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub object_key: String,
    pub state: FileState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Folder {
    pub id: Uuid,
    pub name: String,
    pub parent_folder_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct FolderPathEntry {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DriveBrowse {
    pub parent_folder_id: Option<Uuid>,
    pub breadcrumbs: Vec<FolderPathEntry>,
    pub folders: Vec<Folder>,
    pub files: Vec<DriveFile>,
}

#[derive(Debug, Clone)]
pub struct PendingFile {
    pub id: Uuid,
    pub size_bytes: i64,
    pub object_key: String,
    pub state: FileState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingUpload {
    pub file_id: Uuid,
    pub filename: String,
    pub parent_folder_id: Option<Uuid>,
    pub size_bytes: i64,
    pub part_size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub parts_received: i64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadPart {
    pub part_number: i32,
    pub size_bytes: i64,
    pub etag: String,
}

#[derive(Debug, Clone)]
pub struct ResumableUploadSession {
    pub file_id: Uuid,
    pub owner_id: Uuid,
    pub filename: String,
    pub parent_folder_id: Option<Uuid>,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub object_key: String,
    pub multipart_upload_id: String,
    pub part_size_bytes: i64,
    pub state: FileState,
    pub upload_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub parts: Vec<UploadPart>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchAccess {
    Owned,
    Shared,
}

impl SearchAccess {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owned => "owned",
            Self::Shared => "shared",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchFileResult {
    pub file: DriveFile,
    pub access: SearchAccess,
    pub owner: Option<FileUser>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeEntityType {
    File,
    Folder,
}

impl ChangeEntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
        }
    }
}

impl From<String> for ChangeEntityType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "folder" => Self::Folder,
            _ => Self::File,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeOp {
    Upsert,
    Delete,
}

impl ChangeOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Upsert => "upsert",
            Self::Delete => "delete",
        }
    }
}

impl From<String> for ChangeOp {
    fn from(value: String) -> Self {
        match value.as_str() {
            "delete" => Self::Delete,
            _ => Self::Upsert,
        }
    }
}

/// A single entry in the per-owner change feed. `file`/`folder` carry the
/// entity snapshot for `upsert` operations; both are `None` for tombstones
/// (`delete`), where only `entity_id` identifies the removed item.
#[derive(Debug, Clone)]
pub struct ChangeLogEntry {
    pub seq: i64,
    pub entity_type: ChangeEntityType,
    pub entity_id: Uuid,
    pub op: ChangeOp,
    pub occurred_at: DateTime<Utc>,
    pub file: Option<DriveFile>,
    pub folder: Option<Folder>,
}
