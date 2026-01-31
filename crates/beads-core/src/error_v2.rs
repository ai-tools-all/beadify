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
    RepoNotFound {
        searched_paths: String,
    },

    /// Repository already initialized
    #[snafu(display(
        "beads repository already exists at: {}\n\n\
         Cannot initialize over an existing repository.\n\n\
         To create a new repository:\n  \
         1. Delete {}/.beads/\n  \
         2. Run: beads init --prefix <prefix>",
        path.display(), path.display()
    ))]
    RepoAlreadyExists {
        path: PathBuf,
    },

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
        let suggestion = crate::utils::fuzzy::find_best_match(
            &provided.to_lowercase(),
            valid_options,
            0.75,
        )
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
}
