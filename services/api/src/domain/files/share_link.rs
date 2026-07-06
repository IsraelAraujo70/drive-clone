use argon2::password_hash::rand_core::{OsRng, RngCore};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generate an opaque 32-byte random token, base64url (no padding) encoded.
/// The plaintext token is only ever returned once, at creation time.
pub fn generate_share_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Hash a share token with SHA-256. Only the hash is persisted, so a database
/// dump never leaks working links.
pub fn hash_share_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Owner-facing view of a share link. Never carries the plaintext token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareLink {
    pub id: Uuid,
    pub file_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Metadata resolved from a valid public share link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicShareTarget {
    pub filename: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub object_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_hash_deterministically() {
        let first = generate_share_token();
        let second = generate_share_token();
        assert_ne!(first, second);
        assert_eq!(first.len(), 43);
        // Same token hashes to the same 32-byte digest every time.
        assert_eq!(hash_share_token(&first), hash_share_token(&first));
        assert_eq!(hash_share_token(&first).len(), 32);
        // Different tokens hash to different digests.
        assert_ne!(hash_share_token(&first), hash_share_token(&second));
    }
}
