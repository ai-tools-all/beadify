Of course. Here is a concise code plan to implement the prefixed, incrementing ID strategy.

This plan focuses on modifying the `beads` CLI and the `beads-core` library to handle the new ID generation, while demonstrating that the rest of the architecture remains robust.

---

### Code Plan: Prefixed, Incrementing IDs

The goal is to change the issue ID from a ULID to a format like `proj-001`, where `proj` is a user-defined prefix and `001` is a monotonically increasing number for that repository.

This requires two main changes:
1.  Storing the ID prefix and the last-used ID counter within the repository's database.
2.  Updating the `init` command to accept a prefix and the `create` command's logic to generate the new ID.

#### Summary of Changes

*   **`beads` (CLI):** The `init` command will gain a `--prefix` argument.
*   **`beads-core`:**
    *   The `init_repo` function will now store the prefix and initialize an ID counter in the database.
    *   A new private function, `generate_next_issue_id`, will transactionally read the counter, increment it, and format the new ID.
    *   The `create_issue` function will use this new ID generation logic.
*   **Merge Driver:** The merge strategy remains sound because the full event line (which includes the unique ID) is what's being merged.

---

### Crate 1: `beads` (The CLI)

We only need to modify the `init` command to capture the prefix.

**`crates/beads/src/main.rs`**
```rust
// ... imports ...

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new beads repository
    Init {
        /// A short, unique prefix for issue IDs (e.g., "proj", "beads")
        #[arg(long)]
        prefix: String,
    },
    // ... other commands are unchanged ...
}

fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    let repo = if !matches!(cli.command, Commands::Init {..}) { // Note the pattern change
        Some(find_repo()?)
    } else {
        None
    };

    match &cli.command {
        Commands::Init { prefix } => commands::init::run(prefix)?, // Pass the prefix
        // ... other command arms are unchanged ...
    }

    Ok(())
}
```

**`crates/beads/src/commands/init.rs`**
```rust
use anyhow::Result;
use std::env;
use std::path::Path;

pub fn run(prefix: &str) -> Result<()> {
    let path = env::current_dir()?;
    // Delegate to the core function with the new prefix argument
    beads_core::init_repo(&path, prefix)?;
    println!("Initialized empty beads repository in {:?}", path.join(".beads"));
    println!("Issue ID prefix set to: '{}'", prefix);
    Ok(())
}
```

---

### Crate 2: `beads-core` (The Core Logic)

This is where the main logic changes reside. We'll use the `meta` table we designed earlier to store the configuration.

**`crates/beads-core/src/db.rs`**

Let's add a convenient helper to get values from our `meta` table.

```rust
// ... existing db functions ...

/// Gets a value from the meta table.
pub fn get_meta(conn: &rusqlite::Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
    let mut rows = stmt.query_map([key], |row| row.get(0))?;

    match rows.next() {
        Some(Ok(value)) => Ok(Some(value)),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}
```

**`crates/beads-core/src/lib.rs`**

Update `init_repo` and `create_issue` to handle the new ID scheme.

```rust
// ... re-exports ...

// --- Private Helper for ID Generation ---

/// Generates the next sequential issue ID within a transaction.
/// Reads the prefix and counter, increments the counter, and returns the new ID.
fn generate_next_issue_id(conn: &rusqlite::Connection) -> Result<String> {
    let tx = conn.transaction()?;

    // 1. Get the prefix
    let prefix = db::get_meta(&tx, "id_prefix")?
        .ok_or(BeadsError::Configuration("ID prefix not found".into()))?;

    // 2. Get the current counter, defaulting to 0 if not found
    let last_id_str = db::get_meta(&tx, "last_issue_id")?.unwrap_or_else(|| "0".to_string());
    let last_id: u32 = last_id_str.parse().unwrap_or(0);
    let next_id = last_id + 1;

    // 3. Save the new counter value
    db::set_meta(&tx, "last_issue_id", &next_id.to_string())?;

    tx.commit()?;

    // 4. Format the final ID (e.g., "proj-001")
    Ok(format!("{}-{:03}", prefix, next_id))
}


// --- Phase 1: High-Level API Functions (Updated) ---

/// Initializes a new beads repository in the given directory with a specific ID prefix.
pub fn init_repo(path: &std::path::Path, prefix: &str) -> Result<BeadsRepo> {
    let repo = BeadsRepo::new(path.to_path_buf());
    if repo.beads_dir.exists() {
        return Err(BeadsError::AlreadyInitialized);
    }
    std::fs::create_dir(&repo.beads_dir)?;
    std::fs::File::create(&repo.log_path)?;

    let conn = repo.open_db()?;
    db::create_schema(&conn)?;

    // Store the prefix and initialize the counter in a transaction
    let tx = conn.transaction()?;
    db::set_meta(&tx, "id_prefix", prefix)?;
    db::set_meta(&tx, "last_issue_id", "0")?;
    tx.commit()?;

    Ok(repo)
}

/// Creates a new issue, writing to the log and updating the cache.
pub fn create_issue(repo: &BeadsRepo, title: &str, kind: &str, priority: u32) -> Result<Event> {
    let conn = repo.open_db()?;
    let mut generator = log::get_monotonic_generator(&conn)?;

    // Use our new ID generation function
    let issue_id = generate_next_issue_id(&conn)?;

    let op = OpKind::Create;
    let data = EventData::Create {
        title: title.to_string(),
        kind: kind.to_string(),
        priority,
    };
    log::write_event(repo, &conn, &mut generator, op, issue_id, data)
}

// ... other functions like get_all_issues and run_full_sync are unchanged ...
```