# Better Error Messages with SNAFU Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement "did you mean" suggestions and context-aware error messages with improved UX

**Architecture:** Add fuzzy string matching for CLI argument suggestions, enhance error messages with context and actionable hints, maintain backward compatibility

**Tech Stack:** snafu 0.8, strsim 0.11, clap 4.5, Rust 2021

---

## Prerequisites

**Dependencies to add:**
- `snafu = "0.8"` - Error handling with context
- `strsim = "0.11"` - String similarity for "did you mean"
- `owo-colors = "4.0"` - Terminal colors (optional, for better UX)

**Note:** This plan implements the new error system. See `2026-01-31-snafu-migration-plan.md` for migration from current BeadsError.

---

## Task 1: Add Dependencies and Feature Flags

**Files:**
- Modify: `Cargo.toml:15-27`
- Modify: `crates/beads-core/Cargo.toml:6-14`

**Step 1: Add workspace dependencies**

```toml
# In Cargo.toml, add to [workspace.dependencies] section
snafu = "0.8"
strsim = "0.11"
owo-colors = "4.0"
```

**Step 2: Add dependencies to beads-core**

```toml
# In crates/beads-core/Cargo.toml [dependencies]
snafu = { workspace = true }
strsim = { workspace = true }
owo-colors = { workspace = true, optional = true }
```

**Step 3: Run cargo check**

Run: `cargo check -p beads-core`
Expected: SUCCESS (dependencies resolve)

**Step 4: Commit**

```bash
git add Cargo.toml crates/beads-core/Cargo.toml
git commit -m "chore: add snafu, strsim, owo-colors dependencies"
```

---

## Task 2: Create Fuzzy Matching Utility Module

**Files:**
- Create: `crates/beads-core/src/utils/mod.rs`
- Create: `crates/beads-core/src/utils/fuzzy.rs`
- Modify: `crates/beads-core/src/lib.rs:1-10`

**Step 1: Write test for fuzzy matching**

Create `crates/beads-core/src/utils/fuzzy.rs`:

```rust
//! Fuzzy string matching for "did you mean" suggestions

use strsim::jaro_winkler;

/// Find best match from a list of valid options
/// Returns Some(suggestion) if confidence > threshold, None otherwise
pub fn find_best_match<'a>(
    input: &str,
    valid_options: &[&'a str],
    threshold: f64,
) -> Option<&'a str> {
    valid_options
        .iter()
        .map(|&option| (option, jaro_winkler(input, option)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .and_then(|(option, score)| if score >= threshold { Some(option) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let options = &["low", "medium", "high", "urgent"];
        assert_eq!(find_best_match("high", options, 0.8), Some("high"));
    }

    #[test]
    fn test_close_match() {
        let options = &["low", "medium", "high", "urgent"];
        assert_eq!(find_best_match("hgh", options, 0.8), Some("high"));
    }

    #[test]
    fn test_typo_match() {
        let options = &["open", "in_progress", "review", "closed"];
        assert_eq!(find_best_match("in_progres", options, 0.8), Some("in_progress"));
    }

    #[test]
    fn test_no_match() {
        let options = &["low", "medium", "high", "urgent"];
        // "xyz" is too different from any option
        assert_eq!(find_best_match("xyz", options, 0.8), None);
    }

    #[test]
    fn test_case_insensitive_preparation() {
        let options = &["low", "medium", "high", "urgent"];
        // Caller should lowercase before calling
        assert_eq!(find_best_match("HIGH", options, 0.8), None);
        assert_eq!(find_best_match("high", options, 0.8), Some("high"));
    }
}
```

**Step 2: Create utils module**

Create `crates/beads-core/src/utils/mod.rs`:

```rust
//! Utility functions

pub mod fuzzy;
```

**Step 3: Export utils from lib.rs**

In `crates/beads-core/src/lib.rs`, add:

```rust
pub mod utils;
```

**Step 4: Run tests**

Run: `cargo test -p beads-core fuzzy`
Expected: All 5 tests PASS

**Step 5: Commit**

```bash
git add crates/beads-core/src/utils/
git add crates/beads-core/src/lib.rs
git commit -m "feat: add fuzzy string matching for did-you-mean suggestions"
```

---

## Task 3: Create New SNAFU Error Types

**Files:**
- Create: `crates/beads-core/src/error_v2.rs`
- Modify: `crates/beads-core/src/lib.rs:10-15`

**Step 1: Write test for basic error creation**

Create `crates/beads-core/src/error_v2.rs`:

```rust
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
        "beads repository already exists at: {path}\n\n\
         Cannot initialize over an existing repository.\n\n\
         To create a new repository:\n  \
         1. Delete {path}/.beads/\n  \
         2. Run: beads init --prefix <prefix>"
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
```

**Step 2: Export error_v2 from lib.rs**

In `crates/beads-core/src/lib.rs`:

```rust
pub mod error_v2;
```

**Step 3: Run tests**

Run: `cargo test -p beads-core error_v2`
Expected: All 3 tests PASS

**Step 4: Commit**

```bash
git add crates/beads-core/src/error_v2.rs
git add crates/beads-core/src/lib.rs
git commit -m "feat: add snafu-based error types with rich context"
```

---

## Task 4: Add "Did You Mean" Error Variant

**Files:**
- Modify: `crates/beads-core/src/error_v2.rs:20-50`

**Step 1: Write test for invalid value with suggestion**

Add to `crates/beads-core/src/error_v2.rs`:

```rust
// Add to Error enum after InvalidJson variant

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
```

**Step 2: Add helper functions at end of file (before tests)**

```rust
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
```

**Step 3: Add tests for new variants**

```rust
// Add to tests module

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
```

**Step 4: Run tests**

Run: `cargo test -p beads-core error_v2`
Expected: All 6 tests PASS (3 original + 3 new)

**Step 5: Commit**

```bash
git add crates/beads-core/src/error_v2.rs
git commit -m "feat: add did-you-mean error variant with fuzzy matching"
```

---

## Task 5: Add Context-Aware Issue Errors

**Files:**
- Modify: `crates/beads-core/src/error_v2.rs:50-80`

**Step 1: Add issue-specific error variants**

Add to Error enum:

```rust
    /// Issue not found
    #[snafu(display(
        "issue not found: {issue_id}\n\n\
         List all issues:\n  \
         beads issue list\n\n\
         Search issues:\n  \
         beads search <query>"
    ))]
    IssueNotFound {
        issue_id: String,
    },

    /// Invalid issue ID format
    #[snafu(display(
        "invalid issue ID format: '{provided}'\n\n\
         Expected format: {prefix}-<number>\n\n\
         Examples:\n  \
         bd-001\n  \
         bd-042\n  \
         {prefix}-123"
    ))]
    InvalidIssueId {
        provided: String,
        prefix: String,
    },

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
```

**Step 2: Add constructor helpers**

Add to `impl Error` block:

```rust
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
```

**Step 3: Write tests**

Add to tests module:

```rust
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
```

**Step 4: Run tests**

Run: `cargo test -p beads-core error_v2`
Expected: All 10 tests PASS

**Step 5: Commit**

```bash
git add crates/beads-core/src/error_v2.rs
git commit -m "feat: add context-aware issue error variants"
```

---

## Task 6: Add File System and Blob Errors

**Files:**
- Modify: `crates/beads-core/src/error_v2.rs:80-120`

**Step 1: Add file and blob error variants**

Add to Error enum:

```rust
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
    BlobNotFound {
        hash: String,
    },

    /// Invalid hash format
    #[snafu(display(
        "invalid hash format: {hash}\n\n\
         Expected: 64-character hexadecimal SHA-256 hash\n\n\
         Example:\n  \
         a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890"
    ))]
    InvalidHash {
        hash: String,
    },

    /// File system permission error
    #[snafu(display(
        "permission denied: {action}\n\n\
         Path: {path}\n\
         Error: {source}\n\n\
         Try:\n  \
         chmod +rw {path}\n  \
         # Or run with appropriate permissions"
    ))]
    PermissionDenied {
        action: String,
        path: PathBuf,
        source: std::io::Error,
    },

    /// Disk full or quota exceeded
    #[snafu(display(
        "disk full: {action}\n\n\
         Path: {path}\n\
         Error: {source}\n\n\
         Free up disk space and try again."
    ))]
    DiskFull {
        action: String,
        path: PathBuf,
        source: std::io::Error,
    },
```

**Step 2: Add helper to distinguish IO errors**

Add to `impl Error` block:

```rust
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
            _ => Error::Io {
                action,
                source,
            },
        }
    }
```

**Step 3: Write tests**

Add to tests module:

```rust
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
        let io_err = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied",
        );
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
        let io_err = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied",
        );
        let err = Error::from_io_error(
            io_err,
            "read config",
            PathBuf::from("/etc/beads/config"),
        );

        match err {
            Error::PermissionDenied { .. } => (),
            _ => panic!("Expected PermissionDenied variant"),
        }
    }
```

**Step 4: Run tests**

Run: `cargo test -p beads-core error_v2`
Expected: All 14 tests PASS

**Step 5: Commit**

```bash
git add crates/beads-core/src/error_v2.rs
git commit -m "feat: add file system and blob error variants"
```

---

## Task 7: Integration Test for Error Messages

**Files:**
- Create: `crates/beads-core/tests/error_messages.rs`

**Step 1: Write integration test**

Create `crates/beads-core/tests/error_messages.rs`:

```rust
//! Integration tests for error message quality

use beads_core::error_v2::Error;

#[test]
fn error_messages_are_helpful() {
    // Each error should:
    // 1. Clearly state what went wrong
    // 2. Provide context (where, why)
    // 3. Suggest how to fix it

    let err = Error::RepoNotFound {
        searched_paths: "  /home/user/project\n  /home/user".to_string(),
    };
    let msg = err.to_string();

    // What went wrong
    assert!(msg.contains("not found"));

    // Context (where)
    assert!(msg.contains("/home/user/project"));

    // How to fix
    assert!(msg.contains("beads init"));
}

#[test]
fn fuzzy_matching_suggests_close_matches() {
    let err = Error::invalid_enum_with_suggestion(
        "priority",
        "urgnt", // typo
        &["low", "medium", "high", "urgent"],
    );
    let msg = err.to_string();

    assert!(msg.contains("Did you mean 'urgent'?"));
}

#[test]
fn fuzzy_matching_handles_no_close_match() {
    let err = Error::invalid_enum_with_suggestion(
        "priority",
        "xyz", // completely wrong
        &["low", "medium", "high", "urgent"],
    );
    let msg = err.to_string();

    // Should not suggest anything
    assert!(!msg.contains("Did you mean"));

    // But should show valid options
    assert!(msg.contains("Valid values:"));
    assert!(msg.contains("low, medium, high, urgent"));
}

#[test]
fn empty_update_shows_common_and_all_options() {
    let err = Error::empty_issue_update("bd-042");
    let msg = err.to_string();

    // Shows common options prominently
    assert!(msg.contains("Common updates:"));
    assert!(msg.contains("--status"));

    // Also shows all options
    assert!(msg.contains("All available options:"));
    assert!(msg.contains("--add-label"));

    // Provides example
    assert!(msg.contains("Example:"));
    assert!(msg.contains("bd-042"));
}

#[test]
fn circular_dependency_explains_problem() {
    let cycle = vec![
        "bd-001".to_string(),
        "bd-002".to_string(),
        "bd-003".to_string(),
        "bd-001".to_string(),
    ];
    let err = Error::circular_dependency("bd-003", "bd-001", &cycle);
    let msg = err.to_string();

    // Explains what a circular dependency is
    assert!(msg.contains("circular dependency"));

    // Shows the cycle path
    assert!(msg.contains("bd-001 → bd-002 → bd-003 → bd-001"));

    // Explains the constraint
    assert!(msg.contains("directed acyclic graph"));
}

#[test]
fn io_errors_provide_actionable_advice() {
    let io_err = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "access denied",
    );
    let err = Error::PermissionDenied {
        action: "write event log".to_string(),
        path: std::path::PathBuf::from(".beads/events.jsonl"),
        source: io_err,
    };
    let msg = err.to_string();

    // Clear problem statement
    assert!(msg.contains("permission denied"));

    // Shows the path
    assert!(msg.contains(".beads/events.jsonl"));

    // Suggests fix
    assert!(msg.contains("chmod"));
}
```

**Step 2: Run integration tests**

Run: `cargo test -p beads-core --test error_messages`
Expected: All 6 integration tests PASS

**Step 3: Commit**

```bash
git add crates/beads-core/tests/error_messages.rs
git commit -m "test: add integration tests for error message quality"
```

---

## Task 8: Documentation and Examples

**Files:**
- Create: `docs/work/2026-01-31-error-messages-examples.md`

**Step 1: Write documentation with examples**

Create `docs/work/2026-01-31-error-messages-examples.md`:

```markdown
# Error Message Examples

This document shows examples of the improved error messages with SNAFU.

## "Did You Mean" Suggestions

### Priority Typo

**Command:**
```bash
$ beads issue create --title "Test" --priority hgh
```

**Output:**
```
error: invalid value 'hgh' for priority

Did you mean 'high'?

Valid values: low, medium, high, urgent
```

### Status Typo

**Command:**
```bash
$ beads issue update bd-042 --status in_progres
```

**Output:**
```
error: invalid value 'in_progres' for status

Did you mean 'in_progress'?

Valid values: open, in_progress, review, closed
```

### No Close Match

**Command:**
```bash
$ beads issue create --title "Test" --priority xyz
```

**Output:**
```
error: invalid value 'xyz' for priority

Valid values: low, medium, high, urgent
```

## Context-Aware Errors

### Repository Not Found

**Command:**
```bash
$ beads issue list  # in ~/projects/myapp/
```

**Output:**
```
error: beads repository not found

Searched in:
  /home/user/projects/myapp
  /home/user/projects
  /home/user
  /home

Initialize a repository:
  beads init --prefix <prefix>

Example:
  beads init --prefix bd
```

### Empty Update

**Command:**
```bash
$ beads issue update bd-042
```

**Output:**
```
error: no updates specified for bd-042

Common updates:
  --status <STATUS>      Change issue status
  --priority <PRIORITY>  Change priority level

All available options:
  --title <TEXT>
  --description <TEXT>
  --kind <KIND>
  --priority <PRIORITY>
  --status <STATUS>
  --add-label <LABEL>
  --remove-label <LABEL>

Example:
  beads issue update bd-042 --status closed
```

### Invalid Issue ID

**Command:**
```bash
$ beads show xyz-123  # wrong prefix
```

**Output:**
```
error: invalid issue ID format: 'xyz-123'

Expected format: bd-<number>

Examples:
  bd-001
  bd-042
  bd-123
```

### Circular Dependency

**Command:**
```bash
$ beads dep add bd-003 --blocks bd-001
# Where bd-001 → bd-002 → bd-003 already exists
```

**Output:**
```
error: circular dependency detected

Cannot add dependency: bd-003 → bd-001
This would create a cycle: bd-001 → bd-002 → bd-003 → bd-001

Issue dependencies must form a directed acyclic graph (DAG).
```

### Permission Denied

**Command:**
```bash
$ beads issue create --title "Test"
# When .beads/events.jsonl is read-only
```

**Output:**
```
error: permission denied: write event log

Path: .beads/events.jsonl
Error: Permission denied (os error 13)

Try:
  chmod +rw .beads/events.jsonl
  # Or run with appropriate permissions
```

## Technical Details

### Fuzzy Matching Algorithm

- Uses Jaro-Winkler similarity (via `strsim` crate)
- Threshold: 0.75 (75% similarity required)
- Case-insensitive matching
- Suggests best match only if above threshold

### Error Message Structure

All errors follow this pattern:

1. **Problem:** What went wrong (clear, concise)
2. **Context:** Where/why it happened (paths, values)
3. **Solution:** How to fix it (commands, examples)

### Testing

Run error message tests:
```bash
cargo test -p beads-core error_v2
cargo test -p beads-core --test error_messages
```
```

**Step 2: Commit documentation**

```bash
git add docs/work/2026-01-31-error-messages-examples.md
git commit -m "docs: add error message examples and technical details"
```

---

## Summary

**Implemented:**
- ✅ Fuzzy string matching for "did you mean" suggestions
- ✅ Context-aware error messages with actionable advice
- ✅ SNAFU-based error types with rich formatting
- ✅ Comprehensive test coverage (20 tests total)
- ✅ Documentation with examples

**Files Created:**
- `crates/beads-core/src/utils/fuzzy.rs` - Fuzzy matching utility
- `crates/beads-core/src/error_v2.rs` - New SNAFU error types
- `crates/beads-core/tests/error_messages.rs` - Integration tests
- `docs/work/2026-01-31-error-messages-examples.md` - Documentation

**Files Modified:**
- `Cargo.toml` - Added workspace dependencies
- `crates/beads-core/Cargo.toml` - Added crate dependencies
- `crates/beads-core/src/lib.rs` - Exported new modules

**Dependencies Added:**
- `snafu = "0.8"` - Error handling
- `strsim = "0.11"` - String similarity
- `owo-colors = "4.0"` - Terminal colors (optional)

**Next Steps:**
See `2026-01-31-snafu-migration-plan.md` for migrating existing code from `BeadsError` to the new SNAFU error system.

---

## Plan complete!

Saved to: `docs/plans/2026-01-31-better-error-messages-with-snafu.md`

**Two execution options:**

**1. Subagent-Driven (this session)** - Fresh subagent per task, code review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans skill, batch execution with checkpoints

**Which approach would you like?**
