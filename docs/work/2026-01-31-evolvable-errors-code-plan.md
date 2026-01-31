# Evolvable Error System - Code Plan

## What Was Already Implemented

Based on git commits 8b60dc3 → 21a1e01:

### ✅ Iteration 1: Enum Validation (commits 8b60dc3, df6effa, 8409eec)
- Custom `ValueEnum` for Priority, Kind, Status with case-insensitive matching
- Numeric aliases (0-3) for priority
- Migrated CLI args to typed enums
- Deleted `cli/errors.rs` (manual validation helpers)
- Hidden deprecated commands

### ✅ Iteration 2: Improved Messages (commit 21a1e01)
- "No updates specified" → lists all options + example
- "Invalid JSON data" → shows expected format
- "Repository not found" → suggests `beads init`

### ❌ Still Has Problems
```rust
// create.rs:46 and update.rs:50 (DUPLICATED!)
.map_err(|e| anyhow!("Invalid JSON data: {}\n\nExpected format: '{{...}}'", e))?

// update.rs:85-88 (HARDCODED, CAN'T EVOLVE)
anyhow!("No updates specified.\n\nAvailable options:\n  --title <TITLE>...")
```

**Issues:**
- ❌ Strings scattered across 8+ command files
- ❌ Duplicate formatting
- ❌ Can't add error codes later
- ❌ Can't change format globally
- ❌ No structured metadata

---

## Code Plan: Evolvable Error Refactor

### Phase 1: Structured Error Types

#### File 1: `crates/beads-core/src/error.rs`

**Add structured variants:**

```rust
#[derive(Debug, Error)]
pub enum BeadsError {
    // === Existing (keep as-is) ===
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
    #[error("beads repository not found")]  // ← Remove inline hint
    RepoNotFound,
    #[error("missing repository configuration: {0}")]
    MissingConfig(&'static str),
    #[error("blob not found: {0}")]
    BlobNotFound(String),
    #[error("invalid hash: {0}")]
    InvalidHash(String),
    #[error("{0}")]
    Custom(String),

    // === NEW: Structured CLI Errors ===
    #[error("update requires at least one field")]  // ← Keep simple message
    EmptyUpdate {
        entity_id: String,
        available_fields: Vec<&'static str>,
    },

    #[error("invalid JSON data")]  // ← Keep simple message
    InvalidJsonData {
        source: serde_json::Error,
        expected_fields: Vec<&'static str>,
        context: &'static str,  // "create" or "update"
    },

    #[error("required field missing: {field}")]
    MissingRequiredField {
        field: &'static str,
    },

    #[error("invalid document format")]
    InvalidDocFormat {
        provided: String,
    },
}
```

**Add builder methods:**

```rust
impl BeadsError {
    /// Create EmptyUpdate error for issue update
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
}
```

**Add custom Display implementation:**

```rust
use std::fmt::{self, Display};

impl Display for BeadsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // System errors - use default from wrapped error
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

            // Structured CLI errors - CENTRAL FORMATTING
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

            Self::MissingRequiredField { field } => {
                write!(f, "{} is required and cannot be empty", field)
            }

            Self::InvalidDocFormat { provided } => {
                write!(f, "Invalid doc format '{}'. Expected 'name:path'\n\n", provided)?;
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

**Estimated changes:** ~100 lines added to `error.rs`

---

#### File 2: `crates/beads/src/commands/issue/create.rs`

**Line 39: Empty title validation**

```rust
// BEFORE
if title.trim().is_empty() {
    return Err(anyhow!("Title is required and cannot be empty"));
}

// AFTER
if title.trim().is_empty() {
    return Err(BeadsError::missing_field("title").into());
}
```

**Line 46: Invalid JSON**

```rust
// BEFORE
let parsed = serde_json::from_str::<serde_json::Value>(data_str)
    .map_err(|e| anyhow!("Invalid JSON data: {}\n\nExpected format: '{{\"description\":\"...\",\"priority\":1,\"kind\":\"bug\"}}'", e))?;

// AFTER
let parsed = serde_json::from_str::<serde_json::Value>(data_str)
    .map_err(BeadsError::invalid_json_for_create)?;
```

**Line 107: Invalid doc format**

```rust
// BEFORE
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

**Add import:**
```rust
use beads_core::BeadsError;  // Add to existing imports
```

**Estimated changes:** 3 lines modified, 1 import added

---

#### File 3: `crates/beads/src/commands/issue/update.rs`

**Line 50: Invalid JSON**

```rust
// BEFORE
let json = serde_json::from_str::<serde_json::Value>(&data_str)
    .map_err(|e| anyhow!("Invalid JSON data: {}\n\nExpected format: '{{\"description\":\"...\",\"priority\":1,\"status\":\"closed\"}}'", e))?;

// AFTER
let json = serde_json::from_str::<serde_json::Value>(&data_str)
    .map_err(BeadsError::invalid_json_for_update)?;
```

**Line 85-88: Empty update**

```rust
// BEFORE
if !has_field_updates && !has_label_operations {
    return Err(anyhow!(
        "No updates specified.\n\nAvailable options:\n  --title <TITLE>\n  --description <DESCRIPTION>\n  --kind <KIND>\n  --priority <PRIORITY>\n  --status <STATUS>\n  --add-label <LABELS>\n  --remove-label <LABELS>\n  --data <JSON>\n\nExample: beads issue update {} --status closed",
        id
    ));
}

// AFTER
if !has_field_updates && !has_label_operations {
    return Err(BeadsError::empty_update(id).into());
}
```

**Add import:**
```rust
use beads_core::BeadsError;  // Add to existing imports
```

**Estimated changes:** 2 blocks simplified (12 lines → 2 lines), 1 import added

---

#### File 4: `crates/beads/src/commands/create.rs` (deprecated)

**Line ~40: Invalid JSON**

```rust
// BEFORE
let data = serde_json::from_str::<serde_json::Value>(&data_str)
    .map_err(|e| anyhow!("Invalid JSON data: {}", e))?;

// AFTER
let data = serde_json::from_str::<serde_json::Value>(&data_str)
    .map_err(BeadsError::invalid_json_for_create)?;
```

**Add import:**
```rust
use beads_core::BeadsError;
```

**Estimated changes:** 1 line modified, 1 import added

---

#### File 5: `crates/beads/src/commands/update.rs` (deprecated)

**Similar to File 4:** Replace JSON parsing error

**Estimated changes:** 1 line modified, 1 import added

---

### Phase 1 Summary

| File | Lines Changed | Type |
|------|---------------|------|
| `beads-core/src/error.rs` | +100 | Add variants, builders, Display impl |
| `beads/src/commands/issue/create.rs` | -10, +3 | Replace anyhow! with builders |
| `beads/src/commands/issue/update.rs` | -14, +2 | Replace anyhow! with builders |
| `beads/src/commands/create.rs` | -1, +1 | Replace anyhow! |
| `beads/src/commands/update.rs` | -1, +1 | Replace anyhow! |

**Total:** ~100 lines added, ~26 lines removed → Net +74 lines

**Benefits:**
✅ Zero hardcoded multi-line error strings
✅ Single source of truth for formatting
✅ No duplicate error messages
✅ Easy to add error codes later (Phase 2)

---

## Testing Strategy

### Unit Tests

Add to `crates/beads-core/src/error.rs`:

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
```

### Manual Testing

```bash
# Build
cargo build

# Test error messages
beads issue update bd-001  # → Should show "No updates specified" with all options
beads issue create --title "test" --data "invalid"  # → Should show JSON format hint
beads issue create --title ""  # → Should show "title is required"
cd /tmp && beads issue list  # → Should suggest beads init

# Verify existing functionality still works
beads issue create --title "test" --priority HIGH
beads issue update bd-001 --status closed
```

---

## Migration Steps

1. **Modify `error.rs`** (single file)
   - Add new variants
   - Add builder methods
   - Add Display impl
   - Add tests
   - Verify: `cargo build`

2. **Update `issue/create.rs`** (3 small changes)
   - Replace title validation
   - Replace JSON error
   - Replace doc format error
   - Verify: `cargo build && cargo test`

3. **Update `issue/update.rs`** (2 small changes)
   - Replace JSON error
   - Replace empty update error
   - Verify: `cargo build && cargo test`

4. **Update deprecated commands** (optional, 2 files)
   - Replace JSON errors in `create.rs` and `update.rs`
   - Verify: `cargo build`

5. **Manual testing** (all scenarios)
   - Test each error message type
   - Verify output matches expected format

6. **Commit**
   ```bash
   git add crates/beads-core/src/error.rs \
           crates/beads/src/commands/issue/create.rs \
           crates/beads/src/commands/issue/update.rs
   git commit -m "refactor(errors): implement evolvable error system with structured types"
   ```

---

## Phase 2: Error Codes (Optional Future Work)

After Phase 1 is stable, add error codes:

```rust
impl BeadsError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyUpdate { .. } => "E200",
            Self::InvalidJsonData { .. } => "E201",
            Self::MissingRequiredField { .. } => "E202",
            // ...
        }
    }

    pub fn help_url(&self) -> Option<String> {
        Some(format!("https://docs.beads.dev/errors/{}", self.code()))
    }
}

// Update Display to include code
impl Display for BeadsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error[{}]: ", self.code())?;
        // ... existing formatting ...
    }
}
```

**Output:**
```
error[E200]: No updates specified for bd-015.

Available options:
  --title
  --status

Example: beads issue update bd-015 --status closed

For more info: https://docs.beads.dev/errors/E200
```

---

## Success Criteria

✅ All error messages formatted via `Display` trait in `error.rs`
✅ Zero `anyhow!()` with multi-line hardcoded strings in commands
✅ CLI validation errors use structured `BeadsError` variants
✅ Can change error format globally by editing one file
✅ Unit tests verify error message content
✅ Manual testing shows same user-visible output
✅ No behavioral changes to functionality

---

## Timeline

- **Phase 1 implementation:** 1-2 hours
- **Testing:** 30 minutes
- **Total:** ~2.5 hours

---

## Files to Modify (Phase 1)

1. `crates/beads-core/src/error.rs` (major: +100 lines)
2. `crates/beads/src/commands/issue/create.rs` (minor: 3 changes)
3. `crates/beads/src/commands/issue/update.rs` (minor: 2 changes)
4. `crates/beads/src/commands/create.rs` (minor: 1 change, optional)
5. `crates/beads/src/commands/update.rs` (minor: 1 change, optional)

**Total files:** 3 required + 2 optional deprecated = 5 files
