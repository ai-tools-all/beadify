# CLI Error Handling - Iteration 2

## Summary

Improved non-enum error messages to provide helpful context and actionable suggestions.

## Changes Made

### 1. Improved "No updates specified" Error

**File:** `crates/beads/src/commands/issue/update.rs`

**Before:**
```
Error: No updates specified
```

**After:**
```
Error: No updates specified.

Available options:
  --title <TITLE>
  --description <DESCRIPTION>
  --kind <KIND>
  --priority <PRIORITY>
  --status <STATUS>
  --add-label <LABELS>
  --remove-label <LABELS>
  --data <JSON>

Example: beads issue update bd-095 --status closed
```

### 2. Improved "Invalid JSON data" Error

**File:** `crates/beads/src/commands/issue/create.rs` and `update.rs`

**Before:**
```
Error: Invalid JSON data: expected value at line 1 column 1
```

**After:**
```
Error: Invalid JSON data: expected value at line 1 column 1

Expected format: '{"description":"...","priority":1,"kind":"bug"}'
```

### 3. Improved "Repository not found" Error

**File:** `crates/beads-core/src/error.rs`

**Before:**
```
Error: beads repository not found
```

**After:**
```
Error: beads repository not found. Run 'beads init --prefix <prefix>' to create one.
```

## Test Results

### ✅ All Error Messages Now Helpful

```bash
# No updates specified
$ beads issue update bd-095
Error: No updates specified.

Available options:
  --title <TITLE>
  --description <DESCRIPTION>
  ...

# Invalid JSON
$ beads issue create --title "Test" --data "invalid json"
Error: Invalid JSON data: expected value at line 1 column 1

Expected format: '{"description":"...","priority":1,"kind":"bug"}'

# Missing repo
$ beads issue list  # (in /tmp)
Error: beads repository not found. Run 'beads init --prefix <prefix>' to create one.
```

## Summary of Both Iterations

| Error Type | Before | After |
|------------|--------|-------|
| Invalid enum value | Silent default / cryptic error | Clear error with possible values list |
| Case sensitivity | Only lowercase worked | Case-insensitive (HIGH, High, high all work) |
| Numeric priority | Not supported | 0-3 aliases for low-urgent |
| No updates specified | "No updates specified" | Lists all available options with example |
| Invalid JSON | "Invalid JSON data: {error}" | Shows expected format hint |
| Missing repo | "beads repository not found" | Suggests `beads init` command |

## Next Steps (Optional)

1. Add integration tests with `assert_cmd` to verify error messages
2. Consider adding similar improvements to other commands (dep, label, doc)
3. Document the CLI error handling patterns for future contributors
