use std::sync::OnceLock;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

use crate::application::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::Internal)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

pub fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| hash_password("not-a-real-password").expect("hashing cannot fail"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hashing_round_trips() {
        let hash = hash_password("hunter2hunter2").unwrap();
        assert!(verify_password("hunter2hunter2", &hash));
        assert!(!verify_password("wrong-password", &hash));
        assert!(!verify_password("hunter2hunter2", "not-a-phc-string"));
    }
}
