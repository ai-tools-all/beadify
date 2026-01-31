# CLI Error Handling with Clap Integration

## Goal
Helpful, context-aware error messages in the beads CLI. Typed enums at the clap layer with automatic validation, possible-value hints, and case-insensitive matching — while keeping numeric priority backward compat and preserving the events.jsonl format.

## Decisions (Resolved)
- Validate at clap layer (not business logic)
- Case-insensitive matching
- Flags override `--data` JSON
- `beads issue <action>` is the canonical path; old top-level commands get `#[command(hide = true)]`
- Priority accepts both names (`high`) and numbers (`2`) — stored as u32 internally
- events.jsonl format unchanged: priority=u32, kind=string, status=string

---

## Architecture: Old Path vs New Path

### NEW PATH (canonical) — `beads issue <action>`
| File | Purpose |
|------|---------|
| `crates/beads/src/cli/enums.rs` | `Priority`, `Kind`, `Status` with `ValueEnum` + custom `from_str` for numeric compat |
| `crates/beads/src/cli/mod.rs` | Re-exports |
| `crates/beads/src/commands/issue/create.rs` | Issue create using typed enums |
| `crates/beads/src/commands/issue/update.rs` | Issue update using typed enums |
| `crates/beads/src/commands/issue/list.rs` | Issue list with typed filters |
| `crates/beads/src/commands/issue/show.rs` | Issue show |
| `crates/beads/src/commands/issue/mod.rs` | Subcommand routing |

### OLD PATH (deprecated, hidden) — `beads create`, `beads update`, etc.
| File | Purpose | Status |
|------|---------|--------|
| `crates/beads/src/commands/create.rs` | Old create (requires `--data` JSON) | DEPRECATED — hide, delete in v2 |
| `crates/beads/src/commands/update.rs` | Old update (`--priority` as u32) | DEPRECATED — hide, delete in v2 |
| `crates/beads/src/commands/list.rs` | Old list (no kind/priority filters) | DEPRECATED — hide, delete in v2 |
| `crates/beads/src/commands/show.rs` | Old show | DEPRECATED — hide, delete in v2 |

### SHARED (both paths use)
| File | Purpose |
|------|---------|
| `crates/beads/src/commands/search.rs` | Search — migrate to typed enums |
| `crates/beads/src/commands/ready.rs` | Ready — no enum args, unchanged |
| `crates/beads/src/commands/sync.rs` | Sync — no enum args, unchanged |
| `crates/beads/src/commands/delete.rs` | Delete — no enum args, unchanged |
| `crates/beads/src/commands/dep.rs` | Dependencies — no enum args, unchanged |
| `crates/beads/src/commands/label.rs` | Labels — no enum args, unchanged |
| `crates/beads/src/commands/doc.rs` | Documents — no enum args, unchanged |

### TO DELETE (after typed enums land)
| File | Reason |
|------|--------|
| `crates/beads/src/cli/errors.rs` | Clap handles error messages natively with ValueEnum |

---

## events.jsonl Format (UNCHANGED)

Priority maps internally to u32 before hitting the core library. The event format is stable:

```json
{"event_id":"...","ts":"...","op":"create","id":"bd-001","actor":"...","data":{"title":"...","kind":"task","priority":1,"status":"open"}}
{"event_id":"...","ts":"...","op":"update","id":"bd-001","actor":"...","data":{"priority":3,"status":"in_progress"}}
```

- `priority`: always u32 (0=low, 1=medium, 2=high, 3=urgent)
- `kind`: always string ("bug", "feature", "refactor", "docs", "chore", "task")
- `status`: always string ("open", "in_progress", "review", "closed", "deleted")

Old events remain parsable. New CLI just changes how users *input* values, not how they're *stored*.

---

## Implementation Plan

### Step 1: Rework `cli/enums.rs` with `ValueEnum` + numeric compat

Replace hand-rolled parsing with clap's `ValueEnum` derive. Add custom `value_parser` for Priority that accepts both names and numbers.

```rust
#[derive(clap::ValueEnum, Clone, Copy)]
pub enum Kind {
    Bug, Feature, Refactor, Docs, Chore, Task,
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum Status {
    Open,
    #[value(alias = "in-progress", alias = "inprogress")]
    InProgress,
    Review, Closed,
}

// Priority needs custom parser for numeric backward compat
#[derive(Clone, Copy)]
pub enum Priority {
    Low = 0, Medium = 1, High = 2, Urgent = 3,
}

// Custom value_parser that accepts "high" OR "2"
pub fn parse_priority(s: &str) -> Result<Priority, String> {
    match s.to_lowercase().as_str() {
        "low" | "0" => Ok(Priority::Low),
        "medium" | "1" => Ok(Priority::Medium),
        "high" | "2" => Ok(Priority::High),
        "urgent" | "3" => Ok(Priority::Urgent),
        _ => Err(format!(
            "invalid priority '{}'. Valid: low (0), medium (1), high (2), urgent (3)",
            s
        )),
    }
}

impl Priority {
    pub fn as_u32(self) -> u32 { self as u32 }
}
impl Kind {
    pub fn as_str(&self) -> &'static str { /* match */ }
}
impl Status {
    pub fn as_str(&self) -> &'static str { /* match */ }
}
```

### Step 2: Update `IssueCommand` in `main.rs` to use typed enums

```rust
enum IssueCommand {
    Create {
        #[arg(short, long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, value_enum)]
        kind: Option<Kind>,                    // was Option<String>
        #[arg(long, value_parser = parse_priority)]
        priority: Option<Priority>,            // was Option<String>
        // ... rest unchanged
    },
    Update {
        // ...
        #[arg(long, value_enum)]
        kind: Option<Kind>,
        #[arg(long, value_parser = parse_priority)]
        priority: Option<Priority>,
        #[arg(long, value_enum)]
        status: Option<Status>,                // was Option<String>
        // ...
    },
    List {
        #[arg(long, value_enum)]
        status: Option<Status>,
        #[arg(long, value_parser = parse_priority)]
        priority: Option<Priority>,            // was Option<String>
        #[arg(long, value_enum)]
        kind: Option<Kind>,
        // ...
    },
}
```

### Step 3: Hide old commands in `main.rs`

```rust
#[derive(Subcommand)]
enum Commands {
    Init { ... },

    #[command(hide = true)]  // deprecated
    Create { ... },

    #[command(hide = true)]  // deprecated
    Show { ... },

    #[command(hide = true)]  // deprecated
    List { ... },

    #[command(hide = true)]  // deprecated
    Update { ... },

    // ... Sync, Search, Ready, Dep, Label, Doc, Delete stay visible
    // Issue stays visible (canonical path)
}
```

### Step 4: Update new-path command handlers

`commands/issue/create.rs`:
- Change signature: `priority: Option<Priority>` instead of `Option<String>`
- Remove manual `Priority::from_str()` call — already parsed by clap
- `final_priority = priority.map(|p| p.as_u32()).or(json_priority).unwrap_or(1)`

`commands/issue/update.rs`:
- Same pattern — remove manual parsing, use `.as_u32()`, `.as_str()`
- Delete the `invalid_enum_error_short` call — clap rejects bad values before we get here

`commands/issue/list.rs`:
- Accept typed `Option<Status>`, `Option<Priority>`, `Option<Kind>`
- Convert to strings/u32 when passing to core library

### Step 5: Migrate `search` command to typed enums

`commands/search.rs` currently uses `Option<String>` for kind/status and `Option<u32>` for priority. Update to use the typed enums for consistent UX across all commands.

### Step 6: Delete `cli/errors.rs`

No longer needed — clap generates error messages automatically. Remove the file and the `pub mod errors;` from `cli/mod.rs`.

### Step 7: Improve non-enum error messages

In command handlers, improve remaining hand-written errors:
- "No updates specified" → list available flags
- "Invalid JSON data" → show expected format hint
- Missing repo → suggest `beads init`

### Step 8: Add integration tests

Add `assert_cmd` dev-dependency. Test actual binary output:
- Invalid priority → shows valid values
- Invalid kind → shows valid values
- `--help` shows possible values inline
- Numeric priority still works (`--priority 2`)
- Case insensitive (`--priority HIGH`)
- Old hidden commands still work (backward compat)

---

## What This Gives Us

**Before:**
```
$ beads issue create --title "test" --priority critical
# silently defaults to medium, no error
```

**After:**
```
$ beads issue create --title "test" --priority critical
error: invalid priority 'critical'. Valid: low (0), medium (1), high (2), urgent (3)

$ beads issue create --help
  --priority <PRIORITY>  [possible values: low, medium, high, urgent]
  --kind <KIND>          [possible values: bug, feature, refactor, docs, chore, task]
```

Old commands still work but are hidden from `--help`:
```
$ beads create --title "test" --data '{"priority":2}'  # still works
$ beads --help  # doesn't show create/update/list/show
```
