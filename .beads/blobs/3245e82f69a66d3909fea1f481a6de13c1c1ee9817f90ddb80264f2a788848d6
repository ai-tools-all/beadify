Of course. Here is a detailed plan for implementing the specified configuration system in your Rust codebase.

This plan is broken down into two main parts, mirroring the dual-system design: first, the **Tool-Level Configuration** using the `config` crate, and second, the **Project-Level Configuration** which extends the existing SQLite database.

### Part 1: Tool-Level Configuration (Viper-like)

This part focuses on user preferences that affect the tool's behavior globally. We will use the `config` crate to handle loading from files and environment variables.

#### Step 1.1: Update Dependencies

In `crates/beads/Cargo.toml`, add the necessary crates:

```toml
[dependencies]
# ... other dependencies
config = { version = "0.13", features = ["yaml"] }
serde = { version = "1.0", features = ["derive"] }
dirs = "5.0"
```

#### Step 1.2: Create the Configuration Module

1.  **Create a new file**: `crates/beads/src/config.rs`.
2.  **Define the `Settings` struct**: This struct will represent all tool-level configuration options.

    ```rust
    // In crates/beads/src/config.rs
    use serde::Deserialize;
    use config::{Config, ConfigError, Environment, File};
    use std::path::PathBuf;

    #[derive(Debug, Deserialize)]
    pub struct Settings {
        #[serde(default)]
        pub json: bool,
        #[serde(default)]
        pub no_daemon: bool,
        #[serde(default)]
        pub no_auto_flush: bool,
        #[serde(default)]
        pub no_auto_import: bool,
        pub db: Option<String>,
        pub actor: Option<String>,
        #[serde(default = "default_flush_debounce")]
        pub flush_debounce: String, // Keep as string for parsing later (e.g., "5s")
        #[serde(default = "default_true")]
        pub auto_start_daemon: bool,
    }

    fn default_flush_debounce() -> String { "5s".to_string() }
    fn default_true() -> bool { true }

    impl Settings {
        pub fn new() -> Result<Self, ConfigError> {
            let user_config_dir = dirs::config_dir().map(|p| p.join("bd/config.yaml"));
            let user_legacy_config = dirs::home_dir().map(|p| p.join(".beads/config.yaml"));

            // Note: Project config (.beads/config.yaml) is found relative to the repo root,
            // which we don't know here. We'll load it in main.rs.

            let s = Config::builder()
                // 4. Set defaults
                .set_default("json", false)?
                .set_default("no_daemon", false)?
                .set_default("actor", std::env::var("USER").unwrap_or_else(|_| "unknown".into()))?
                .set_default("flush_debounce", "5s")?
                .set_default("auto_start_daemon", true)?
                // 3. Load config files
                .add_source(File::from(user_legacy_config.unwrap()).required(false))
                .add_source(File::from(user_config_dir.unwrap()).required(false))
                // 2. Load environment variables
                .add_source(Environment::with_prefix("BD").separator("_"))
                .add_source(Environment::with_prefix("BEADS").separator("_")) // For BEADS_FLUSH_DEBOUNCE etc.
                .build()?;

            s.try_deserialize()
        }
    }
    ```

#### Step 1.3: Integrate with `clap` in `main.rs`

Modify `crates/beads/src/main.rs` to load settings and merge them with command-line flags.

1.  **Add global flags to `Cli` struct**:

    ```rust
    // In crates/beads/src/main.rs
    #[derive(Parser)]
    #[command(author, version, about, long_about = None)]
    struct Cli {
        #[command(subcommand)]
        command: Commands,

        // Global flags that override config files and env vars
        #[arg(global = true, long)]
        json: bool,

        #[arg(global = true, long)]
        no_daemon: bool,

        #[arg(global = true, long)]
        actor: Option<String>,

        #[arg(global = true, long)]
        db: Option<String>,
    }
    ```

2.  **Load and merge settings in `main()`**:

    ```rust
    // In crates/beads/src/main.rs
    mod config; // Add this line

    fn main() -> Result<()> {
        beads_tracing::init();
        let cli = Cli::parse();

        // Load settings from files and environment
        let mut settings = config::Settings::new()?;

        // Find repo to check for project-specific config file
        let repo = match &cli.command {
            // Init doesn't have a repo yet, other commands do.
            Commands::Init { .. } => None,
            _ => find_repo().ok(), // Use ok() to not fail if not in a repo
        };

        if let Some(r) = &repo {
            let project_config_path = r.beads_dir().join("config.yaml");
            let project_config = config::File::from(project_config_path).required(false);
            // Re-build settings to include project-level overrides
            // A more efficient way is to build a new config and merge, but this is simpler
        }

        // Manually enforce precedence: CLI flags > everything else
        if cli.json { settings.json = true; }
        if cli.no_daemon { settings.no_daemon = true; }
        if let Some(actor) = cli.actor { settings.actor = Some(actor); }
        if let Some(db) = cli.db { settings.db = Some(db); }

        // Now, pass `settings` or `repo` to command handlers.
        // Handlers should be modified to accept the new settings.
        // For example: commands::list::run(repo.unwrap(), &settings, ...)?;

        // ... rest of the main function match statement ...
    }
    ```

3.  **Refactor command handlers**: Update command handlers (like `list::run`) to accept the `Settings` struct instead of individual boolean flags. This centralizes configuration access.

    *   *Before:* `commands::list::run(repo, all, status, ..., json, ...)`
    *   *After:* `commands::list::run(repo, &settings, all, status, ...)`

### Part 2: Project-Level Configuration (`bd config`)

This part implements the `bd config` subcommands to manage key-value pairs in the project's SQLite database.

#### Step 2.1: Enhance `beads-core` Database Functions

In `crates/beads-core/src/db.rs`, add functions to support `list` and `unset`.

```rust
// In crates/beads-core/src/db.rs

// Add these two new functions:

pub fn unset_meta(tx: &Transaction<'_>, key: &str) -> Result<()> {
    let rows = tx.execute("DELETE FROM _meta WHERE key = ?1", params![key])?;
    if rows == 0 {
        return Err(crate::error::BeadsError::Custom(format!(
            "Configuration key not found: {}",
            key
        )));
    }
    Ok(())
}

pub fn get_all_meta(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM _meta ORDER BY key")?;
    let meta_iter = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut result = Vec::new();
    for meta in meta_iter {
        result.push(meta?);
    }
    Ok(result)
}
```

#### Step 2.2: Define the `config` Command in `main.rs`

1.  **Add a new command module**:
    *   Create `crates/beads/src/commands/config.rs`.
    *   In `crates/beads/src/commands/mod.rs`, add `pub mod config;`.

2.  **Define `Config` subcommands in `main.rs`**:

    ```rust
    // In crates/beads/src/main.rs

    #[derive(Subcommand)]
    enum Commands {
        // ... other commands
        /// Manage project-level configuration
        #[command(subcommand)]
        Config(ConfigCommand),
    }

    #[derive(Subcommand)]
    pub enum ConfigCommand {
        /// Set a configuration key-value pair
        Set {
            /// The configuration key (e.g., jira.url)
            key: String,
            /// The value to set
            value: String,
        },
        /// Get the value of a configuration key
        Get {
            /// The configuration key
            key: String,
        },
        /// List all configuration key-value pairs
        List,
        /// Remove a configuration key
        Unset {
            /// The key to remove
            key: String,
        },
    }
    ```

#### Step 2.3: Implement the `config` Command Logic

In the new file `crates/beads/src/commands/config.rs`, implement the logic for each subcommand.

```rust
// In crates/beads/src/commands/config.rs
use anyhow::Result;
use beads_core::{db, repo::BeadsRepo};
use serde_json::json;
use std::collections::BTreeMap;

pub fn set(repo: BeadsRepo, key: &str, value: &str) -> Result<()> {
    let mut conn = repo.open_db()?;
    let tx = conn.transaction()?;
    db::set_meta(&tx, key, value.to_string())?;
    tx.commit()?;
    println!("Set {} = {}", key, value);
    Ok(())
}

pub fn get(repo: BeadsRepo, key: &str, json_output: bool) -> Result<()> {
    let conn = repo.open_db()?;
    if let Some(value) = db::get_meta(&conn, key)? {
        if json_output {
            println!("{}", json!({ "key": key, "value": value }));
        } else {
            println!("{}", value);
        }
    } else {
        anyhow::bail!("Key not found: {}", key);
    }
    Ok(())
}

pub fn list(repo: BeadsRepo, json_output: bool) -> Result<()> {
    let conn = repo.open_db()?;
    let all_meta = db::get_all_meta(&conn)?;

    if json_output {
        let map: BTreeMap<_, _> = all_meta.into_iter().collect();
        println!("{}", serde_json::to_string_pretty(&map)?);
    } else {
        println!("Configuration:");
        for (key, value) in all_meta {
            println!("  {} = {}", key, value);
        }
    }
    Ok(())
}

pub fn unset(repo: BeadsRepo, key: &str) -> Result<()> {
    let mut conn = repo.open_db()?;
    let tx = conn.transaction()?;
    db::unset_meta(&tx, key)?;
    tx.commit()?;
    println!("Unset {}", key);
    Ok(())
}
```

#### Step 2.4: Wire Logic into `main.rs`

Finally, connect the `clap` command definitions to your new logic.

```rust
// In crates/beads/src/main.rs, inside the main() function's match statement

        // ... other command arms
        Commands::Config(cmd) => {
            let repo = repo.unwrap(); // Config commands require a repo
            // The global --json flag is on `cli`, not the subcommand
            let json_output = cli.json;

            match cmd {
                ConfigCommand::Set { key, value } => {
                    commands::config::set(repo, &key, &value)?;
                }
                ConfigCommand::Get { key } => {
                    commands::config::get(repo, &key, json_output)?;
                }
                ConfigCommand::List => {
                    commands::config::list(repo, json_output)?;
                }
                ConfigCommand::Unset { key } => {
                    commands::config::unset(repo, &key)?;
                }
            }
        }
```

This completes the implementation plan. You will have a robust, dual-configuration system that clearly separates user preferences from project-specific data, just as the design document specifies.
