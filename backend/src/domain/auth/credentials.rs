use crate::domain::error::DomainError;

pub fn validate_email(email: &str) -> Result<(), DomainError> {
    let valid = email.len() <= 254
        && !email.contains(char::is_whitespace)
        && match email.split_once('@') {
            Some((local, domain)) => {
                !local.is_empty()
                    && !domain.contains('@')
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.')
            }
            None => false,
        };

    if valid {
        Ok(())
    } else {
        Err(DomainError::Validation("Enter a valid email address"))
    }
}

pub fn validate_password(password: &str) -> Result<(), DomainError> {
    if password.chars().count() >= 8 && password.len() <= 128 {
        Ok(())
    } else {
        Err(DomainError::Validation(
            "Password must be between 8 and 128 characters",
        ))
    }
}

pub fn validate_display_name(display_name: &str) -> Result<(), DomainError> {
    if !display_name.is_empty() && display_name.chars().count() <= 100 {
        Ok(())
    } else {
        Err(DomainError::Validation(
            "Name must be between 1 and 100 characters",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_validation_accepts_normal_addresses() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("first.last@sub.domain.dev").is_ok());
    }

    #[test]
    fn email_validation_rejects_malformed_addresses() {
        for email in [
            "",
            "no-at-sign",
            "@example.com",
            "user@",
            "user@nodot",
            "user@.com",
            "user@domain.",
            "a b@example.com",
        ] {
            assert!(validate_email(email).is_err(), "should reject {email:?}");
        }
    }

    #[test]
    fn password_validation_enforces_length() {
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password("1234567").is_err());
        assert!(validate_password(&"x".repeat(129)).is_err());
    }
}
