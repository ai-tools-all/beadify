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
        /// Labels to add to the issue (comma-separated)
        #[arg(short, long)]
        label: Option<String>,
        /// Attach documents in format "name:path" (can be used multiple times)
        #[arg(long)]
        doc: Vec<String>,
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
        /// Show flat list instead of tree hierarchy
        #[arg(long)]
        flat: bool,
        /// Filter by labels (AND - must have ALL specified labels)
        #[arg(long)]
        label: Option<String>,
        /// Filter by labels (OR - must have AT LEAST ONE specified label)
        #[arg(long)]
        label_any: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Show labels column in table view
        #[arg(long)]
        labels: bool,
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
        #[arg(long)]
        data: Option<String>,
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
    /// Manage issue dependencies
    #[command(subcommand)]
    Dep(DepCommand),
    /// Manage issue labels
    #[command(subcommand)]
    Label(LabelCommand),
    /// Manage issue documents
    #[command(subcommand)]
    Doc(DocCommand),
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
}

#[derive(Subcommand)]
enum DepCommand {
    /// Show dependencies and dependents for an issue
    Show {
        /// The issue ID
        issue_id: String,
    },
    /// Add a dependency: this issue depends on another
    Add {
        /// The issue that depends on another
        issue_id: String,
        /// The issue it depends on (blocker)
        depends_on_id: String,
    },
    /// Remove a dependency
    Remove {
        /// The issue with the dependency
        issue_id: String,
        /// The dependency to remove
        depends_on_id: String,
    },
}

#[derive(Subcommand)]
enum LabelCommand {
    /// Add a label to an issue
    Add {
        /// The issue to label
        issue_id: String,
        /// The label name
        label_name: String,
    },
    /// Remove a label from an issue
    Remove {
        /// The issue to unlabel
        issue_id: String,
        /// The label name
        label_name: String,
    },
    /// List labels on an issue
    List {
        /// The issue ID
        issue_id: String,
    },
    /// List all labels in the database
    #[command(name = "list-all")]
    ListAll,
}

#[derive(Subcommand)]
enum DocCommand {
    /// Add a document to an issue from a file
    Add {
        /// The issue to attach the document to
        issue_id: String,
        /// Path to the file to attach
        file_path: String,
    },
    /// Export a document to the workspace for editing
    Edit {
        /// The issue containing the document
        issue_id: String,
        /// Name of the document to edit
        doc_name: String,
    },
    /// Sync changes from workspace back to blob store
    Sync {
        /// The issue containing the document
        issue_id: String,
        /// Name of the document to sync
        doc_name: String,
    },
    /// List all documents attached to an issue
    List {
        /// The issue ID
        issue_id: String,
    },
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
        Commands::Create { title, data, depends_on, label, doc } => {
            info!(command = "create", %title, deps = depends_on.len(), label = label.as_deref(), docs = doc.len());
            commands::create::run(repo.unwrap(), &title, &data, depends_on, label, doc)?;
        }
        Commands::Show { id } => {
            info!(command = "show", %id);
            commands::show::run(repo.unwrap(), &id)?;
        }
        Commands::List {
            all,
            status,
            flat,
            label,
            label_any,
            json,
            labels,
        } => {
            info!(command = "list", all, status = status.as_deref(), flat, label = label.as_deref(), label_any = label_any.as_deref(), json, labels);
            commands::list::run(repo.unwrap(), all, status, flat, label, label_any, json, labels)?;
        }
        Commands::Update {
            id,
            title,
            kind,
            priority,
            status,
            data,
        } => {
            info!(command = "update", %id, title = title.as_deref(), kind = kind.as_deref(), priority, status = status.as_deref(), data = data.as_deref());
            commands::update::run(repo.unwrap(), &id, title, kind, priority, status, data)?;
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
        Commands::Dep(dep_cmd) => match dep_cmd {
            DepCommand::Show { issue_id } => {
                info!(command = "dep show", %issue_id);
                commands::dep::show(repo.unwrap(), &issue_id)?;
            }
            DepCommand::Add {
                issue_id,
                depends_on_id,
            } => {
                info!(command = "dep add", %issue_id, %depends_on_id);
                commands::dep::add(repo.unwrap(), &issue_id, &depends_on_id)?;
            }
            DepCommand::Remove {
                issue_id,
                depends_on_id,
            } => {
                info!(command = "dep remove", %issue_id, %depends_on_id);
                commands::dep::remove(repo.unwrap(), &issue_id, &depends_on_id)?;
            }
        },
        Commands::Label(label_cmd) => match label_cmd {
            LabelCommand::Add { issue_id, label_name } => {
                info!(command = "label add", %issue_id, %label_name);
                commands::label::add(repo.unwrap(), &issue_id, &label_name)?;
            }
            LabelCommand::Remove { issue_id, label_name } => {
                info!(command = "label remove", %issue_id, %label_name);
                commands::label::remove(repo.unwrap(), &issue_id, &label_name)?;
            }
            LabelCommand::List { issue_id } => {
                info!(command = "label list", %issue_id);
                commands::label::list(repo.unwrap(), &issue_id)?;
            }
            LabelCommand::ListAll => {
                info!(command = "label list-all");
                commands::label::list_all(repo.unwrap())?;
            }
        },
        Commands::Doc(doc_cmd) => match doc_cmd {
            DocCommand::Add { issue_id, file_path } => {
                info!(command = "doc add", %issue_id, %file_path);
                commands::doc::add(repo.unwrap(), &issue_id, &file_path)?;
            }
            DocCommand::Edit { issue_id, doc_name } => {
                info!(command = "doc edit", %issue_id, %doc_name);
                commands::doc::edit(repo.unwrap(), &issue_id, &doc_name)?;
            }
            DocCommand::Sync { issue_id, doc_name } => {
                info!(command = "doc sync", %issue_id, %doc_name);
                commands::doc::sync(repo.unwrap(), &issue_id, &doc_name)?;
            }
            DocCommand::List { issue_id } => {
                info!(command = "doc list", %issue_id);
                commands::doc::list(repo.unwrap(), &issue_id)?;
            }
        },
        Commands::Delete { issue_ids, force, cascade, from_file } => {
            info!(command = "delete", ids = issue_ids.len(), force, cascade, from_file = from_file.as_deref());
            commands::delete::run(repo.unwrap(), issue_ids, force, cascade, from_file)?;
        },
    }

    Ok(())
}
