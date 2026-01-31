# CLI Error Handling - Iteration 1

## Summary

Implemented clap `ValueEnum` integration for automatic validation and helpful error messages in the beads CLI.

## Changes Made

### 1. Updated `cli/enums.rs` with Custom ValueEnum Implementation

- Implemented `clap::ValueEnum` trait manually for `Priority`, `Kind`, and `Status` enums
- Added case-insensitive matching via aliases (e.g., "HIGH", "High", "high" all work)
- Added numeric aliases for priority (0, 1, 2, 3 map to low, medium, high, urgent)
- Added multiple format aliases for status (in-progress, in_progress, inprogress, etc.)

### 2. Updated `main.rs` CLI Arguments

- Changed `IssueCommand::Create`, `Update`, `List` to use typed enums (`Priority`, `Kind`, `Status`)
- Changed `Commands::Search` to use typed enums
- Added `#[arg(value_enum)]` attribute to enable clap validation
- Added `#[command(hide = true)]` to deprecated commands (Create, Show, List, Update)

### 3. Updated Command Handlers

- `commands/issue/create.rs`: Changed priority param from `Option<String>` to `Option<u32>`
- `commands/issue/update.rs`: Changed priority/status/kind to pre-validated types
- `commands/issue/list.rs`: Changed filter params to use pre-validated types
- Removed manual validation logic (now handled by clap)

### 4. Removed `cli/errors.rs`

- Deleted the manual error message helpers (no longer needed)
- Removed `pub mod errors;` from `cli/mod.rs`

## Test Results

### ✅ Working

```bash
# Help shows possible values
$ beads issue create --help
      --kind <KIND>
          Issue kind: bug, feature, refactor, docs, chore, task
          Possible values:
          - bug:      Bug fix
          - feature:  New feature
          ...

# Case-insensitive matching
$ beads issue create --title "Test" --priority HIGH
Created issue bd-095

$ beads issue create --title "Test 2" --priority 2 --kind BUG
Created issue bd-096

# Status update with different formats
$ beads issue update bd-095 --status CLOSED
Updated issue bd-095

# Invalid values show helpful error
$ beads issue create --title "Test" --priority critical
error: invalid value 'critical' for '--priority <PRIORITY>'
  [possible values: low, medium, high, urgent]
```

### ⚠️ Known Issues

1. **"No updates specified" error** - When running `beads issue update <id>` without flags, the error message doesn't list available options:
   ```
   $ beads issue update bd-095
   Error: No updates specified
   ```
   Should be: `"No updates specified. Use: --title, --description, --kind, --priority, --status, --add-label, --remove-label"`

2. **Deprecated commands hidden** - Old commands still work but are hidden from help:
   ```
   $ beads --help  # No longer shows create, show, list, update
   ```

## Next Steps (Iteration 2)

1. Improve non-enum error messages:
   - "No updates specified" → list available flags
   - "Invalid JSON data" → show expected format hint
   - Missing repo → suggest `beads init`

2. Test edge cases:
   - Numeric priority 0-3
   - Mixed case variations
   - Invalid enum values

3. Consider adding integration tests with `assert_cmd`
