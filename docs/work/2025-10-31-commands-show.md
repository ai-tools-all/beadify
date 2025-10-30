Of course. To close the loop for a functional MVP (ignoring the merge driver), the system must reliably sync changes between a remote and a local copy in a single-branch, non-conflicting workflow.

We need to do two things:
1.  **Fix a critical bug** in the full-resync logic that prevents it from working correctly.
2.  **Add the `show` command** to view a single issue, which is a core requirement for any issue tracker.

Here is the minimal path to a working MVP.

---

### Step 1: Fix the `clear_state` Bug (Critical)

Currently, when a full sync is triggered (`sync_repo(..., full: true)`), it calls `db::clear_state`. This function only deletes from the `issues` table, but it **forgets to clear the `_meta` table**.

This means after a full rebuild, the database still contains the *old* `last_processed_offset` and `last_event_id`, causing all future incremental syncs to fail or miss data. This breaks the core promise of the log being the source of truth.

**The Fix:** Update `db::clear_state` to also delete from `_meta`.

<crates/beads-core/src/db.rs>
```rust
// ... Lines 1-86 are unchanged ...

pub fn clear_state(tx: &Transaction<'_>) -> Result<()> {
    tx.execute("DELETE FROM issues", [])?;
    tx.execute("DELETE FROM _meta", [])?; // <-- ADD THIS LINE
    Ok(())
}

// ... rest of file is unchanged ...
```

With this one-line change, the `--full` sync and the automatic recovery mechanism in `apply_incremental` will now work reliably.

---

### Step 2: Implement the `show` Command (Essential Feature)

An MVP isn't complete without the ability to view the details of a specific issue. This requires adding a DB query and wiring up a new command.

#### 1. Add `get_issue` to `db.rs`

<crates/beads-core/src/db.rs>
```rust
// ... Add this new function to the file ...
pub fn get_issue(conn: &Connection, id: &str) -> Result<Option<Issue>> {
    let mut stmt = conn.prepare("SELECT id, title, kind, priority, status FROM issues WHERE id = ?1")?;
    let issue = stmt
        .query_row(params![id], |row| {
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: row.get(2)?,
                priority: row.get::<_, i64>(3)? as u32,
                status: row.get(4)?,
            })
        })
        .optional()?;
    Ok(issue)
}
```

#### 2. Expose `get_issue` in `lib.rs`

<crates/beads-core/src/lib.rs>
```rust
// ...
// On line 11, add `get_issue as db_get_issue`
use db::{apply_issue_update, create_schema, get_all_issues as db_get_all, get_issue as db_get_issue, set_meta, upsert_issue};

// ...

// Add this new public function
pub fn get_issue(repo: &BeadsRepo, id: &str) -> Result<Option<Issue>> {
    let conn = repo.open_db()?;
    create_schema(&conn)?;
    db_get_issue(&conn, id)
}
```

#### 3. Create the `show` command module

Create a new file: `crates/beads/src/commands/show.rs`

```rust
use anyhow::{anyhow, Result};
use beads_core::{get_issue, repo::BeadsRepo};

pub fn run(repo: BeadsRepo, id: &str) -> Result<()> {
    let issue = get_issue(&repo, id)?.ok_or_else(|| anyhow!("Issue '{}' not found", id))?;

    println!("ID:       {}", issue.id);
    println!("Title:    {}", issue.title);
    println!("Status:   {}", issue.status);
    println!("Kind:     {}", issue.kind);
    println!("Priority: {}", issue.priority);

    Ok(())
}
```

#### 4. Register the new command

<crates/beads/src/commands/mod.rs>
```rust
pub mod create;
pub mod init;
pub mod list;
pub mod show; // <-- ADD THIS
pub mod sync;
pub mod update;
```

<crates/beads/src/main.rs>
```rust
// ...

#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    List,
    /// Show details of a specific issue
    Show {
        id: String,
    },
    /// Update an existing issue
    Update {
    //...
}

//...

fn main() -> Result<()> {
    // ...
    match cli.command {
        // ...
        Commands::List => {
            info!(command = "list");
            commands::list::run(repo.unwrap())?;
        }
        Commands::Show { id } => {
            info!(command = "show", %id);
            commands::show::run(repo.unwrap(), &id)?;
        }
        Commands::Update {
        // ...
    }
    // ...
}
```

---

### What This Achieves

With these two changes, you have a complete, functional MVP loop for a **single user**:

1.  A user can `init` a repository.
2.  They can `create` and `update` issues, which are written locally instantly.
3.  They can `list` all issues and `show` details for one issue.
4.  They can run `beads sync` to `git push` their local `events.jsonl` changes.
5.  On another machine, they can `clone` the repo, run `beads sync` which will `git pull` and perform a full rebuild of the database from the log. All issues will appear correctly.
6.  Subsequent `beads sync` commands will correctly apply new events incrementally.

This closes the core loop. The next logical step would be to re-introduce the merge driver to handle concurrent writes and enable multi-user collaboration.