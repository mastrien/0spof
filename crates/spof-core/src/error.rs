use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpofError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Outbox error: {0}")]
    Outbox(String),
}

pub type Result<T> = std::result::Result<T, SpofError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_formatting() {
        let err = SpofError::Network("timeout".to_string());
        assert_eq!(err.to_string(), "Network error: timeout");

        let auth_err = SpofError::Authentication("invalid token".to_string());
        assert_eq!(auth_err.to_string(), "Authentication error: invalid token");

        let val_err = SpofError::Validation("empty payload".to_string());
        assert_eq!(val_err.to_string(), "Validation error: empty payload");

        let outbox_err = SpofError::Outbox("item missing".to_string());
        assert_eq!(outbox_err.to_string(), "Outbox error: item missing");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let spof_err: SpofError = io_err.into();
        assert!(spof_err.to_string().contains("file not found"));
    }
}
