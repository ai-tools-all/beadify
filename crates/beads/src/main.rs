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
    Init,
    /// Create a new issue
    Create {
        #[arg(short, long)]
        title: String,
        #[arg(short, long, default_value = "task")]
        kind: String,
        #[arg(short, long, default_value_t = 2)]
        priority: u32,
    },
    /// List all issues from the local cache
    List,
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
}

fn main() -> Result<()> {
    beads_tracing::init();

    let cli = Cli::parse();
    let repo = match cli.command {
        Commands::Init => None,
        _ => Some(find_repo()?),
    };

    match cli.command {
        Commands::Init => {
            info!("command = init");
            commands::init::run()?;
        }
        Commands::Create {
            title,
            kind,
            priority,
        } => {
            info!(command = "create", %title, %kind, priority);
            commands::create::run(repo.unwrap(), &title, &kind, priority)?;
        }
        Commands::List => {
            info!(command = "list");
            commands::list::run(repo.unwrap())?;
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
    }

    Ok(())
}
