Excellent. This is a crucial feature that moves `beads` from a simple logger to a real task management system. The plan you've provided is solid, especially the SQL query, which is the heart of the logic.

Let's integrate this `ready` command into our existing four-crate workspace structure. This will involve significant additions to `beads-core` (new models, database schema changes, and the query logic) and a corresponding new command in the `beads` CLI crate.

---

### Improved Plan: Implementing the `ready` Command

This plan introduces the concepts of **Status** and **Dependencies** to our system. An issue's "readiness" is determined by its status and the status of the issues that block it.

#### Phase 1: Enhance `beads-core` (The Foundation)

We must first teach the core library about statuses and dependencies.

**1. `crates/beads-core/src/model.rs` (Introduce New Concepts)**

We need to add `Status` and `Dependency` types and update the `Issue` and `EventData` models.

```rust
// In model.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Status {
    Open,
    InProgress,
    Blocked,
    Closed,
}

impl Default for Status {
    fn default() -> Self { Status::Open }
}

impl Status {
    // Helper for converting to string for DB storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::InProgress => "in_progress",
            Status::Blocked => "blocked",
            Status::Closed => "closed",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum DependencyKind {
    Blocks,
    Related,
}

// The main Issue struct, now with status and assignee
#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub priority: u32,
    #[serde(default)] // New issues will default to Open
    pub status: Status,
    pub assignee: Option<String>,
    pub created_at: String, // Keep as ISO 8601 string
}

// Update EventData to handle status changes and dependency creation
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventData {
    Create {
        title: String,
        kind: String,
        priority: u32,
    },
    // New event types
    UpdateStatus {
        status: Status,
    },
    UpdateAssignee {
        assignee: Option<String>,
    },
    AddDependency {
        depends_on_id: String,
        kind: DependencyKind,
    },
}
```

**2. `crates/beads-core/src/db.rs` (Update Schema and Add Query)**

The database needs to store this new information.

```rust
// In db.rs

// Update create_schema to include the new columns and a dependencies table
pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;
        CREATE TABLE IF NOT EXISTS issues (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            kind TEXT NOT NULL,
            priority INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'open', -- New column
            assignee TEXT,                        -- New column
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS dependencies (
            issue_id TEXT NOT NULL,
            depends_on_id TEXT NOT NULL,
            kind TEXT NOT NULL, -- 'blocks', 'related'
            PRIMARY KEY (issue_id, depends_on_id),
            FOREIGN KEY (issue_id) REFERENCES issues(id),
            FOREIGN KEY (depends_on_id) REFERENCES issues(id)
        );
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        COMMIT;"
    )?;
    Ok(())
}

// ... other db functions ...

// The core query logic for the 'ready' command
pub fn get_ready_issues(conn: &Connection, filter: &WorkFilter) -> Result<Vec<Issue>> {
    // Base query from your plan
    let mut sql = String::from("
        SELECT id, title, kind, priority, status, assignee, created_at
        FROM issues i
        WHERE i.status = 'open'
          AND NOT EXISTS (
            SELECT 1 FROM dependencies d
            JOIN issues blocked ON d.depends_on_id = blocked.id
            WHERE d.issue_id = i.id
              AND d.kind = 'blocks'
              AND blocked.status IN ('open', 'in_progress', 'blocked')
          )
    ");

    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    // Dynamically add filters
    if let Some(priority) = filter.priority {
        sql.push_str(" AND i.priority = ?");
        params.push(Box::new(priority));
    }
    if let Some(ref assignee) = filter.assignee {
        sql.push_str(" AND i.assignee = ?");
        params.push(Box::new(assignee.clone()));
    }

    sql.push_str(" ORDER BY i.priority ASC, i.created_at DESC");

    if let Some(limit) = filter.limit {
        sql.push_str(" LIMIT ?");
        params.push(Box::new(limit));
    }

    let mut stmt = conn.prepare(&sql)?;
    let issue_iter = stmt.query_map(&params[..], |row| {
        // ... logic to map row to Issue struct ...
        Ok(Issue { /* ... */ })
    })?;

    let mut issues = Vec::new();
    for issue in issue_iter {
        issues.push(issue?);
    }
    Ok(issues)
}
```

**3. `crates/beads-core/src/lib.rs` (Expose the New API)**

Create the `WorkFilter` struct and the public function for the CLI to call.

```rust
// In lib.rs

// Re-export new models
pub use model::{Status, DependencyKind};

// Public struct for filtering, can be constructed by the CLI
#[derive(Debug, Default)]
pub struct WorkFilter {
    pub limit: Option<u32>,
    pub priority: Option<u32>,
    pub assignee: Option<String>,
}

// The high-level API function
pub fn get_ready_issues(repo: &BeadsRepo, filter: WorkFilter) -> Result<Vec<Issue>> {
    let conn = repo.open_db()?;
    db::get_ready_issues(&conn, &filter)
}

// We also need a way to create dependencies. This would be for a future `bd link` command.
pub fn add_dependency(repo: &BeadsRepo, issue_id: &str, depends_on_id: &str, kind: DependencyKind) -> Result<Event> {
    let conn = repo.open_db()?;
    let mut generator = log::get_monotonic_generator(&conn)?;
    let op = OpKind::Update; // This is an update to an existing issue
    let data = EventData::AddDependency {
        depends_on_id: depends_on_id.to_string(),
        kind,
    };
    log::write_event(repo, &conn, &mut generator, op, issue_id.to_string(), data)
}
```

---

#### Phase 2: Implement `beads` (The CLI)

Now, we'll create the user-facing command.

**1. `crates/beads/src/main.rs` (Define the Command)**

Add `Ready` to the `Commands` enum with all its flags.

```rust
// In main.rs

#[derive(Subcommand)]
enum Commands {
    // ... other commands ...
    /// Find issues that are ready for work (not blocked)
    Ready {
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        priority: Option<u32>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long, help = "Output in JSON format")]
        json: bool,
    },
    // We should also add the command to create dependencies
    /// Link issues together with dependencies
    Link {
        /// The ID of the issue that has a dependency
        issue_id: String,
        /// The ID of the issue it depends on
        #[arg(long)]
        blocks: String,
    }
}

fn main() -> Result<()> {
    // ...
    match &cli.command {
        // ... other command arms ...
        Commands::Ready { limit, priority, assignee, json } => {
            commands::ready::run(repo.unwrap(), *limit, *priority, assignee.clone(), *json)?
        }
        Commands::Link { issue_id, blocks } => {
            commands::link::run(repo.unwrap(), issue_id, blocks)?
        }
    }
    // ...
}
```

**2. `crates/beads/src/commands/ready.rs` (New Command File)**

This file will construct the filter, call the core library, and format the output.

```rust
// In crates/beads/src/commands/ready.rs

use anyhow::Result;
use beads_core::{BeadsRepo, WorkFilter};

pub fn run(
    repo: BeadsRepo,
    limit: Option<u32>,
    priority: Option<u32>,
    assignee: Option<String>,
    json: bool,
) -> Result<()> {
    // 1. Build the filter from CLI arguments
    let filter = WorkFilter {
        limit,
        priority,
        assignee,
    };

    // 2. Delegate to the core library
    let ready_issues = beads_core::get_ready_issues(&repo, filter)?;

    if ready_issues.is_empty() {
        println!("No ready work found.");
        return Ok(());
    }

    // 3. Format the output based on the --json flag
    if json {
        let json_output = serde_json::to_string_pretty(&ready_issues)?;
        println!("{}", json_output);
    } else {
        println!("Ready Work (Priority | ID | Title)");
        println!("------------------------------------");
        for issue in ready_issues {
            println!(
                "P{} | {} | {}",
                issue.priority,
                issue.id,
                issue.title
            );
        }
    }

    Ok(())
}
```

**(A similar thin wrapper, `link.rs`, would be created to call `beads_core::add_dependency`.)**