# Issue Deletion Feature - Implementation Plan (v2)

## Overview

Implement soft deletion using status="deleted" in Update events. This approach keeps all data in the event log for audit/recovery while excluding deleted issues from the database and UI.

## Key Changes Based on Feedback

1. **Use Update OpKind** with status="deleted" (NOT a new Delete OpKind)
2. **Soft delete approach**: Issues remain in events.jsonl, excluded from SQLite
3. **Cascade deletion**: Append multiple Update events (one per issue) setting status="deleted"
4. **Event application**: Skip issues with status="deleted" during sync/init
5. **Future compaction**: Physical removal from events.jsonl can happen later

## User Requirements

### Command Syntax
```bash
# Single issue deletion (preview mode)
beads delete bd-001

# Force single deletion
beads delete bd-001 --force

# Batch deletion
beads delete bd-001 bd-002 bd-003 --force

# Delete from file (one ID per line)
beads delete --from-file deletions.txt --force

# Cascade deletion (recursively delete dependents)
beads delete bd-001 --cascade --force
```

## Architecture Analysis

### Current System Components

1. **Event Log** (`events.jsonl`)
   - Append-only event stream
   - Currently supports: Create, Update, Comment, Link, Unlink, Archive
   - **No new OpKind needed** - use Update with status="deleted"

2. **Database Tables**
   - `issues`: Main issue storage (id, title, kind, priority, status, data)
   - `dependencies`: Issue dependencies (with ON DELETE CASCADE)
   - `issue_labels`: Issue-label mappings (with ON DELETE CASCADE)
   - `labels`: Label definitions
   - `_meta`: Metadata storage

3. **Event Application**
   - During sync/init, events are replayed to build SQLite cache
   - **NEW**: Skip issues with status="deleted" - don't insert into SQLite

4. **Text References**
   - Issues can reference other issues in title, description, design, acceptance_criteria, notes
   - Common patterns: "bd-001", "Fixes bd-123", "Depends on bd-042"

## Implementation Plan

### Phase 1: Update Event Application Logic

#### 1.1 Skip Deleted Issues During Replay
**File:** `crates/beads-core/src/log.rs`

Update `apply_event()` function:
```rust
fn apply_event(tx: &Transaction<'_>, event: &Event) -> Result<()> {
    match event.op {
        OpKind::Create => {
            let data = &event.data;
            
            // Extract status from data, default to "open"
            let status = data.get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("open");
            
            // Skip if status is "deleted"
            if status == "deleted" {
                return Ok(());
            }
            
            let issue = Issue {
                id: event.id.clone(),
                title: data.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                kind: data.get("kind").and_then(|v| v.as_str()).unwrap_or("task").to_string(),
                priority: data.get("priority").and_then(|v| v.as_u64()).unwrap_or(2) as u32,
                status: status.to_string(),
                // ... other fields
            };
            db::upsert_issue(tx, &issue)
        }
        OpKind::Update => {
            #[derive(Deserialize)]
            struct UpdateData {
                #[serde(default)]
                status: Option<String>,
                // ... other fields
            }
            
            let update: UpdateData = serde_json::from_value(event.data.clone())?;
            
            // Check if this is a deletion (status="deleted")
            if let Some(ref status) = update.status {
                if status == "deleted" {
                    // Remove issue from database
                    db::delete_issue(tx, &event.id)?;
                    // Update text references
                    db::update_text_references(tx, &event.id)?;
                    return Ok(());
                }
            }
            
            // Normal update handling
            db::apply_issue_update(tx, &event.id, &update)
        }
        // ... other OpKind cases
    }
}
```

#### 1.2 Database Delete Function
**File:** `crates/beads-core/src/db.rs`

Add functions:
```rust
/// Delete issue and all related data from SQLite
/// Note: This is for removing from the local cache only
/// The issue still exists in events.jsonl
pub fn delete_issue(tx: &Transaction<'_>, issue_id: &str) -> Result<()> {
    // Dependencies and issue_labels automatically deleted by CASCADE
    tx.execute("DELETE FROM issues WHERE id = ?1", params![issue_id])?;
    Ok(())
}

/// Update text references to deleted issues
/// Replace "bd-001" with "[deleted:bd-001]" in all text fields
pub fn update_text_references(tx: &Transaction<'_>, deleted_id: &str) -> Result<usize> {
    let fields = ["title", "description", "design", "acceptance_criteria", "notes"];
    let replacement = format!("[deleted:{}]", deleted_id);
    let search_pattern = format!("%{}%", deleted_id);
    
    let mut total_updated = 0;
    
    for field in &fields {
        // Check if column exists (some might be NULL)
        let query = format!(
            "UPDATE issues SET {} = REPLACE({}, ?1, ?2) WHERE {} IS NOT NULL AND {} LIKE ?3",
            field, field, field, field
        );
        
        let updated = tx.execute(&query, params![deleted_id, &replacement, &search_pattern])?;
        total_updated += updated;
    }
    
    Ok(total_updated)
}

/// Check if an issue is deleted (by checking events.jsonl)
/// Used for validation before operations
pub fn is_issue_deleted(conn: &Connection, issue_id: &str) -> Result<bool> {
    // If issue doesn't exist in database, it might be deleted
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM issues WHERE id = ?1",
            params![issue_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    
    Ok(!exists)
}
```

### Phase 2: Core Library API

**File:** `crates/beads-core/src/lib.rs`

Add public functions:
```rust
/// Delete a single issue by setting status="deleted"
pub fn delete_issue(repo: &BeadsRepo, issue_id: &str) -> Result<DeleteResult> {
    let mut conn = repo.open_db()?;
    create_schema(&conn)?;

    // Check if issue exists and is not already deleted
    let issue = get_issue(repo, issue_id)?.ok_or_else(|| 
        BeadsError::Custom(format!("Issue '{}' not found or already deleted", issue_id))
    )?;

    // Get dependents (issues that depend on this)
    let dependents = get_dependents(repo, issue_id)?;

    // Create update event with status="deleted"
    let update = IssueUpdate {
        status: Some("deleted".to_string()),
        ..Default::default()
    };
    
    let (event, new_offset) = log::append_update_event(repo, &conn, issue_id, &update)?;

    // Apply deletion in transaction
    let tx = conn.transaction()?;
    db::delete_issue(&tx, issue_id)?;
    let refs_updated = db::update_text_references(&tx, issue_id)?;
    set_meta(&tx, "last_event_id", event.event_id.clone())?;
    set_meta(&tx, "last_processed_offset", new_offset.to_string())?;
    tx.commit()?;

    Ok(DeleteResult {
        issue_id: issue_id.to_string(),
        title: issue.title,
        dependents,
        references_updated: refs_updated,
    })
}

/// Get issues that would be affected by deletion (for preview)
pub fn get_delete_impact(repo: &BeadsRepo, issue_id: &str, cascade: bool) -> Result<DeleteImpact> {
    let issue = get_issue(repo, issue_id)?.ok_or_else(|| 
        BeadsError::Custom(format!("Issue '{}' not found or already deleted", issue_id))
    )?;

    let dependents = get_dependents(repo, issue_id)?;
    let text_refs = find_text_references(repo, issue_id)?;

    let mut all_issues = vec![ImpactItem {
        id: issue_id.to_string(),
        title: issue.title.clone(),
    }];

    if cascade {
        let recursive_deps = get_all_dependents_recursive(repo, issue_id)?;
        all_issues.extend(recursive_deps);
    }

    Ok(DeleteImpact {
        issues_to_delete: all_issues,
        blocked_issues: dependents,
        text_references: text_refs,
    })
}

/// Cascade delete: recursively delete all dependents
/// Each issue gets its own Update event with status="deleted"
pub fn delete_issue_cascade(repo: &BeadsRepo, issue_id: &str) -> Result<Vec<DeleteResult>> {
    let mut results = Vec::new();

    // Get all dependents recursively (topologically sorted)
    let all_dependents = get_all_dependents_recursive_sorted(repo, issue_id)?;

    // Delete in reverse dependency order (leaves first)
    // Each deletion creates a separate Update event
    for dependent in all_dependents.iter().rev() {
        match delete_issue(repo, &dependent.id) {
            Ok(result) => results.push(result),
            Err(e) => {
                eprintln!("Warning: Failed to delete {}: {}", dependent.id, e);
                // Continue with other deletions
            }
        }
    }

    // Finally delete the root issue
    results.push(delete_issue(repo, issue_id)?);

    Ok(results)
}

/// Batch delete multiple issues
/// Each issue gets its own Update event with status="deleted"
pub fn delete_issues_batch(
    repo: &BeadsRepo,
    issue_ids: Vec<String>,
    cascade: bool,
) -> Result<BatchDeleteResult> {
    let mut successes = Vec::new();
    let mut failures = Vec::new();

    for issue_id in issue_ids {
        let result = if cascade {
            delete_issue_cascade(repo, &issue_id)
                .map(|results| results.into_iter().map(|r| r.issue_id).collect())
        } else {
            delete_issue(repo, &issue_id)
                .map(|r| vec![r.issue_id])
        };

        match result {
            Ok(deleted_ids) => successes.extend(deleted_ids),
            Err(e) => {
                failures.push(DeleteFailure {
                    issue_id: issue_id.clone(),
                    error: e.to_string(),
                });
            }
        }
    }

    Ok(BatchDeleteResult {
        successes,
        failures,
    })
}

/// Find all text references to an issue (for preview)
fn find_text_references(repo: &BeadsRepo, issue_id: &str) -> Result<Vec<String>> {
    let conn = repo.open_db()?;
    let pattern = format!("%{}%", issue_id);
    
    let mut stmt = conn.prepare(
        r#"
        SELECT id FROM issues 
        WHERE title LIKE ?1 
           OR description LIKE ?1 
           OR design LIKE ?1 
           OR acceptance_criteria LIKE ?1 
           OR notes LIKE ?1
        "#
    )?;
    
    let refs = stmt
        .query_map(params![pattern], |row| row.get(0))?
        .collect::<std::result::Result<Vec<String>, _>>()?;
    
    Ok(refs)
}

/// Get all dependents recursively, topologically sorted
fn get_all_dependents_recursive_sorted(repo: &BeadsRepo, issue_id: &str) -> Result<Vec<ImpactItem>> {
    let mut visited = std::collections::HashSet::new();
    let mut result = Vec::new();
    
    fn visit(
        repo: &BeadsRepo,
        id: &str,
        visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<ImpactItem>,
    ) -> Result<()> {
        if visited.contains(id) {
            return Ok(()); // Already processed or cycle detected
        }
        visited.insert(id.to_string());
        
        let dependents = get_dependents(repo, id)?;
        for dep_id in dependents {
            visit(repo, &dep_id, visited, result)?;
        }
        
        // Add after processing dependents (post-order for deletion)
        if let Some(issue) = get_issue(repo, id)? {
            result.push(ImpactItem {
                id: id.to_string(),
                title: issue.title,
            });
        }
        
        Ok(())
    }
    
    let dependents = get_dependents(repo, issue_id)?;
    for dep_id in dependents {
        visit(repo, &dep_id, &mut visited, &mut result)?;
    }
    
    Ok(result)
}
```

Supporting types:
```rust
#[derive(Debug, Clone)]
pub struct DeleteResult {
    pub issue_id: String,
    pub title: String,
    pub dependents: Vec<String>,
    pub references_updated: usize,
}

#[derive(Debug, Clone)]
pub struct ImpactItem {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct DeleteImpact {
    pub issues_to_delete: Vec<ImpactItem>,
    pub blocked_issues: Vec<String>,
    pub text_references: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteFailure {
    pub issue_id: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct BatchDeleteResult {
    pub successes: Vec<String>,
    pub failures: Vec<DeleteFailure>,
}
```

### Phase 3: CLI Command

**File:** `crates/beads/src/commands/delete.rs` (new)

```rust
use std::fs;
use std::io::{self, Write};

use anyhow::{Context, Result};
use beads_core::{
    delete_issue, delete_issue_cascade, delete_issues_batch, get_delete_impact,
    repo::BeadsRepo,
};

pub fn run(
    repo: BeadsRepo,
    issue_ids: Vec<String>,
    force: bool,
    cascade: bool,
    from_file: Option<String>,
) -> Result<()> {
    // Collect all issue IDs
    let mut all_ids = issue_ids;

    if let Some(file_path) = from_file {
        let content = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read file: {}", file_path))?;

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                all_ids.push(trimmed.to_string());
            }
        }
    }

    if all_ids.is_empty() {
        anyhow::bail!("No issue IDs provided");
    }

    // Preview mode: show what would be deleted
    if !force {
        println!("Preview mode - the following would be deleted:");
        println!("(Issues will be marked as deleted in events.jsonl)");
        println!();

        let mut total_issues = 0;
        let mut total_blocked = 0;
        let mut total_refs = 0;

        for issue_id in &all_ids {
            match get_delete_impact(&repo, issue_id, cascade) {
                Ok(impact) => {
                    println!("Issue: {}", issue_id);
                    
                    if cascade && impact.issues_to_delete.len() > 1 {
                        println!("  Would delete {} issue(s) (cascade):", impact.issues_to_delete.len());
                        for item in &impact.issues_to_delete {
                            println!("    - {} - {}", item.id, item.title);
                        }
                    } else {
                        println!("  Would delete: {} - {}", 
                            impact.issues_to_delete[0].id,
                            impact.issues_to_delete[0].title);
                    }
                    
                    if !impact.blocked_issues.is_empty() {
                        println!("  ⚠️  {} issue(s) depend on this:", impact.blocked_issues.len());
                        for blocked in &impact.blocked_issues {
                            println!("    - {}", blocked);
                        }
                    }
                    
                    if !impact.text_references.is_empty() {
                        println!("  📝 {} issue(s) reference this in text", impact.text_references.len());
                    }
                    
                    total_issues += impact.issues_to_delete.len();
                    total_blocked += impact.blocked_issues.len();
                    total_refs += impact.text_references.len();
                    
                    println!();
                }
                Err(e) => {
                    eprintln!("✗ Error analyzing {}: {}", issue_id, e);
                    println!();
                }
            }
        }

        println!("Summary:");
        println!("  Total issues to delete: {}", total_issues);
        if total_blocked > 0 {
            println!("  ⚠️  Issues with dependents: {}", total_blocked);
        }
        if total_refs > 0 {
            println!("  📝 Text references to update: {}", total_refs);
        }
        println!();
        println!("Run with --force to confirm deletion");
        return Ok(());
    }

    // Confirm before cascade deletion
    if cascade && !all_ids.is_empty() {
        print!("⚠️  Cascade deletion will delete dependents recursively. Continue? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    // Execute deletions
    let result = delete_issues_batch(&repo, all_ids, cascade)?;

    // Report results
    if !result.successes.is_empty() {
        println!("✓ Successfully deleted {} issue(s):", result.successes.len());
        for issue_id in &result.successes {
            println!("  - {}", issue_id);
        }
    }

    if !result.failures.is_empty() {
        println!();
        println!("✗ Failed to delete {} issue(s):", result.failures.len());
        for failure in &result.failures {
            println!("  - {}: {}", failure.issue_id, failure.error);
        }
    }

    Ok(())
}
```

**File:** `crates/beads/src/commands/mod.rs`

Add:
```rust
pub mod delete;
```

**File:** `crates/beads/src/main.rs`

Add to Commands enum:
```rust
/// Delete one or more issues (soft delete with status="deleted")
Delete {
    /// Issue IDs to delete
    issue_ids: Vec<String>,
    /// Confirm deletion without preview
    #[arg(long)]
    force: bool,
    /// Recursively delete dependent issues
    #[arg(long)]
    cascade: bool,
    /// Read issue IDs from file (one per line)
    #[arg(long)]
    from_file: Option<String>,
},
```

Add to match statement:
```rust
Commands::Delete { issue_ids, force, cascade, from_file } => {
    info!(command = "delete", ids = issue_ids.len(), force, cascade, from_file = from_file.as_deref());
    commands::delete::run(repo.unwrap(), issue_ids, force, cascade, from_file)?;
}
```

## Key Benefits of Soft Delete Approach

### 1. **Audit Trail**
- All deletions recorded as Update events in events.jsonl
- Full history preserved: who deleted what and when
- Can analyze deletion patterns

### 2. **Recovery**
- Issues not physically removed from events.jsonl
- Future `beads undelete` command could restore issues
- Replay events.jsonl with filter to recover state

### 3. **Compaction Later**
- Physical removal deferred to compaction phase
- Compaction can have policies: "keep deleted issues for 90 days"
- Clean separation of concerns

### 4. **Simpler Implementation**
- Reuse existing Update OpKind and event handling
- No new event types to handle in merge conflicts
- Less code, less complexity

### 5. **Backward Compatible**
- Old code sees deleted issues as status="deleted"
- No breaking changes to event log format
- Graceful degradation

## Edge Cases and Considerations

### 1. Document Cleanup
**Approach:** Keep blobs - same as before
- Blobs are content-addressable and deduplicated
- Multiple issues might reference the same blob
- Future `beads gc` command can clean orphaned blobs

### 2. Dependency Handling
**Cascade deletion:**
- Get all dependents recursively (topological sort)
- Create one Update event per issue (in correct order)
- Each dependent gets status="deleted"
- Dependencies automatically removed by SQLite CASCADE

**Non-cascade deletion:**
- Warn user if issue has dependents
- Dependencies remain in place (orphaned)
- Could add --allow-orphan flag to suppress warning

### 3. Label Cleanup
**Approach:** Keep labels
- Labels are reusable definitions
- issue_labels mappings removed by SQLite CASCADE
- Future `beads label prune` can clean unused labels

### 4. Text Reference Updates
**Approach:** Replace references with [deleted:ID]
- Scan all text fields in remaining issues
- Use SQL REPLACE: `bd-001` → `[deleted:bd-001]`
- Happens during event application
- Provides clear indication in UI

### 5. Merge Conflicts
**Scenario:** Two people delete the same issue
- Both append Update events with status="deleted"
- Merge driver handles append-only log
- Both events recorded
- Second delete is idempotent (issue already not in SQLite)

### 6. Double Delete
**Scenario:** User tries to delete already deleted issue
- `get_issue()` returns None (not in SQLite)
- Error: "Issue not found or already deleted"
- No event created
- Idempotent operation

### 7. Filtering Deleted Issues
**During event replay:**
- Create events with status="deleted": Skip (don't insert to SQLite)
- Update events setting status="deleted": Remove from SQLite
- All list/show commands automatically exclude deleted issues

## Testing Strategy

### Unit Tests
1. Event application: Skip Create with status="deleted"
2. Event application: Remove on Update with status="deleted"
3. `delete_issue()` - creates Update event with status="deleted"
4. `delete_issue_cascade()` - creates multiple Update events
5. `update_text_references()` - replaces references correctly
6. `find_text_references()` - finds all occurrences
7. Topological sort for cascade deletion
8. Idempotency: delete already deleted issue

### Integration Tests
1. Delete issue and verify removed from SQLite
2. Delete issue with dependencies (should work)
3. Delete issue with dependents (should warn)
4. Cascade delete with multiple levels
5. Batch delete from file
6. Preview mode shows correct impact
7. Text references updated in remaining issues
8. Replay events.jsonl with deleted issues (should be excluded)
9. Sync with deleted issues from remote

### Edge Case Tests
1. Delete non-existent issue
2. Delete already deleted issue (idempotent)
3. Cascade delete with circular dependencies
4. Delete issue with documents attached (blobs remain)
5. Delete issue with labels (mappings removed)
6. Concurrent deletion in merge scenario

## Implementation Checklist

### Phase 1: Core Logic
- [ ] Update `apply_event()` to skip Create with status="deleted"
- [ ] Update `apply_event()` to handle Update with status="deleted"
- [ ] Add `delete_issue()` to db.rs (remove from SQLite)
- [ ] Add `update_text_references()` to db.rs
- [ ] Add `is_issue_deleted()` to db.rs
- [ ] Add `find_text_references()` to lib.rs

### Phase 2: Library API
- [ ] Implement `delete_issue()` in lib.rs
- [ ] Implement `get_delete_impact()` in lib.rs
- [ ] Implement `delete_issue_cascade()` in lib.rs
- [ ] Implement `delete_issues_batch()` in lib.rs
- [ ] Implement `get_all_dependents_recursive_sorted()` in lib.rs
- [ ] Add DeleteResult, DeleteImpact, etc. types

### Phase 3: CLI
- [ ] Create delete.rs command with preview mode
- [ ] Add Delete command to main.rs
- [ ] Implement --force flag
- [ ] Implement --cascade flag with confirmation
- [ ] Implement --from-file flag
- [ ] Add proper error messages and feedback

### Testing & Documentation
- [ ] Write unit tests for core logic
- [ ] Write integration tests
- [ ] Test edge cases
- [ ] Update README.md with delete examples
- [ ] Test preview mode output
- [ ] Test batch deletion workflow
- [ ] Verify sync behavior with deleted issues
- [ ] Test merge conflict scenarios

## Security and Safety

1. **Preview by default:** Require --force for actual deletion
2. **Cascade confirmation:** Extra prompt for cascade deletions
3. **Clear feedback:** Show exactly what will be deleted
4. **Error handling:** Continue batch deletion on individual failures
5. **Audit trail:** All deletions recorded in events.jsonl
6. **No data loss:** Issues remain in events.jsonl, blobs preserved
7. **Idempotent:** Deleting already deleted issue is safe
8. **Text references:** Clearly marked as [deleted:ID]

## Future Enhancements

1. **Undelete command:** Restore deleted issues
   ```bash
   beads undelete bd-001
   # Creates Update event setting status back to "open"
   ```

2. **Compaction:** Physical removal from events.jsonl
   ```bash
   beads compact --remove-deleted --older-than 90d
   # Rewrites events.jsonl without deleted issues
   ```

3. **Garbage collection:** Clean up orphaned blobs
   ```bash
   beads gc --dry-run
   beads gc --force
   ```

4. **Label pruning:** Remove unused labels
   ```bash
   beads label prune --unused
   ```

5. **List deleted issues:** See what was deleted
   ```bash
   beads list --deleted
   # Scans events.jsonl for status="deleted"
   ```

6. **Delete with reason:** Add metadata
   ```bash
   beads delete bd-001 --reason "Duplicate of bd-042" --force
   # Store reason in Update event data field
   ```

7. **Interactive mode:** Select issues to delete
   ```bash
   beads delete --interactive
   # Shows list, user selects with checkboxes
   ```

8. **JSON output:** For programmatic use
   ```bash
   beads delete bd-001 --json
   # Returns structured impact analysis
   ```

## Migration from Hard Delete (If Needed)

If we later want to support hard delete (actual Delete OpKind):

1. Add Delete OpKind to model.rs
2. Keep soft delete as default
3. Add `--hard` flag for physical deletion
4. Hard delete creates Delete event instead of Update
5. Compaction becomes no-op for hard-deleted issues

For now, soft delete with status="deleted" provides all needed functionality with better safety and audit trail.
