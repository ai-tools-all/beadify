use std::fmt::{self, Display};
use std::error::Error;

#[derive(Debug)]
pub enum BeadsError {
    // === System errors ===
    Io { source: std::io::Error },
    Db { source: rusqlite::Error },
    Serde { source: serde_json::Error },
    UlidDecode { source: ulid::DecodeError },

    // === Repository errors ===
    AlreadyInitialized,
    RepoNotFound,
    MissingConfig { key: &'static str },

    // === Resource errors ===
    BlobNotFound { hash: String },
    InvalidHash { hash: String },

    // === Structured CLI errors ===
    EmptyUpdate {
        entity_id: String,
        fields: String,
    },

    InvalidJsonData {
        source: serde_json::Error,
        context: &'static str,
        fields: String,
    },

    MissingRequiredField { field: &'static str },
    InvalidDocFormat { provided: String },

    // === Fallback ===
    Custom { message: String },
}

impl Error for BeadsError {}

impl BeadsError {
    /// Create EmptyUpdate error for issue update
    pub fn empty_update(entity_id: impl Into<String>) -> Self {
        let entity_id = entity_id.into();
        let fields = vec![
            "  --title",
            "  --description",
            "  --kind",
            "  --priority",
            "  --status",
            "  --add-label",
            "  --remove-label",
            "  --data",
        ]
        .join("\n");

        Self::EmptyUpdate {
            entity_id,
            fields,
        }
    }

    /// Create InvalidJsonData error for create command
    pub fn invalid_json_for_create(source: serde_json::Error) -> Self {
        let fields = vec!["  \"description\": <value>,", "  \"priority\": <value>,", "  \"kind\": <value>,"].join("\n");

        Self::InvalidJsonData {
            source,
            context: "create",
            fields,
        }
    }

    /// Create InvalidJsonData error for update command
    pub fn invalid_json_for_update(source: serde_json::Error) -> Self {
        let fields = vec![
            "  \"description\": <value>,",
            "  \"priority\": <value>,",
            "  \"status\": <value>,",
            "  \"kind\": <value>,",
        ]
        .join("\n");

        Self::InvalidJsonData {
            source,
            context: "update",
            fields,
        }
    }

    /// Create MissingRequiredField error
    pub fn missing_field(field: &'static str) -> Self {
        Self::MissingRequiredField { field }
    }

    /// Create InvalidDocFormat error
    pub fn invalid_doc_format(provided: impl Into<String>) -> Self {
        Self::InvalidDocFormat {
            provided: provided.into(),
        }
    }

    /// Create Custom error
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Custom {
            message: message.into(),
        }
    }

    /// Create MissingConfig error
    pub fn missing_config(key: &'static str) -> Self {
        Self::MissingConfig { key }
    }
}

// Implement From traits for system errors
impl From<std::io::Error> for BeadsError {
    fn from(source: std::io::Error) -> Self {
        Self::Io { source }
    }
}

impl From<rusqlite::Error> for BeadsError {
    fn from(source: rusqlite::Error) -> Self {
        Self::Db { source }
    }
}

impl From<serde_json::Error> for BeadsError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serde { source }
    }
}

impl From<ulid::DecodeError> for BeadsError {
    fn from(source: ulid::DecodeError) -> Self {
        Self::UlidDecode { source }
    }
}

impl Display for BeadsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { source } => write!(f, "io error: {}", source),
            Self::Db { source } => write!(f, "database error: {}", source),
            Self::Serde { source } => write!(f, "serialization error: {}", source),
            Self::UlidDecode { source } => write!(f, "ulid decode error: {}", source),
            Self::AlreadyInitialized => write!(f, "beads repository already initialized"),
            Self::RepoNotFound => {
                write!(f, "beads repository not found\n\nRun 'beads init --prefix <prefix>' to create one")
            }
            Self::MissingConfig { key } => write!(f, "missing repository configuration: {}", key),
            Self::BlobNotFound { hash } => write!(f, "blob not found: {}", hash),
            Self::InvalidHash { hash } => write!(f, "invalid hash: {}", hash),
            Self::EmptyUpdate {
                entity_id,
                fields,
            } => {
                write!(
                    f,
                    "update requires at least one field\n\nNo updates specified for {}.\n\nAvailable options:\n{}\n\nExample: beads issue update {} --status closed",
                    entity_id, fields, entity_id
                )
            }
            Self::InvalidJsonData {
                source,
                context,
                fields,
            } => {
                write!(
                    f,
                    "invalid JSON data: {}\n\nExpected format for {}:\n{{\n{}}}",
                    source, context, fields
                )
            }
            Self::MissingRequiredField { field } => {
                write!(f, "{} is required and cannot be empty", field)
            }
            Self::InvalidDocFormat { provided } => {
                write!(
                    f,
                    "Invalid doc format '{}'. Expected 'name:path'\n\nExample: --doc readme:./README.md",
                    provided
                )
            }
            Self::Custom { message } => write!(f, "{}", message),
        }
    }
}

pub type Result<T> = std::result::Result<T, BeadsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_update_formatting() {
        let err = BeadsError::empty_update("bd-015");
        let msg = err.to_string();

        assert!(msg.contains("No updates specified"));
        assert!(msg.contains("bd-015"));
        assert!(msg.contains("--title"));
        assert!(msg.contains("--status"));
        assert!(msg.contains("Example:"));
    }

    #[test]
    fn test_invalid_json_for_create() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err = BeadsError::invalid_json_for_create(json_err);
        let msg = err.to_string();

        assert!(msg.contains("invalid JSON"));
        assert!(msg.contains("create"));
        assert!(msg.contains("description"));
        assert!(msg.contains("priority"));
    }

    #[test]
    fn test_missing_field() {
        let err = BeadsError::missing_field("title");
        let msg = err.to_string();

        assert!(msg.contains("title"));
        assert!(msg.contains("required"));
        assert!(msg.contains("cannot be empty"));
    }

    #[test]
    fn test_repo_not_found_includes_hint() {
        let err = BeadsError::RepoNotFound;
        let msg = err.to_string();

        assert!(msg.contains("beads repository not found"));
        assert!(msg.contains("beads init"));
        assert!(msg.contains("--prefix"));
    }
}
