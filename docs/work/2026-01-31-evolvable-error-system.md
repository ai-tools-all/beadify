# Evolvable Error System Refactoring Plan

## Problem Statement

Current error handling uses scattered `anyhow!()` strings across CLI commands, making the system hard to evolve:

**Issues:**
- ❌ Error messages hardcoded in 8+ command files
- ❌ Duplicate formatting (e.g., JSON error duplicated in `create.rs` and `update.rs`)
- ❌ Can't add error codes or help URLs later
- ❌ Can't change format globally (e.g., switch to JSON output)
- ❌ Testing specific errors is difficult
- ❌ No structured metadata for context
- ❌ Can't extract for internationalization

**Example of current problem:**
```rust
// commands/issue/create.rs:46
anyhow!("Invalid JSON data: {}\n\nExpected format: '{{\"description\":\"...\",\"priority\":1,\"kind\":\"bug\"}}'", e)

// commands/issue/update.rs:50 (DUPLICATE!)
anyhow!("Invalid JSON data: {}\n\nExpected format: '{{\"description\":\"...\",\"priority\":1,\"status\":\"closed\"}}'", e)

// commands/issue/update.rs:85-88
anyhow!("No updates specified.\n\nAvailable options:\n  --title <TITLE>...")
```

---

## Current State Analysis

### Error Definitions (`crates/beads-core/src/error.rs`)
```rust
#[derive(Debug, Error)]
pub enum BeadsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("ulid decode error: {0}")]
    UlidDecode(#[from] ulid::DecodeError),
    #[error("beads repository already initialized")]
    AlreadyInitialized,
    #[error("beads repository not found. Run 'beads init --prefix <prefix>' to create one.")]
    RepoNotFound,
    #[error("missing repository configuration: {0}")]
    MissingConfig(&'static str),
    #[error("update requires at least one field")]
    EmptyUpdate,
    #[error("blob not found: {0}")]
    BlobNotFound(String),
    #[error("invalid hash: {0}")]
    InvalidHash(String),
    #[error("{0}")]
    Custom(String),
}
```

**Current approach:** Simple `#[error(...)]` attributes with inline messages.

### CLI Error Usage (files using `anyhow!()`)
- `crates/beads/src/commands/issue/create.rs`
- `crates/beads/src/commands/issue/update.rs`
- `crates/beads/src/commands/create.rs` (deprecated)
- `crates/beads/src/commands/update.rs` (deprecated)
- `crates/beads/src/commands/show.rs` (deprecated)
- `crates/beads/src/commands/doc.rs`
- `crates/beads/src/commands/dep.rs`
- `crates/beads/src/commands/sync.rs`

---

## Proposed Solution: Structured, Evolvable Errors

### Core Principle
**Separate error DATA from PRESENTATION**

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│ CLI Layer (crates/beads)                                │
│ - Uses anyhow::Context for stack traces                │
│ - Catches BeadsError and adds CLI context              │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│ Core Layer (crates/beads-core)                          │
│ - BeadsError with structured variants                  │
│ - Builder methods for common errors                    │
│ - Custom Display impl (central formatting)             │
│ - Error codes + help URLs                              │
└─────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Structured Error Types (High Priority)

#### Step 1.1: Define Structured Variants

**File:** `crates/beads-core/src/error.rs`

```rust
use std::fmt::{self, Display};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BeadsError {
    // === System Errors (auto-derived via #[from]) ===
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("ulid decode error: {0}")]
    UlidDecode(#[from] ulid::DecodeError),

    // === Repository Errors ===
    #[error("beads repository already initialized")]
    AlreadyInitialized,

    #[error("beads repository not found")]
    RepoNotFound,

    #[error("missing repository configuration: {0}")]
    MissingConfig(&'static str),

    // === CLI Validation Errors (with structured data) ===

    /// No fields specified for update operation
    #[error("no updates specified")]
    EmptyUpdate {
        entity_id: String,
        available_fields: Vec<&'static str>,
    },

    /// Invalid JSON in --data flag
    #[error("invalid JSON data")]
    InvalidJsonData {
        source: serde_json::Error,
        expected_fields: Vec<&'static str>,
        context: &'static str,  // "create" or "update"
    },

    /// Required field is missing or empty
    #[error("required field missing: {field}")]
    MissingRequiredField {
        field: &'static str,
        suggestion: Option<String>,
    },

    /// Invalid document format in --doc flag
    #[error("invalid document format")]
    InvalidDocFormat {
        provided: String,
        expected: &'static str,
    },

    // === Resource Errors ===
    #[error("blob not found: {0}")]
    BlobNotFound(String),

    #[error("invalid hash: {0}")]
    InvalidHash(String),

    /// Generic fallback
    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, BeadsError>;
```

#### Step 1.2: Add Builder Methods

**File:** `crates/beads-core/src/error.rs`

```rust
impl BeadsError {
    /// Create EmptyUpdate error for issue updates
    pub fn empty_update(entity_id: impl Into<String>) -> Self {
        Self::EmptyUpdate {
            entity_id: entity_id.into(),
            available_fields: vec![
                "--title",
                "--description",
                "--kind",
                "--priority",
                "--status",
                "--add-label",
                "--remove-label",
                "--data",
            ],
        }
    }

    /// Create InvalidJsonData error for create command
    pub fn invalid_json_for_create(source: serde_json::Error) -> Self {
        Self::InvalidJsonData {
            source,
            expected_fields: vec!["description", "priority", "kind"],
            context: "create",
        }
    }

    /// Create InvalidJsonData error for update command
    pub fn invalid_json_for_update(source: serde_json::Error) -> Self {
        Self::InvalidJsonData {
            source,
            expected_fields: vec!["description", "priority", "status", "kind"],
            context: "update",
        }
    }

    /// Create MissingRequiredField error with optional suggestion
    pub fn missing_field(field: &'static str) -> Self {
        Self::MissingRequiredField {
            field,
            suggestion: None,
        }
    }

    /// Create MissingRequiredField error with suggestion
    pub fn missing_field_with_hint(field: &'static str, suggestion: String) -> Self {
        Self::MissingRequiredField {
            field,
            suggestion: Some(suggestion),
        }
    }

    /// Create InvalidDocFormat error
    pub fn invalid_doc_format(provided: impl Into<String>) -> Self {
        Self::InvalidDocFormat {
            provided: provided.into(),
            expected: "name:path",
        }
    }
}
```

#### Step 1.3: Custom Display Implementation

**File:** `crates/beads-core/src/error.rs`

```rust
impl Display for BeadsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // System errors use default Display from wrapped error
            Self::Io(e) => write!(f, "io error: {}", e),
            Self::Db(e) => write!(f, "database error: {}", e),
            Self::Serde(e) => write!(f, "serialization error: {}", e),
            Self::UlidDecode(e) => write!(f, "ulid decode error: {}", e),

            // Repository errors
            Self::AlreadyInitialized => {
                write!(f, "beads repository already initialized")
            }
            Self::RepoNotFound => {
                write!(f, "beads repository not found")?;
                write!(f, "\n\nRun 'beads init --prefix <prefix>' to create one")
            }
            Self::MissingConfig(key) => {
                write!(f, "missing repository configuration: {}", key)
            }

            // Structured CLI errors with rich formatting
            Self::EmptyUpdate { entity_id, available_fields } => {
                write!(f, "No updates specified for {}.\n\n", entity_id)?;
                write!(f, "Available options:\n")?;
                for field in available_fields {
                    write!(f, "  {}\n", field)?;
                }
                write!(f, "\nExample: beads issue update {} --status closed", entity_id)
            }

            Self::InvalidJsonData { source, expected_fields, context } => {
                write!(f, "Invalid JSON data: {}\n\n", source)?;
                write!(f, "Expected format for {}:\n{{\n", context)?;
                for field in expected_fields {
                    write!(f, "  \"{}\": <value>,\n", field)?;
                }
                write!(f, "}}")
            }

            Self::MissingRequiredField { field, suggestion } => {
                write!(f, "{} is required and cannot be empty", field)?;
                if let Some(hint) = suggestion {
                    write!(f, "\n\n{}", hint)?;
                }
                Ok(())
            }

            Self::InvalidDocFormat { provided, expected } => {
                write!(f, "Invalid doc format '{}'. Expected '{}'\n\n", provided, expected)?;
                write!(f, "Example: --doc readme:./README.md")
            }

            // Resource errors
            Self::BlobNotFound(hash) => write!(f, "blob not found: {}", hash),
            Self::InvalidHash(hash) => write!(f, "invalid hash: {}", hash),

            // Fallback
            Self::Custom(msg) => write!(f, "{}", msg),
        }
    }
}
```

#### Step 1.4: Update Command Handlers

**File:** `crates/beads/src/commands/issue/update.rs`

```rust
// BEFORE
if !has_field_updates && !has_label_operations {
    return Err(anyhow!(
        "No updates specified.\n\nAvailable options:\n  --title <TITLE>...",
        id
    ));
}

// AFTER
if !has_field_updates && !has_label_operations {
    return Err(BeadsError::empty_update(id).into());
}
```

**File:** `crates/beads/src/commands/issue/create.rs`

```rust
// BEFORE (line 39)
if title.trim().is_empty() {
    return Err(anyhow!("Title is required and cannot be empty"));
}

// AFTER
if title.trim().is_empty() {
    return Err(BeadsError::missing_field("title").into());
}

// BEFORE (line 46)
let parsed = serde_json::from_str::<serde_json::Value>(data_str)
    .map_err(|e| anyhow!("Invalid JSON data: {}\n\nExpected format: '{{\"description\":\"...\",\"priority\":1,\"kind\":\"bug\"}}'", e))?;

// AFTER
let parsed = serde_json::from_str::<serde_json::Value>(data_str)
    .map_err(BeadsError::invalid_json_for_create)?;

// BEFORE (line 107)
if parts.len() != 2 {
    eprintln!("Invalid doc format '{}'. Expected 'name:path'", doc_spec);
    continue;
}

// AFTER
if parts.len() != 2 {
    eprintln!("{}", BeadsError::invalid_doc_format(doc_spec));
    continue;
}
```

**File:** `crates/beads/src/commands/issue/update.rs`

```rust
// BEFORE (line 50)
let json = serde_json::from_str::<serde_json::Value>(&data_str)
    .map_err(|e| anyhow!("Invalid JSON data: {}\n\nExpected format: '{{\"description\":\"...\",\"priority\":1,\"status\":\"closed\"}}'", e))?;

// AFTER
let json = serde_json::from_str::<serde_json::Value>(&data_str)
    .map_err(BeadsError::invalid_json_for_update)?;
```

---

### Phase 2: Error Codes & Help URLs (Medium Priority)

#### Step 2.1: Add Error Code System

**File:** `crates/beads-core/src/error.rs`

```rust
impl BeadsError {
    /// Get error code for documentation/debugging
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "E000",
            Self::Db(_) => "E001",
            Self::Serde(_) => "E002",
            Self::UlidDecode(_) => "E003",
            Self::AlreadyInitialized => "E100",
            Self::RepoNotFound => "E101",
            Self::MissingConfig(_) => "E102",
            Self::EmptyUpdate { .. } => "E200",
            Self::InvalidJsonData { .. } => "E201",
            Self::MissingRequiredField { .. } => "E202",
            Self::InvalidDocFormat { .. } => "E203",
            Self::BlobNotFound(_) => "E300",
            Self::InvalidHash(_) => "E301",
            Self::Custom(_) => "E999",
        }
    }

    /// Get help URL for this error
    pub fn help_url(&self) -> Option<String> {
        Some(format!(
            "https://docs.beads.dev/errors/{}",
            self.code()
        ))
    }
}
```

#### Step 2.2: Update Display to Include Codes

```rust
impl Display for BeadsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print error code prefix
        write!(f, "error[{}]: ", self.code())?;

        // ... existing message formatting ...

        // Optionally add help URL at end
        if let Some(url) = self.help_url() {
            write!(f, "\n\nFor more information: {}", url)?;
        }

        Ok(())
    }
}
```

**Example output:**
```
error[E200]: No updates specified for bd-015.

Available options:
  --title
  --description
  --status

Example: beads issue update bd-015 --status closed

For more information: https://docs.beads.dev/errors/E200
```

---

### Phase 3: Enhanced Diagnostics (Future)

#### Option A: Colored Output (using `colored` crate)

```rust
// Add dependency: colored = "2"

impl Display for BeadsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use colored::*;

        match self {
            Self::EmptyUpdate { entity_id, available_fields } => {
                writeln!(f, "{}", "No updates specified".red().bold())?;
                writeln!(f, "\n{}", "Available options:".cyan())?;
                for field in available_fields {
                    writeln!(f, "  {}", field.green())?;
                }
                write!(f, "\n{} beads issue update {} --status closed",
                    "Example:".yellow(), entity_id)
            }
            // ...
        }
    }
}
```

#### Option B: Rich Diagnostics (using `miette` crate)

```rust
// Add dependency: miette = { version = "5", features = ["fancy"] }

use miette::{Diagnostic, SourceSpan};

#[derive(Error, Debug, Diagnostic)]
pub enum BeadsError {
    #[error("Invalid JSON in --data flag")]
    #[diagnostic(
        code(beads::invalid_json),
        help("Expected format: {{\"description\":\"...\",\"priority\":1}}")
    )]
    InvalidJsonData {
        #[source_code]
        src: String,

        #[label("invalid JSON here")]
        span: SourceSpan,
    },
    // ...
}
```

---

## Migration Strategy

### Step-by-Step Migration

1. **Phase 1.1-1.3:** Add new error variants + builders to `beads-core/src/error.rs`
   - No breaking changes, purely additive
   - Existing code continues to work

2. **Phase 1.4:** Migrate CLI commands one file at a time
   - Start with `commands/issue/update.rs` (most complex)
   - Then `commands/issue/create.rs`
   - Then deprecated commands
   - Each migration is self-contained

3. **Test after each file migration:**
   ```bash
   cargo build
   cargo test
   # Manual testing of specific command
   beads issue update bd-001
   beads issue create --title "test"
   ```

4. **Phase 2:** Add error codes (non-breaking)
   - Adds methods to `BeadsError`
   - Update `Display` impl
   - Test output format changes

5. **Phase 3:** Optional enhancements (future work)

### Backward Compatibility

- ✅ No breaking changes to `beads-core` API
- ✅ Error messages improve but functionality unchanged
- ✅ Old deprecated commands still work
- ✅ `events.jsonl` format unchanged

---

## Testing Strategy

### Unit Tests

**File:** `crates/beads-core/src/error.rs`

```rust
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
        let json_err = serde_json::from_str::<serde_json::Value>("invalid")
            .unwrap_err();
        let err = BeadsError::invalid_json_for_create(json_err);
        let msg = err.to_string();

        assert!(msg.contains("Invalid JSON"));
        assert!(msg.contains("create"));
        assert!(msg.contains("description"));
        assert!(msg.contains("priority"));
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(BeadsError::RepoNotFound.code(), "E101");
        assert_eq!(BeadsError::empty_update("test").code(), "E200");
    }

    #[test]
    fn test_help_urls() {
        let err = BeadsError::RepoNotFound;
        assert_eq!(
            err.help_url(),
            Some("https://docs.beads.dev/errors/E101".to_string())
        );
    }
}
```

### Integration Tests

**File:** `crates/beads/tests/error_messages.rs` (new file)

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn empty_update_shows_available_options() {
    Command::cargo_bin("beads")
        .unwrap()
        .args(&["issue", "update", "bd-001"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No updates specified"))
        .stderr(predicate::str::contains("--title"))
        .stderr(predicate::str::contains("--status"))
        .stderr(predicate::str::contains("Example:"));
}

#[test]
fn invalid_json_shows_expected_format() {
    Command::cargo_bin("beads")
        .unwrap()
        .args(&["issue", "create", "--title", "test", "--data", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"))
        .stderr(predicate::str::contains("Expected format"));
}

#[test]
fn missing_title_shows_error() {
    Command::cargo_bin("beads")
        .unwrap()
        .args(&["issue", "create", "--title", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("title"))
        .stderr(predicate::str::contains("required"));
}
```

---

## Clarifying Questions

### Before Implementation

1. **Error Code Format:**
   - Use simple numeric codes (`E001`, `E002`, ...) like rustc?
   - Or semantic codes (`REPO_001`, `CLI_001`, ...)?
   - Current proposal: Simple numeric with ranges (E000-E099 = system, E100-E199 = repo, E200-E299 = CLI)

2. **Help URLs:**
   - Create actual documentation at `docs.beads.dev/errors/*`?
   - Or link to GitHub issues/wiki?
   - Current proposal: Document in repo first (`docs/errors/E001.md`), publish later

3. **Colored Output:**
   - Add colored output now (Phase 1) or later (Phase 3)?
   - Respect `NO_COLOR` env var?
   - Current proposal: Add in Phase 3, respect `NO_COLOR`

4. **Verbose Mode:**
   - Add `--verbose` flag that shows error codes + stack traces?
   - Current proposal: Yes, add `BEADS_VERBOSE` env var

5. **JSON Output:**
   - Support `--json` flag for machine-readable errors?
   - Current proposal: Phase 3 enhancement

6. **Deprecated Commands:**
   - Migrate deprecated commands (`create.rs`, `update.rs`, `show.rs`) or skip?
   - Current proposal: Migrate for consistency, even if deprecated

---

## Benefits After Refactor

### Immediate (Phase 1)
✅ Single source of truth for error messages
✅ No duplicated formatting logic
✅ Easy to test specific error messages
✅ Structured error metadata
✅ Consistent error format across all commands

### Medium Term (Phase 2)
✅ Error codes for documentation
✅ Help URLs for detailed explanations
✅ Can filter/search by error code
✅ Analytics on which errors users hit most

### Long Term (Phase 3)
✅ Rich terminal diagnostics
✅ JSON output mode
✅ Internationalization ready
✅ Custom error reporters (e.g., send to logging service)

---

## Example Usage After Refactor

### Before
```rust
// create.rs
if title.trim().is_empty() {
    return Err(anyhow!("Title is required and cannot be empty"));
}

// update.rs
if !has_updates {
    return Err(anyhow!(
        "No updates specified.\n\nAvailable options:\n  --title <TITLE>\n  --description <DESCRIPTION>\n  --kind <KIND>\n  --priority <PRIORITY>\n  --status <STATUS>\n  --add-label <LABELS>\n  --remove-label <LABELS>\n  --data <JSON>\n\nExample: beads issue update {} --status closed",
        id
    ));
}

// create.rs (duplicate!)
.map_err(|e| anyhow!("Invalid JSON data: {}\n\nExpected format: '{{\"description\":\"...\",\"priority\":1,\"kind\":\"bug\"}}'", e))?
```

### After
```rust
// All commands
if title.trim().is_empty() {
    return Err(BeadsError::missing_field("title").into());
}

if !has_updates {
    return Err(BeadsError::empty_update(id).into());
}

.map_err(BeadsError::invalid_json_for_create)?
```

**All formatting lives in one place:** `beads-core/src/error.rs`

---

## Files to Modify

### Phase 1
- `crates/beads-core/src/error.rs` (major changes)
- `crates/beads/src/commands/issue/create.rs` (minor changes)
- `crates/beads/src/commands/issue/update.rs` (minor changes)
- `crates/beads/src/commands/create.rs` (minor changes)
- `crates/beads/src/commands/update.rs` (minor changes)
- `crates/beads/src/commands/doc.rs` (minor changes)
- `crates/beads/src/commands/dep.rs` (minor changes)

### Phase 2
- `crates/beads-core/src/error.rs` (add code/URL methods)
- `docs/errors/*.md` (new error documentation files)

### Phase 3 (Optional)
- `Cargo.toml` (add `colored` or `miette` dependency)
- `crates/beads-core/src/error.rs` (enhance Display impl)

---

## Timeline Estimate

- **Phase 1:** 2-3 hours (core refactor + migration)
- **Phase 2:** 1-2 hours (error codes + help system)
- **Phase 3:** 1-2 hours (enhanced diagnostics)

**Total:** 4-7 hours for complete implementation

---

## Success Criteria

✅ Zero `anyhow!()` with hardcoded multi-line messages in CLI commands
✅ All CLI validation errors use structured `BeadsError` variants
✅ Error messages formatted consistently via `Display` impl
✅ Can change error format globally by editing one file
✅ Can add new error metadata (codes, URLs) without touching commands
✅ Integration tests verify error messages
✅ No behavioral changes to user-facing functionality

---

## References

- rustc error handling: https://rustc-dev-guide.rust-lang.org/diagnostics.html
- miette crate: https://docs.rs/miette/
- thiserror crate: https://docs.rs/thiserror/
- Rust API Guidelines (error handling): https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err
