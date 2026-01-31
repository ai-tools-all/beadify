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
}
