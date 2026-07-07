use crate::domain::error::DomainError;

const FILENAME_MAX_CHARS: usize = 255;
const CONTENT_TYPE_MAX_CHARS: usize = 255;
const SEARCH_QUERY_MAX_CHARS: usize = 100;

pub fn validate_filename(filename: &str) -> Result<String, DomainError> {
    validate_item_name(filename)
}

pub fn validate_folder_name(name: &str) -> Result<String, DomainError> {
    validate_item_name(name)
}

fn validate_item_name(filename: &str) -> Result<String, DomainError> {
    let filename = filename.trim();
    let valid = !filename.is_empty()
        && filename.chars().count() <= FILENAME_MAX_CHARS
        && !filename.contains('/')
        && !filename.contains('\\')
        && filename != "."
        && filename != "..";
    if valid {
        Ok(filename.to_string())
    } else {
        Err(DomainError::Validation("Enter a valid filename"))
    }
}

pub fn validate_content_type(content_type: &str) -> Result<String, DomainError> {
    let content_type = content_type.trim().to_ascii_lowercase();
    let valid = !content_type.is_empty()
        && content_type.len() <= CONTENT_TYPE_MAX_CHARS
        && content_type.contains('/')
        && !content_type.contains(char::is_whitespace);
    if valid {
        Ok(content_type)
    } else {
        Err(DomainError::Validation("Enter a valid content type"))
    }
}

pub fn validate_size(size_bytes: i64, max_file_size_bytes: i64) -> Result<(), DomainError> {
    if size_bytes <= 0 {
        Err(DomainError::Validation(
            "File size must be greater than zero",
        ))
    } else if size_bytes > max_file_size_bytes {
        Err(DomainError::FileTooLarge)
    } else {
        Ok(())
    }
}

pub fn validate_checksum(checksum: Option<&str>) -> Result<Option<String>, DomainError> {
    match checksum {
        None => Ok(None),
        Some(checksum)
            if checksum.len() == 64
                && checksum
                    .chars()
                    .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()) =>
        {
            Ok(Some(checksum.to_string()))
        }
        Some(_) => Err(DomainError::Validation("Enter a valid SHA-256 checksum")),
    }
}

pub fn validate_search_query(query: &str) -> Result<String, DomainError> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > SEARCH_QUERY_MAX_CHARS {
        Err(DomainError::Validation("Enter a search query"))
    } else {
        Ok(query.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_validation_rejects_empty_paths_and_parent_refs() {
        assert!(validate_filename("report.pdf").is_ok());
        for filename in ["", "   ", "../x", "a/b", "a\\b", ".", ".."] {
            assert!(
                validate_filename(filename).is_err(),
                "should reject {filename:?}"
            );
        }
    }

    #[test]
    fn checksum_validation_requires_lowercase_sha256_hex() {
        assert!(validate_checksum(Some(&"a".repeat(64))).is_ok());
        assert!(validate_checksum(None).unwrap().is_none());
        assert!(validate_checksum(Some(&"A".repeat(64))).is_err());
        assert!(validate_checksum(Some("abc")).is_err());
    }

    #[test]
    fn size_validation_enforces_positive_and_maximum() {
        assert!(validate_size(1, 10).is_ok());
        assert!(matches!(
            validate_size(0, 10),
            Err(DomainError::Validation(_))
        ));
        assert!(matches!(
            validate_size(11, 10),
            Err(DomainError::FileTooLarge)
        ));
    }

    #[test]
    fn search_query_validation_trims_and_limits_length() {
        assert_eq!(validate_search_query(" report ").unwrap(), "report");
        assert!(validate_search_query("").is_err());
        assert!(validate_search_query("   ").is_err());
        assert!(validate_search_query(&"a".repeat(101)).is_err());
    }
}
