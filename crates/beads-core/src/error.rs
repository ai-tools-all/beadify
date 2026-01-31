//! SNAFU-based error handling with context and suggestions
//!
//! This module provides rich error types with:
//! - Contextual information (what, where, why)
//! - Actionable suggestions (how to fix)
//! - "Did you mean" fuzzy matching for user input errors

use snafu::prelude::*;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for beads operations
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Repository not found in current directory or parents
    #[snafu(display(
        "beads repository not found\n\n\
         Searched in:\n{searched_paths}\n\n\
         Initialize a repository:\n  \
         beads init --prefix <prefix>\n\n\
         Example:\n  \
         beads init --prefix bd"
    ))]
    RepoNotFound { searched_paths: String },

    /// Repository already initialized
    #[snafu(display(
        "beads repository already exists at: {}\n\n\
         Cannot initialize over an existing repository.\n\n\
         To create a new repository:\n  \
         1. Delete {}/.beads/\n  \
         2. Run: beads init --prefix <prefix>",
        path.display(), path.display()
    ))]
    RepoAlreadyExists { path: PathBuf },

    /// I/O error with context
    #[snafu(display("failed to {action}: {source}"))]
    Io {
        action: String,
        source: std::io::Error,
    },

    /// Database error with context
    #[snafu(display("database error during {operation}: {source}"))]
    Database {
        operation: String,
        source: rusqlite::Error,
    },

    /// JSON parsing error with context
    #[snafu(display(
        "invalid JSON in {context}: {source}\n\n\
         Expected format:\n{expected_format}\n\n\
         Example:\n{example}"
    ))]
    InvalidJson {
        context: String,
        expected_format: String,
        example: String,
        source: serde_json::Error,
    },

    /// Invalid enum value with "did you mean" suggestion
    #[snafu(display(
        "invalid value '{provided}' for {field}\n\n\
         {suggestion}\
         Valid values: {valid_values}"
    ))]
    InvalidEnumValue {
        field: String,
        provided: String,
        suggestion: String, // Either "Did you mean 'X'?\n\n" or empty
        valid_values: String,
    },

    /// Update command called with no fields specified
    #[snafu(display(
        "no updates specified for {entity_id}\n\n\
         Common updates:\n{common_examples}\n\n\
         All available options:\n{all_fields}\n\n\
         Example:\n  beads issue update {entity_id} --status closed"
    ))]
    EmptyUpdate {
        entity_id: String,
        common_examples: String,
        all_fields: String,
    },

    /// Issue not found
    #[snafu(display(
        "issue not found: {issue_id}\n\n\
         List all issues:\n  \
         beads issue list\n\n\
         Search issues:\n  \
         beads search <query>"
    ))]
    IssueNotFound { issue_id: String },

    /// Invalid issue ID format
    #[snafu(display(
        "invalid issue ID format: '{provided}'\n\n\
         Expected format: {prefix}-<number>\n\n\
         Examples:\n  \
         bd-001\n  \
         bd-042\n  \
         {prefix}-123"
    ))]
    InvalidIssueId { provided: String, prefix: String },

    /// Circular dependency detected
    #[snafu(display(
        "circular dependency detected\n\n\
         Cannot add dependency: {from} → {to}\n\
         This would create a cycle: {cycle_path}\n\n\
         Issue dependencies must form a directed acyclic graph (DAG)."
    ))]
    CircularDependency {
        from: String,
        to: String,
        cycle_path: String,
    },

    /// Missing required field
    #[snafu(display(
        "missing required field: {field}\n\n\
         This field cannot be empty.\n\n\
         Example:\n  \
         beads issue create --title \"Fix login bug\" {example_usage}"
    ))]
    MissingRequiredField {
        field: String,
        example_usage: String,
    },

    /// Blob not found in content store
    #[snafu(display(
        "blob not found: {hash}\n\n\
         The content hash '{hash}' does not exist in .beads/blobs/\n\n\
         This may indicate:\n  \
         - Corrupted repository\n  \
         - Missing blob file\n  \
         - Invalid hash reference\n\n\
         Try:\n  \
         beads sync  # Re-sync from event log"
    ))]
    BlobNotFound { hash: String },

    /// Invalid hash format
    #[snafu(display(
        "invalid hash format: {hash}\n\n\
         Expected: 64-character hexadecimal SHA-256 hash\n\n\
         Example:\n  \
         a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890"
    ))]
    InvalidHash { hash: String },

    /// File system permission error
    #[snafu(display(
        "permission denied: {action}\n\n\
         Path: {}\n\
         Error: {source}\n\n\
         Try:\n  \
         chmod +rw {}\n  \
         # Or run with appropriate permissions",
        path.display(), path.display()
    ))]
    PermissionDenied {
        action: String,
        path: PathBuf,
        source: std::io::Error,
    },

    /// Disk full or quota exceeded
    #[snafu(display(
        "disk full: {action}\n\n\
         Path: {}\n\
         Error: {source}\n\n\
         Free up disk space and try again.",
        path.display()
    ))]
    DiskFull {
        action: String,
        path: PathBuf,
        source: std::io::Error,
    },

    /// Other error with message
    #[snafu(display("{message}"))]
    Other { message: String },
}

impl Error {
    /// Create InvalidEnumValue with fuzzy matching suggestion
    pub fn invalid_enum_with_suggestion(
        field: impl Into<String>,
        provided: impl Into<String>,
        valid_options: &[&str],
    ) -> Self {
        let provided = provided.into();
        let field = field.into();

        // Try fuzzy match with 0.75 threshold
        let suggestion =
            crate::utils::fuzzy::find_best_match(&provided.to_lowercase(), valid_options, 0.75)
                .map(|matched| format!("Did you mean '{}'?\n\n", matched))
                .unwrap_or_default();

        let valid_values = valid_options.join(", ");

        Error::InvalidEnumValue {
            field,
            provided,
            suggestion,
            valid_values,
        }
    }

    /// Create EmptyUpdate error for issue updates
    pub fn empty_issue_update(entity_id: impl Into<String>) -> Self {
        let entity_id = entity_id.into();

        let common_examples = [
            "  --status <STATUS>      Change issue status",
            "  --priority <PRIORITY>  Change priority level",
        ]
        .join("\n");

        let all_fields = [
            "  --title <TEXT>",
            "  --description <TEXT>",
            "  --kind <KIND>",
            "  --priority <PRIORITY>",
            "  --status <STATUS>",
            "  --add-label <LABEL>",
            "  --remove-label <LABEL>",
        ]
        .join("\n");

        Error::EmptyUpdate {
            entity_id,
            common_examples,
            all_fields,
        }
    }

    /// Create InvalidIssueId with repository prefix
    pub fn invalid_issue_id(provided: impl Into<String>, prefix: impl Into<String>) -> Self {
        Error::InvalidIssueId {
            provided: provided.into(),
            prefix: prefix.into(),
        }
    }

    /// Create CircularDependency with cycle path
    pub fn circular_dependency(
        from: impl Into<String>,
        to: impl Into<String>,
        cycle: &[String],
    ) -> Self {
        let cycle_path = cycle.join(" → ");

        Error::CircularDependency {
            from: from.into(),
            to: to.into(),
            cycle_path,
        }
    }

    /// Create MissingRequiredField with example
    pub fn missing_field(field: impl Into<String>, example_usage: impl Into<String>) -> Self {
        Error::MissingRequiredField {
            field: field.into(),
            example_usage: example_usage.into(),
        }
    }

    /// Create appropriate IO error based on error kind
    pub fn from_io_error(source: std::io::Error, action: impl Into<String>, path: PathBuf) -> Self {
        let action = action.into();

        match source.kind() {
            std::io::ErrorKind::PermissionDenied => Error::PermissionDenied {
                action,
                path,
                source,
            },
            std::io::ErrorKind::OutOfMemory | std::io::ErrorKind::WriteZero => {
                // WriteZero often indicates disk full
                Error::DiskFull {
                    action,
                    path,
                    source,
                }
            }
            _ => Error::Io { action, source },
        }
    }
}

// Implement From traits for automatic error conversion
impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Error::Io {
            action: "I/O operation".to_string(),
            source,
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(source: rusqlite::Error) -> Self {
        Error::Database {
            operation: "database operation".to_string(),
            source,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Error::InvalidJson {
            context: "JSON parsing".to_string(),
            expected_format: "valid JSON".to_string(),
            example: "{}".to_string(),
            source,
        }
    }
}

impl From<ulid::DecodeError> for Error {
    fn from(source: ulid::DecodeError) -> Self {
        Error::Io {
            action: format!("decode ULID: {}", source),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_not_found_message() {
        let err = Error::RepoNotFound {
            searched_paths: "  /home/user/project\n  /home/user\n  /home".to_string(),
        };
        let msg = err.to_string();

        assert!(msg.contains("beads repository not found"));
        assert!(msg.contains("Searched in:"));
        assert!(msg.contains("/home/user/project"));
        assert!(msg.contains("beads init"));
    }

    #[test]
    fn test_repo_already_exists_message() {
        let err = Error::RepoAlreadyExists {
            path: PathBuf::from("/home/user/project"),
        };
        let msg = err.to_string();

        assert!(msg.contains("already exists"));
        assert!(msg.contains("/home/user/project"));
        assert!(msg.contains("Delete"));
    }

    #[test]
    fn test_io_error_context() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file.txt");
        let err = Error::Io {
            action: "read configuration file".to_string(),
            source: io_err,
        };
        let msg = err.to_string();

        assert!(msg.contains("failed to read configuration file"));
    }

    #[test]
    fn test_invalid_enum_with_close_match() {
        let err = Error::invalid_enum_with_suggestion(
            "priority",
            "hgh",
            &["low", "medium", "high", "urgent"],
        );
        let msg = err.to_string();

        assert!(msg.contains("invalid value 'hgh'"));
        assert!(msg.contains("Did you mean 'high'?"));
        assert!(msg.contains("Valid values: low, medium, high, urgent"));
    }

    #[test]
    fn test_invalid_enum_no_match() {
        let err = Error::invalid_enum_with_suggestion(
            "priority",
            "xyz",
            &["low", "medium", "high", "urgent"],
        );
        let msg = err.to_string();

        assert!(msg.contains("invalid value 'xyz'"));
        assert!(!msg.contains("Did you mean"));
        assert!(msg.contains("Valid values:"));
    }

    #[test]
    fn test_empty_update_message() {
        let err = Error::empty_issue_update("bd-042");
        let msg = err.to_string();

        assert!(msg.contains("no updates specified for bd-042"));
        assert!(msg.contains("Common updates:"));
        assert!(msg.contains("--status"));
        assert!(msg.contains("--priority"));
        assert!(msg.contains("All available options:"));
        assert!(msg.contains("Example:"));
    }

    #[test]
    fn test_issue_not_found() {
        let err = Error::IssueNotFound {
            issue_id: "bd-999".to_string(),
        };
        let msg = err.to_string();

        assert!(msg.contains("issue not found: bd-999"));
        assert!(msg.contains("beads issue list"));
        assert!(msg.contains("beads search"));
    }

    #[test]
    fn test_invalid_issue_id() {
        let err = Error::invalid_issue_id("xyz-123", "bd");
        let msg = err.to_string();

        assert!(msg.contains("invalid issue ID format: 'xyz-123'"));
        assert!(msg.contains("Expected format: bd-<number>"));
        assert!(msg.contains("bd-001"));
    }

    #[test]
    fn test_circular_dependency() {
        let cycle = vec![
            "bd-001".to_string(),
            "bd-002".to_string(),
            "bd-003".to_string(),
            "bd-001".to_string(),
        ];
        let err = Error::circular_dependency("bd-003", "bd-001", &cycle);
        let msg = err.to_string();

        assert!(msg.contains("circular dependency detected"));
        assert!(msg.contains("bd-001 → bd-002 → bd-003 → bd-001"));
        assert!(msg.contains("directed acyclic graph"));
    }

    #[test]
    fn test_missing_required_field() {
        let err = Error::missing_field("title", "--kind bug");
        let msg = err.to_string();

        assert!(msg.contains("missing required field: title"));
        assert!(msg.contains("cannot be empty"));
        assert!(msg.contains("--kind bug"));
    }

    #[test]
    fn test_blob_not_found() {
        let err = Error::BlobNotFound {
            hash: "a1b2c3d4".to_string(),
        };
        let msg = err.to_string();

        assert!(msg.contains("blob not found: a1b2c3d4"));
        assert!(msg.contains(".beads/blobs/"));
        assert!(msg.contains("beads sync"));
    }

    #[test]
    fn test_invalid_hash() {
        let err = Error::InvalidHash {
            hash: "invalid".to_string(),
        };
        let msg = err.to_string();

        assert!(msg.contains("invalid hash format"));
        assert!(msg.contains("64-character hexadecimal"));
    }

    #[test]
    fn test_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = Error::PermissionDenied {
            action: "write file".to_string(),
            path: PathBuf::from("/protected/file.txt"),
            source: io_err,
        };
        let msg = err.to_string();

        assert!(msg.contains("permission denied: write file"));
        assert!(msg.contains("/protected/file.txt"));
        assert!(msg.contains("chmod"));
    }

    #[test]
    fn test_from_io_error_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = Error::from_io_error(io_err, "read config", PathBuf::from("/etc/beads/config"));

        match err {
            Error::PermissionDenied { .. } => (),
            _ => panic!("Expected PermissionDenied variant"),
        }
    }
}
