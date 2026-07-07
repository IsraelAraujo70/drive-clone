use argon2::password_hash::rand_core::{OsRng, RngCore};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

pub const SESSION_TTL_DAYS: i64 = 30;

pub fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_hash_deterministically() {
        let first = generate_token();
        let second = generate_token();
        assert_ne!(first, second);
        assert_eq!(first.len(), 43);
        assert_eq!(hash_token(&first), hash_token(&first));
        assert_eq!(hash_token(&first).len(), 64);
    }
}
