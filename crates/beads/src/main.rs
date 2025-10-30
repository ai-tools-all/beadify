use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

use beads_core::repo::find_repo;

mod commands;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new beads repository
    Init {
        /// Prefix for issue IDs (e.g. "proj")
        #[arg(long)]
        prefix: String,
    },
    /// Create a new issue
    Create {
        #[arg(short, long)]
        title: String,
        #[arg(long)]
        data: String,
        /// Issues this depends on (can be used multiple times)
        #[arg(long)]
        depends_on: Vec<String>,
    },
    /// Show issue details
    Show {
        id: String,
    },
    /// List all issues from the local cache
    List {
        /// Show all issues including closed (default: only open)
        #[arg(long)]
        all: bool,
        /// Filter by status (open, in_progress, closed, etc.)
        #[arg(long)]
        status: Option<String>,
    },
    /// Update an existing issue
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        priority: Option<u32>,
        #[arg(long)]
        status: Option<String>,
    },
    /// Apply new events from the log to the local database
    Sync {
        #[arg(long)]
        full: bool,
    },
    /// Search issues by text query with optional filters
    Search {
        /// Search query string
        query: String,
        /// Filter by issue kind (feature, task, bug, etc.)
        #[arg(long)]
        kind: Option<String>,
        /// Filter by status (open, in_progress, closed, etc.)
        #[arg(long)]
        status: Option<String>,
        /// Filter by priority level
        #[arg(long)]
        priority: Option<u32>,
        /// Search only in titles, not in descriptions
        #[arg(long)]
        title_only: bool,
    },
    /// Show the next issue to work on, grouped by priority
    Ready,
}

fn main() -> Result<()> {
    beads_tracing::init();

    let cli = Cli::parse();
    let repo = match cli.command {
        Commands::Init { .. } => None,
        _ => Some(find_repo()?),
    };

    match cli.command {
        Commands::Init { prefix } => {
            info!(command = "init", %prefix);
            commands::init::run(&prefix)?;
        }
        Commands::Create { title, data, depends_on } => {
            info!(command = "create", %title, deps = depends_on.len());
            commands::create::run(repo.unwrap(), &title, &data, depends_on)?;
        }
        Commands::Show { id } => {
            info!(command = "show", %id);
            commands::show::run(repo.unwrap(), &id)?;
        }
        Commands::List { all, status } => {
            info!(command = "list", all, status = status.as_deref());
            commands::list::run(repo.unwrap(), all, status)?;
        }
        Commands::Update {
            id,
            title,
            kind,
            priority,
            status,
        } => {
            info!(command = "update", %id, title = title.as_deref(), kind = kind.as_deref(), priority, status = status.as_deref());
            commands::update::run(repo.unwrap(), &id, title, kind, priority, status)?;
        }
        Commands::Sync { full } => {
            info!(command = "sync", full);
            commands::sync::run(repo.unwrap(), full)?;
        }
        Commands::Search {
            query,
            kind,
            status,
            priority,
            title_only,
        } => {
            info!(command = "search", %query, kind = kind.as_deref(), status = status.as_deref(), priority, title_only);
            commands::search::run(repo.unwrap(), &query, kind, status, priority, title_only)?;
        }
        Commands::Ready => {
            info!(command = "ready");
            commands::ready::run(repo.unwrap())?;
        }
    }

    Ok(())
}
