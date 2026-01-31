# Add createdAt Timestamp Tracking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable createdAt timestamp tracking for all issues and support date-based queries (e.g., "issues created in past week").

**Architecture:** 
- Add `created_at` field to the `Issue` struct, capturing the timestamp from the first create event
- Persist `created_at` in the SQLite `issues` table  
- Track schema version in `_meta` table (beads_version, schema_version)
- Automatically migrate on `beads sync` if `created_at` column missing
- Implement datetime query helpers for "past X days/weeks"
- Add a `created_after` and `created_before` filter to the list command
- Support optional query syntax like `beads list --created-since "1 week ago"` or `beads list --created-after "2026-01-20"`
- Support `beads sync --force` to rebuild SQLite cache from events.jsonl

**Backward Compatibility:**
- ✅ Fully automatic - users run `beads sync`, timestamps are populated from events.jsonl
- ✅ No breaking changes - uses soft migration with schema version tracking
- ✅ Users can manually rebuild with `beads sync --force` if needed

**Tech Stack:** Rust, SQLite, Chrono (datetime), chrono-tz (timezone), serde_json

**Timezone Strategy:**
- **Storage:** Always UTC in SQLite and events.jsonl
- **Queries:** Parse user input in their local timezone, convert to UTC for database queries
- **Display:** Show both UTC and local timezone in JSON output
- **Detection:** Auto-detect from `--timezone` flag → `TZ` env var → system default → UTC fallback
- **Config:** Support `~/.beads/config.toml` for persistent default timezone setting

---

## Phase 1: Schema & Model Updates

### Task 1: Update Issue struct and add created_at field

**Files:**
- Modify: `crates/beads-core/src/model.rs`
- Modify: `crates/beads-core/src/db.rs` (schema and upsert)

**Step 1: Add created_at to Issue struct**

In `model.rs`, update the `Issue` struct to include a `created_at` field:

```rust
#[derive(Debug, Clone)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub priority: u32,
    pub status: String,
    pub created_at: String,  // ISO 8601 timestamp
    pub description: Option<String>,
    pub design: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub notes: Option<String>,
    pub data: Option<serde_json::Value>,
}
```

**Step 2: Update SQLite schema**

In `db.rs`, modify `create_schema()` to add `created_at` column:

```rust
pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS issues (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            kind TEXT NOT NULL,
            priority INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'open',
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            description TEXT,
            design TEXT,
            acceptance_criteria TEXT,
            notes TEXT,
            data TEXT
        );
        -- ... rest of schema
        "#,
    )?;
    
    // Add migration for existing tables
    let mut stmt = conn.prepare("PRAGMA table_info(issues)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if !columns.contains(&"created_at".to_string()) {
        conn.execute("ALTER TABLE issues ADD COLUMN created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP", [])?;
    }
    
    // ... rest of migrations
    Ok(())
}
```

**Step 3: Update upsert_issue function**

In `db.rs`, update the `upsert_issue()` function to handle created_at:

```rust
pub fn upsert_issue(tx: &Transaction<'_>, issue: &Issue) -> Result<()> {
    let data = issue.data.as_ref().map(|v| v.to_string());
    tx.execute(
        r#"
        INSERT INTO issues (id, title, kind, priority, status, created_at, description, design, acceptance_criteria, notes, data)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            kind = excluded.kind,
            priority = excluded.priority,
            status = excluded.status,
            description = excluded.description,
            design = excluded.design,
            acceptance_criteria = excluded.acceptance_criteria,
            notes = excluded.notes,
            data = excluded.data
        "#,
        params![
            issue.id,
            issue.title,
            issue.kind,
            issue.priority,
            issue.status,
            issue.created_at,
            issue.description,
            issue.design,
            issue.acceptance_criteria,
            issue.notes,
            data,
        ],
    )?;
    Ok(())
}
```

**Step 4: Add schema versioning to _meta table**

Update `create_schema()` to initialize version tracking:

```rust
pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- ... existing schema ...
        CREATE TABLE IF NOT EXISTS _meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;
    
    // Initialize schema version if not set
    let mut stmt = conn.prepare("SELECT value FROM _meta WHERE key = 'schema_version'")?;
    let has_version: bool = stmt.exists([])?;
    drop(stmt);
    
    if !has_version {
        conn.execute("INSERT INTO _meta (key, value) VALUES ('schema_version', '1.0')", [])?;
        conn.execute("INSERT INTO _meta (key, value) VALUES ('beads_version', env!('CARGO_PKG_VERSION'))", [])?;
    }
    
    // Soft migration: add created_at column if missing
    let mut stmt = conn.prepare("PRAGMA table_info(issues)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if !columns.contains(&"created_at".to_string()) {
        conn.execute("ALTER TABLE issues ADD COLUMN created_at TEXT", [])?;
    }

    Ok(())
}

/// Check if database needs timestamp migration
pub fn needs_timestamp_migration(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info(issues)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    
    Ok(!columns.contains(&"created_at".to_string()))
}
```

**Step 5: Run tests to ensure schema works**

Run: `cargo test -p beads-core --lib`
Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/beads-core/src/model.rs crates/beads-core/src/db.rs
git commit -m "feat: add created_at timestamp field and schema versioning"
```

---

## Phase 2: Event Handling & Timestamp Capture

### Task 2: Capture created_at from first create event

**Files:**
- Modify: `crates/beads-core/src/log.rs`
- Reference: Check `apply_event()` function to understand event replay

**Step 1: Update apply_event to capture created_at from first create**

In `log.rs`, find the `apply_event()` function and modify it to set `created_at` from the event timestamp:

```rust
pub fn apply_event(event: &Event, issues: &mut std::collections::HashMap<String, Issue>) -> Result<()> {
    let issue = issues.entry(event.id.clone()).or_insert_with(|| Issue {
        id: event.id.clone(),
        title: String::new(),
        kind: String::new(),
        priority: 0,
        status: String::new(),
        created_at: event.ts.clone(),  // Capture timestamp from first create
        description: None,
        design: None,
        acceptance_criteria: None,
        notes: None,
        data: None,
    });

    match event.op {
        OpKind::Create => {
            // Only set created_at on first create, don't overwrite on updates
            if issue.created_at.is_empty() {
                issue.created_at = event.ts.clone();
            }
            // ... rest of create logic
        }
        // ... rest of operations
    }
    Ok(())
}
```

**Step 2: Check event structure in append_create_event**

Verify that `append_create_event()` in `log.rs` already includes `ts` field (should be automatic). No changes needed if `ts` is already captured.

**Step 3: Create automatic migration logic in apply_all_events**

In `log.rs`, modify `apply_all_events()` to detect and perform timestamp migration:

```rust
pub fn apply_all_events(
    repo: &BeadsRepo,
    conn: &mut Connection,
    force_rebuild: bool,  // New parameter
) -> Result<(usize, u64, Option<String>)> {
    let file = match File::open(repo.log_path()) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok((0, 0, None));
        }
        Err(err) => return Err(err.into()),
    };

    let reader = BufReader::new(file);

    // Check if migration needed
    let needs_migration = db::needs_timestamp_migration(conn)?;
    if needs_migration || force_rebuild {
        if needs_migration {
            eprintln!("Detected old database schema. Migrating timestamps from events.jsonl...");
        }
        // Clear existing cache if force_rebuild or schema needs migration
        db::clear_state(conn)?;
    }

    let mut events = Vec::new();
    let mut offset = 0u64;

    for line in reader.lines() {
        let line = line?;
        let line_len = line.len() as u64 + 1;
        offset += line_len;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let event: Event = serde_json::from_str(trimmed)?;
        events.push(event);
    }

    events.sort_by(|a, b| a.event_id.cmp(&b.event_id));

    let tx = conn.transaction()?;
    
    // Only clear if we haven't already
    if !needs_migration && !force_rebuild {
        db::clear_state(&tx)?;
    }

    let mut last_event = None;
    for event in &events {
        apply_event(&tx, event)?;
        last_event = Some(event.event_id.clone());
    }

    if let Some(ref event_id) = last_event {
        db::set_meta(&tx, "last_event_id", event_id.clone())?;
        db::set_meta(&tx, "last_processed_offset", offset.to_string())?;
        // Update schema version after successful migration
        db::set_meta(&tx, "schema_version", "1.1")?;
    }
    
    tx.commit()?;

    Ok((events.len(), offset, last_event))
}
```

**Step 4: Update sync command to pass force flag**

The sync command (crates/beads/src/commands/sync.rs) already calls sync_repo. We need to expose the --force flag and pass it through:

In `sync.rs`:

```rust
pub fn run(repo: BeadsRepo, full: bool, force: bool) -> Result<()> {
    Command::new("git")
        .arg("pull")
        .current_dir(repo.root())
        .status()
        .with_context(|| "failed to run git pull")?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("git pull failed"))?;

    // sync_repo will automatically detect and migrate timestamps
    // --force rebuilds the cache even if not needed
    let applied = sync_repo(&repo, full, force)?;
    println!("Applied {applied} events");

    Command::new("git")
        .arg("push")
        .current_dir(repo.root())
        .status()
        .with_context(|| "failed to run git push")?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("git push failed"))?;

    Ok(())
}
```

**Step 5: Run tests**

Run: `cargo test -p beads-core --lib`
Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/beads-core/src/log.rs crates/beads/src/commands/sync.rs
git commit -m "feat: add automatic timestamp migration to beads sync"
```

---

## Phase 2.5: Timezone Detection & Handling

### Task 2.5: Create timezone detection and conversion module

**Files:**
- Create: `crates/beads-core/src/tz.rs` (new file)
- Modify: `crates/beads-core/src/lib.rs` (export tz module)
- Modify: `crates/beads-core/Cargo.toml` (add chrono-tz dependency)

**Step 1: Add chrono-tz to Cargo.toml**

In `crates/beads-core/Cargo.toml`, add to dependencies:

```toml
chrono-tz = { version = "0.8", features = ["serde"] }
```

**Step 2: Create tz.rs with timezone detection**

Create `crates/beads-core/src/tz.rs`:

```rust
use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use crate::error::Result;
use std::str::FromStr;

/// Get user's timezone from priority order:
/// 1. Explicit parameter (from CLI --timezone flag)
/// 2. TZ environment variable
/// 3. System default timezone
/// 4. UTC fallback
pub fn get_user_timezone(explicit_tz: Option<&str>) -> Result<Tz> {
    // 1. CLI flag override
    if let Some(tz_str) = explicit_tz {
        return Tz::from_str(tz_str)
            .map_err(|_| crate::error::Error::Other(format!("Invalid timezone: {}", tz_str)));
    }

    // 2. TZ environment variable
    if let Ok(tz_str) = std::env::var("TZ") {
        return Tz::from_str(&tz_str)
            .map_err(|_| crate::error::Error::Other(format!("Invalid TZ env var: {}", tz_str)));
    }

    // 3. System default timezone detection
    #[cfg(unix)]
    {
        // Try /etc/timezone
        if let Ok(tz_str) = std::fs::read_to_string("/etc/timezone") {
            let tz_name = tz_str.trim();
            if let Ok(tz) = Tz::from_str(tz_name) {
                return Ok(tz);
            }
        }

        // Try symlink /etc/localtime
        if let Ok(link) = std::fs::read_link("/etc/localtime") {
            if let Some(path_str) = link.to_str() {
                if let Some(tz_name) = path_str.split('/').last() {
                    if let Ok(tz) = Tz::from_str(tz_name) {
                        return Ok(tz);
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // Try Windows Registry or system API
        // Fallback to UTC if not available
    }

    // 4. Fallback to UTC
    Ok(Tz::UTC)
}

/// Convert UTC timestamp to user's local timezone
/// Returns formatted string: "2026-01-31 15:30 EST"
pub fn utc_to_local_string(utc_timestamp: &str, user_tz: Tz) -> Result<String> {
    let utc_dt = DateTime::parse_from_rfc3339(utc_timestamp)
        .map_err(|e| crate::error::Error::Other(format!("Failed to parse timestamp: {}", e)))?;
    
    let local_dt = utc_dt.with_timezone(&user_tz);
    let abbr = user_tz.name(&local_dt);
    
    Ok(format!("{} {}", local_dt.format("%Y-%m-%d %H:%M"), abbr))
}

/// Parse relative date expression in user's local timezone and return UTC timestamp
/// Examples: "1 week ago", "3 days ago", "2 months ago"
pub fn parse_relative_in_timezone(expr: &str, user_tz: Tz) -> Result<String> {
    let expr = expr.to_lowercase();
    let parts: Vec<&str> = expr.split_whitespace().collect();

    if parts.len() < 3 || parts[parts.len() - 1] != "ago" {
        return Err(crate::error::Error::Other(
            "Expected format: 'N days ago' or 'N weeks ago'".to_string(),
        ));
    }

    let num: i64 = parts[0]
        .parse()
        .map_err(|_| crate::error::Error::Other(format!("Invalid number: {}", parts[0])))?;
    
    let unit = parts[parts.len() - 2];

    // Get current time in user's timezone
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&user_tz);

    // Calculate target time in user's timezone
    let target_local = match unit {
        "day" | "days" => now_local - Duration::days(num),
        "week" | "weeks" => now_local - Duration::weeks(num),
        "month" | "months" => now_local - Duration::days(num * 30),
        "year" | "years" => now_local - Duration::days(num * 365),
        _ => {
            return Err(crate::error::Error::Other(format!(
                "Unknown time unit: {}. Use days, weeks, months, or years",
                unit
            )))
        }
    };

    // Convert back to UTC for storage
    let target_utc = target_local.with_timezone(&Utc);
    Ok(target_utc.to_rfc3339())
}

/// Parse absolute date in user's local timezone
/// Assumes midnight (00:00) in user's timezone
/// Examples: "2026-01-20", "2026-01-20T15:30:00"
pub fn parse_absolute_in_timezone(date_str: &str, user_tz: Tz) -> Result<String> {
    // Try ISO 8601 with time
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Ok(dt.to_rfc3339());
    }

    // Try date-only format YYYY-MM-DD (assume midnight local time)
    if let Ok(naive_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
        let local_dt = user_tz
            .from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| {
                crate::error::Error::Other(format!(
                    "Ambiguous or invalid local datetime: {}",
                    date_str
                ))
            })?;
        return Ok(local_dt.with_timezone(&Utc).to_rfc3339());
    }

    Err(crate::error::Error::Other(format!(
        "Invalid date format: {}. Use YYYY-MM-DD or ISO 8601",
        date_str
    )))
}

/// Parse either relative or absolute date expression
pub fn parse_date_in_timezone(date_str: &str, user_tz: Tz) -> Result<String> {
    if date_str.contains("ago") {
        parse_relative_in_timezone(date_str, user_tz)
    } else {
        parse_absolute_in_timezone(date_str, user_tz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_timezone_utc_fallback() {
        let tz = get_user_timezone(None).unwrap();
        // Should return UTC or system default
        assert!(!tz.name(&Utc::now().with_timezone(&tz)).is_empty());
    }

    #[test]
    fn test_get_user_timezone_explicit() {
        let tz = get_user_timezone(Some("America/New_York")).unwrap();
        assert_eq!(tz, Tz::America__New_York);
    }

    #[test]
    fn test_utc_to_local_string() {
        let utc_ts = "2026-01-31T20:00:00Z";
        let est = Tz::America__New_York;
        let local = utc_to_local_string(utc_ts, est).unwrap();
        assert!(local.contains("2026-01-31"));
        assert!(local.contains("EST") || local.contains("EDT"));
    }

    #[test]
    fn test_parse_relative_in_timezone() {
        let tz = Tz::UTC;
        let result = parse_relative_in_timezone("1 week ago", tz).unwrap();
        let dt = DateTime::parse_from_rfc3339(&result).unwrap();
        let now = Utc::now();
        // Should be approximately 7 days ago
        let diff = now.signed_duration_since(dt);
        assert!(diff.num_days() >= 6 && diff.num_days() <= 8);
    }

    #[test]
    fn test_parse_absolute_in_timezone() {
        let tz = Tz::UTC;
        let result = parse_absolute_in_timezone("2026-01-20", tz).unwrap();
        let dt = DateTime::parse_from_rfc3339(&result).unwrap();
        // Should be 2026-01-20 at 00:00 UTC
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 20);
    }
}
```

**Step 3: Update lib.rs to export tz module**

In `crates/beads-core/src/lib.rs`, add:

```rust
pub mod tz;
```

**Step 4: Run tests**

Run: `cargo test -p beads-core tz`
Expected: All timezone tests pass

**Step 5: Commit**

```bash
git add crates/beads-core/src/tz.rs crates/beads-core/src/lib.rs crates/beads-core/Cargo.toml
git commit -m "feat: add timezone detection and conversion module"
```

---

## Phase 3: Query Helpers

### Task 3: Add datetime query helper functions

**Files:**
- Create: `crates/beads-core/src/query.rs` (new file)
- Modify: `crates/beads-core/src/lib.rs` (export new module)

**Step 1: Create query.rs with datetime parsing that uses timezone-aware conversion**

Create new file `crates/beads-core/src/query.rs`:

```rust
use chrono::DateTime;
use chrono_tz::Tz;
use crate::error::Result;

/// Parse date expression (relative or absolute) in user's timezone
/// Wrapper around tz module functions that returns UTC timestamp string
pub fn parse_date(date_str: &str, user_tz: Tz) -> Result<String> {
    crate::tz::parse_date_in_timezone(date_str, user_tz)
}

/// Parse relative date expression like "1 week ago" in user's timezone
pub fn parse_relative_date(expr: &str, user_tz: Tz) -> Result<String> {
    crate::tz::parse_relative_in_timezone(expr, user_tz)
}

/// Parse absolute date like "2026-01-20" in user's timezone
pub fn parse_absolute_date(date_str: &str, user_tz: Tz) -> Result<String> {
    crate::tz::parse_absolute_in_timezone(date_str, user_tz)
}

/// Check if issue was created after given UTC timestamp
pub fn created_after(issue_created_at: &str, after_date: &str) -> bool {
    if let (Ok(issue_dt), Ok(after_dt)) = (
        DateTime::parse_from_rfc3339(issue_created_at),
        DateTime::parse_from_rfc3339(after_date),
    ) {
        issue_dt >= after_dt
    } else {
        false
    }
}

/// Check if issue was created before given UTC timestamp
pub fn created_before(issue_created_at: &str, before_date: &str) -> bool {
    if let (Ok(issue_dt), Ok(before_dt)) = (
        DateTime::parse_from_rfc3339(issue_created_at),
        DateTime::parse_from_rfc3339(before_date),
    ) {
        issue_dt <= before_dt
    } else {
        false
    }
}
```

**Step 2: Update lib.rs to export query module**

In `crates/beads-core/src/lib.rs`, add:

```rust
pub mod query;
```

**Step 3: Add chrono dependency**

In `crates/beads-core/Cargo.toml`, ensure chrono is in dependencies:

```toml
chrono = { version = "0.4", features = ["serde"] }
```

**Step 4: Write unit tests for query helpers**

Create `crates/beads-core/src/query.rs` tests at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_relative_date_days_ago() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_relative_date("3 days ago", tz).unwrap();
        // Should return valid RFC3339 date
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_relative_date_weeks_ago() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_relative_date("1 week ago", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_absolute_date_iso() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_absolute_date("2026-01-20T10:00:00Z", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_absolute_date_only() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_absolute_date("2026-01-20", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_date_auto_detects_relative() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_date("1 week ago", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_parse_date_auto_detects_absolute() {
        let tz = chrono_tz::Tz::UTC;
        let result = parse_date("2026-01-20", tz).unwrap();
        assert!(DateTime::parse_from_rfc3339(&result).is_ok());
    }

    #[test]
    fn test_created_after() {
        let issue_date = "2026-01-25T10:00:00Z";
        let after_date = "2026-01-20T00:00:00Z";
        assert!(created_after(issue_date, after_date));
    }

    #[test]
    fn test_created_before() {
        let issue_date = "2026-01-15T10:00:00Z";
        let before_date = "2026-01-20T00:00:00Z";
        assert!(created_before(issue_date, before_date));
    }
}
```

**Step 5: Run tests**

Run: `cargo test -p beads-core query`
Expected: All tests pass

**Step 6: Commit**

```bash
git add crates/beads-core/src/query.rs crates/beads-core/src/lib.rs crates/beads-core/Cargo.toml
git commit -m "feat: add datetime query helpers for filtering by creation date"
```

---

## Phase 4: Database Query Function

### Task 4: Add created_at filters to database queries

**Files:**
- Modify: `crates/beads-core/src/db.rs`
- Modify: `crates/beads-core/src/lib.rs` (add public query function)

**Step 1: Add get_issues_created_after function in db.rs**

```rust
pub fn get_issues_created_after(conn: &Connection, after_date: &str, status_filter: Option<&str>) -> Result<Vec<Issue>> {
    let query = if let Some(status) = status_filter {
        format!(
            "SELECT id, title, kind, priority, status, created_at, description, design, acceptance_criteria, notes, data 
             FROM issues 
             WHERE created_at >= ? AND status = ?
             ORDER BY created_at DESC"
        )
    } else {
        "SELECT id, title, kind, priority, status, created_at, description, design, acceptance_criteria, notes, data 
         FROM issues 
         WHERE created_at >= ?
         ORDER BY created_at DESC".to_string()
    };

    let mut stmt = conn.prepare(&query)?;
    let issues = if let Some(status) = status_filter {
        stmt.query_map(params![after_date, status], |row| {
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: row.get(2)?,
                priority: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                description: row.get(6)?,
                design: row.get(7)?,
                acceptance_criteria: row.get(8)?,
                notes: row.get(9)?,
                data: row.get::<_, Option<String>>(10)?.and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?
    } else {
        stmt.query_map(params![after_date], |row| {
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: row.get(2)?,
                priority: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                description: row.get(6)?,
                design: row.get(7)?,
                acceptance_criteria: row.get(8)?,
                notes: row.get(9)?,
                data: row.get::<_, Option<String>>(10)?.and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?
    };

    issues.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
}
```

**Step 2: Add get_issues_created_between function in db.rs**

```rust
pub fn get_issues_created_between(conn: &Connection, after_date: &str, before_date: &str, status_filter: Option<&str>) -> Result<Vec<Issue>> {
    let query = if let Some(status) = status_filter {
        "SELECT id, title, kind, priority, status, created_at, description, design, acceptance_criteria, notes, data 
         FROM issues 
         WHERE created_at >= ? AND created_at <= ? AND status = ?
         ORDER BY created_at DESC"
    } else {
        "SELECT id, title, kind, priority, status, created_at, description, design, acceptance_criteria, notes, data 
         FROM issues 
         WHERE created_at >= ? AND created_at <= ?
         ORDER BY created_at DESC"
    };

    let mut stmt = conn.prepare(query)?;
    let issues = if let Some(status) = status_filter {
        stmt.query_map(params![after_date, before_date, status], |row| make_issue_from_row(row))?
    } else {
        stmt.query_map(params![after_date, before_date], |row| make_issue_from_row(row))?
    };

    issues.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
}

fn make_issue_from_row(row: &rusqlite::Row) -> rusqlite::Result<Issue> {
    Ok(Issue {
        id: row.get(0)?,
        title: row.get(1)?,
        kind: row.get(2)?,
        priority: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        description: row.get(6)?,
        design: row.get(7)?,
        acceptance_criteria: row.get(8)?,
        notes: row.get(9)?,
        data: row.get::<_, Option<String>>(10)?.and_then(|s| serde_json::from_str(&s).ok()),
    })
}
```

**Step 3: Expose functions in lib.rs**

In `crates/beads-core/src/lib.rs`, add public exports:

```rust
pub use db::{
    get_issues_created_after,
    get_issues_created_between,
};
```

**Step 4: Run tests**

Run: `cargo test -p beads-core`
Expected: All tests pass

**Step 5: Commit**

```bash
git add crates/beads-core/src/db.rs crates/beads-core/src/lib.rs
git commit -m "feat: add database queries for filtering by created_at timestamp"
```

---

## Phase 5: CLI Integration

### Task 5: Add --created-after and --created-before flags to list command

**Files:**
- Modify: `crates/beads/src/commands/issue/list.rs`
- Reference: Check clap command structure for flag definitions

**Step 1: Add datetime flags to the list command struct**

In `crates/beads/src/main.rs`, update the `IssueCommand::List` variant:

```rust
#[derive(Subcommand)]
enum IssueCommand {
    // ... existing variants ...
    
    /// List issues with filtering
    List {
        #[arg(long)]
        all: bool,

        /// Filter by status: open, in-progress, review, closed
        #[arg(long, value_enum)]
        status: Option<Status>,

        /// Filter by priority: low, medium, high, urgent (or 0-3)
        #[arg(long, value_enum)]
        priority: Option<Priority>,

        /// Filter by kind: bug, feature, refactor, docs, chore, task
        #[arg(long, value_enum)]
        kind: Option<Kind>,

        #[arg(long)]
        label: Option<String>,

        #[arg(long)]
        flat: bool,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        labels: bool,
        
        // New datetime filters
        #[arg(long, help = "Filter issues created after this date (e.g., '2026-01-20' or '1 week ago')")]
        created_after: Option<String>,

        #[arg(long, help = "Filter issues created before this date (e.g., '2026-01-20' or '1 week ago')")]
        created_before: Option<String>,
        
        #[arg(long, help = "User timezone for date parsing (e.g., 'America/New_York'). Default: system timezone")]
        timezone: Option<String>,
    },
}
```

**Step 2: Update the main.rs handler to pass timezone to list command**

In `main.rs`, update the `Issue(IssueCommand)` match arm:

```rust
IssueCommand::List {
    all,
    status,
    priority,
    kind,
    label,
    flat,
    json,
    labels,
    created_after,
    created_before,
    timezone,
} => {
    info!(command = "issue list", all, status = ?status, priority = ?priority, kind = ?kind);
    commands::issue::list::run(
        repo.unwrap(),
        all,
        status.map(|s| s.as_str().to_string()),
        priority.map(|p| p.as_u32()),
        kind.map(|k| k.as_str().to_string()),
        label,
        None,
        flat,
        json,
        labels,
        created_after,
        created_before,
        timezone,
    )?;
}
```

**Step 3: Parse and apply date filters in the execute function**

In `crates/beads/src/commands/issue/list.rs`, update the `run` function signature and add timezone handling:

```rust
pub fn run(
    repo: BeadsRepo,
    all: bool,
    status: Option<String>,
    priority: Option<u32>,
    kind: Option<String>,
    label: Option<String>,
    label_any: Option<String>,
    flat: bool,
    json: bool,
    labels_col: bool,
    created_after: Option<String>,
    created_before: Option<String>,
    timezone: Option<String>,
) -> Result<()> {
    // ... existing code ...

    // Get user's timezone
    let user_tz = beads_core::tz::get_user_timezone(timezone.as_deref())?;

    let mut issues = get_all_issues(&repo)?;

    // Apply created_after filter
    if let Some(ref date_str) = created_after {
        let after_date = beads_core::query::parse_date(date_str, user_tz)?;
        issues.retain(|issue| beads_core::query::created_after(&issue.created_at, &after_date));
    }

    // Apply created_before filter
    if let Some(ref date_str) = created_before {
        let before_date = beads_core::query::parse_date(date_str, user_tz)?;
        issues.retain(|issue| beads_core::query::created_before(&issue.created_at, &before_date));
    }

    // ... rest of list logic ...
    Ok(())
}
```

**Step 4: Update JSON output to show both UTC and local timestamps**

In `crates/beads/src/commands/issue/list.rs`, when building JSON output, add timezone conversion:

```rust
// When building JSON for each issue
let created_at_local = beads_core::tz::utc_to_local_string(&issue.created_at, user_tz)?;

json!({
    "id": issue.id,
    "created_at_utc": issue.created_at,
    "created_at_local": created_at_local,
    // ... other fields
})
```

**Step 5: Run integration tests**

Create `.beads/test_created_dates.sh`:

```bash
#!/bin/bash

# Test 1: Create an issue
beads create --data '{"title": "Test issue"}'

# Test 2: List all issues
beads list

# Test 3: List issues created in past week
beads list --created-after "1 week ago"

# Test 4: List issues created in past 3 days
beads list --created-after "3 days ago"

# Test 5: List issues created between dates
beads list --created-after "2026-01-20" --created-before "2026-01-30"
```

Run tests manually: `bash .beads/test_created_dates.sh`
Expected: All commands execute without errors and show appropriate filtering

**Step 6: Commit**

```bash
git add crates/beads/src/main.rs crates/beads/src/commands/issue/list.rs
git commit -m "feat: add --created-after, --created-before, and --timezone filters to beads issue list"
```

---

## Phase 6: Documentation

### Task 6: Document datetime query feature

**Files:**
- Create: `docs/DATETIME_QUERIES.md`
- Modify: `README.md` (add reference to new feature)

**Step 1: Create feature documentation**

Create `docs/DATETIME_QUERIES.md`:

```markdown
# DateTime Query Support

The beads CLI supports filtering issues by creation date using relative or absolute date expressions. Timestamps are stored in UTC but automatically converted to your local timezone.

## Quick Start

```bash
# Issues created in the past week
beads issue list --created-after "1 week ago"

# Issues created in January
beads issue list --created-after "2026-01-01" --created-before "2026-02-01"

# Use your timezone
beads issue list --created-after "1 week ago" --timezone "America/New_York"
```

## Syntax

### Relative Dates (from now)

```bash
beads issue list --created-after "1 day ago"
beads issue list --created-after "2 weeks ago"
beads issue list --created-after "3 months ago"
```

Supported units: days, weeks, months, years

### Absolute Dates (ISO 8601 or YYYY-MM-DD)

```bash
beads issue list --created-after "2026-01-20"
beads issue list --created-after "2026-01-20T10:00:00Z"
```

### Combined Filters

```bash
beads issue list --created-after "2026-01-01" --created-before "2026-01-31"
```

## Timezone Support

Timestamps are stored in UTC internally, but queries respect your local timezone.

### Auto-Detection (priority order)

1. `--timezone` flag: Explicit override
2. `TZ` environment variable: System-wide setting
3. System timezone: Auto-detected from `/etc/timezone` (Linux)
4. Fallback: UTC

### Examples

```bash
# User in EST (America/New_York)
# "1 week ago" means 1 week from NOW (in your timezone)
beads issue list --created-after "1 week ago"

# Explicit timezone override
beads issue list --created-after "1 week ago" --timezone "Europe/London"

# Using environment variable
export TZ=Asia/Tokyo
beads issue list --created-after "3 days ago"
```

## Storage & Display

- **Internal:** UTC timestamps in ISO 8601 format (e.g., 2026-01-31T15:30:00Z)
- **JSON output:** Both UTC and local timezone shown
- **Database:** Stored in `created_at` column (TEXT, UTC)

## Automatic Migration

**No user action required.** When you upgrade and run `beads sync`:

1. Detects old database schema (no `created_at` column)
2. Reads event timestamps from `events.jsonl`
3. Populates `created_at` with first event timestamp for each issue
4. Updates schema version in metadata

Progress is shown:
```
Detected old database schema. Migrating timestamps from events.jsonl...
Applied 150 events
```

### Manual Rebuild

If you need to rebuild the cache for any reason:

```bash
beads sync --force
```

This rebuilds the entire SQLite cache from `events.jsonl` and recalculates all timestamps.
```

**Step 2: Update README.md**

In `README.md`, add a section linking to the new feature documentation:

```markdown
## Features

### DateTime Queries

Filter issues by creation date using relative or absolute dates:

```bash
beads issue list --created-after "1 week ago"
beads issue list --created-after "2026-01-20"
```

Supports timezone-aware queries with automatic system timezone detection. See [DateTime Queries](docs/DATETIME_QUERIES.md) for full documentation.
```

**Step 3: Commit**

```bash
git add docs/DATETIME_QUERIES.md README.md
git commit -m "docs: add datetime query feature documentation"
```

---

## Testing Checklist

**Unit Tests:**
- [ ] Unit tests for `tz.rs` timezone detection pass
- [ ] Unit tests for `tz.rs` timezone conversion pass
- [ ] Unit tests for `query.rs` pass
- [ ] Unit tests for `db.rs` pass

**CLI Integration Tests:**
- [ ] `beads issue list --created-after "1 week ago"` filters correctly (in system timezone)
- [ ] `beads issue list --created-before "2026-02-01"` filters correctly
- [ ] Combined `--created-after` and `--created-before` work together
- [ ] `beads issue list --created-after "1 week ago" --timezone "America/New_York"` uses specified timezone
- [ ] `TZ=America/New_York beads issue list --created-after "1 week ago"` uses TZ env var

**Date Parsing Tests:**
- [ ] Relative date parsing handles "days", "weeks", "months", "years"
- [ ] ISO 8601 dates parse correctly (with timezone)
- [ ] YYYY-MM-DD format parses correctly (assumes midnight local)
- [ ] Invalid dates show helpful error messages
- [ ] Timezone offset handling (e.g., "2026-01-20T15:30:00-05:00")

**Timezone Tests:**
- [ ] System timezone detection works on Linux (/etc/timezone)
- [ ] TZ environment variable is respected
- [ ] `--timezone` flag overrides system/env defaults
- [ ] Invalid timezone names produce helpful errors
- [ ] JSON output shows both UTC and local timestamps

**Data Migration:**
- [ ] Existing issues in events.jsonl get created_at from first event timestamp
- [ ] Updated issues maintain created_at from first event (not overwritten)

---

## Rollback Plan

If issues arise:

1. **Migration fails:** 
   - Check that events.jsonl is readable and not corrupted
   - Run `beads sync --force` to fully rebuild from events.jsonl
   - Check schema_version in `sqlite3 .beads/beads.db "SELECT * FROM _meta WHERE key='schema_version'"`

2. **Timestamp parsing failures:**
   - Implement fallback to NULL if event.ts is missing
   - Log warnings instead of failing entire sync
   - Mark issue for manual review

3. **Performance issues:**
   - Add index on `created_at` column in SQLite:
   ```sql
   CREATE INDEX IF NOT EXISTS idx_issues_created_at ON issues(created_at DESC);
   ```

4. **Timezone detection issues:**
   - Default to UTC if system timezone cannot be detected
   - Allow explicit override via `--timezone` flag
   - Print detected timezone to stderr for debugging

5. **Complete revert (if needed):**
   - Delete `created_at` column: `ALTER TABLE issues DROP COLUMN created_at;` (SQLite doesn't support this directly)
   - Instead, rebuild from scratch: `beads sync --force` after reverting code
   - Or manually: `rm .beads/beads.db` and run `beads sync`

---

**Plan complete and saved.** Two execution options:

1. **Subagent-Driven (this session)** - Fresh subagent per task, review between tasks, fast iteration
2. **Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach would you prefer?
