use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

use beads_core::repo::find_repo;

mod cli;
mod commands;

use cli::enums::{Kind, Priority, Status};

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
    /// Create a new issue (DEPRECATED: use `issue create` instead)
    #[command(
        hide = true,
        long_about = "Create a new issue.\n\nNOTE: This command is deprecated. Use 'beads issue create' with individual flags instead.\nExample: beads issue create --title \"...\" --priority high"
    )]
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
    /// Show issue details (DEPRECATED: use `issue show` instead)
    #[command(hide = true)]
    Show { id: String },
    /// List all issues from the local cache (DEPRECATED: use `issue list` instead)
    #[command(hide = true)]
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
    /// Update an existing issue (DEPRECATED: use `issue update` instead)
    #[command(hide = true)]
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
        /// Filter by issue kind: bug, feature, refactor, docs, chore, task
        #[arg(long, value_enum)]
        kind: Option<Kind>,
        /// Filter by status: open, in-progress, review, closed
        #[arg(long, value_enum)]
        status: Option<Status>,
        /// Filter by priority: low, medium, high, urgent (or 0-3)
        #[arg(long, value_enum)]
        priority: Option<Priority>,
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
    /// Manage issues (create, update, list, show)
    #[command(subcommand)]
    Issue(IssueCommand),
}

#[derive(Subcommand)]
enum IssueCommand {
    /// Create a new issue with natural CLI interface
    Create {
        #[arg(short, long)]
        title: String,

        #[arg(long)]
        description: Option<String>,

        /// Issue kind: bug, feature, refactor, docs, chore, task
        #[arg(long, value_enum)]
        kind: Option<Kind>,

        /// Priority: low, medium, high, urgent (or 0-3)
        #[arg(long, value_enum)]
        priority: Option<Priority>,

        #[arg(short, long)]
        label: Option<String>, // comma-separated

        #[arg(long)]
        depends_on: Vec<String>,

        #[arg(long)]
        doc: Vec<String>, // name:path format

        #[arg(long)]
        data: Option<String>, // JSON escape hatch
    },

    /// Update an existing issue with natural CLI interface
    Update {
        id: String,

        #[arg(long)]
        title: Option<String>,

        #[arg(long)]
        description: Option<String>,

        /// Issue kind: bug, feature, refactor, docs, chore, task
        #[arg(long, value_enum)]
        kind: Option<Kind>,

        /// Priority: low, medium, high, urgent (or 0-3)
        #[arg(long, value_enum)]
        priority: Option<Priority>,

        /// Status: open, in-progress, review, closed
        #[arg(long, value_enum)]
        status: Option<Status>,

        /// Labels to add (repeatable, comma-separated)
        #[arg(short = 'l', long, num_args = 1..)]
        add_label: Vec<String>,

        /// Labels to remove (repeatable, comma-separated)
        #[arg(short = 'r', long, num_args = 1..)]
        remove_label: Vec<String>,

        #[arg(long)]
        data: Option<String>, // JSON escape hatch
    },

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
        label: Option<String>, // filter by labels

        #[arg(long)]
        flat: bool,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        labels: bool,

        /// Filter issues created after this date (e.g., '2026-01-20' or '1 week ago')
        #[arg(long)]
        created_after: Option<String>,

        /// Filter issues created before this date (e.g., '2026-01-20' or '1 week ago')
        #[arg(long)]
        created_before: Option<String>,

        /// User timezone for date parsing (e.g., 'America/New_York'). Default: system timezone
        #[arg(long)]
        timezone: Option<String>,
    },

    /// Show issue details
    Show { id: String },
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
        Commands::Create {
            title,
            data,
            depends_on,
            label,
            doc,
        } => {
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
            info!(
                command = "list",
                all,
                status = status.as_deref(),
                flat,
                label = label.as_deref(),
                label_any = label_any.as_deref(),
                json,
                labels
            );
            commands::list::run(
                repo.unwrap(),
                commands::list::ListParams {
                    show_all: all,
                    status_filter: status,
                    flat,
                    label_filter: label,
                    label_any_filter: label_any,
                    json_output: json,
                    show_labels: labels,
                },
            )?;
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
            info!(command = "search", %query, kind = ?kind, status = ?status, priority = ?priority, title_only);
            commands::search::run(
                repo.unwrap(),
                &query,
                kind.map(|k| k.as_str().to_string()),
                status.map(|s| s.as_str().to_string()),
                priority.map(|p| p.as_u32()),
                title_only,
            )?;
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
            LabelCommand::Add {
                issue_id,
                label_name,
            } => {
                info!(command = "label add", %issue_id, %label_name);
                commands::label::add(repo.unwrap(), &issue_id, &label_name)?;
            }
            LabelCommand::Remove {
                issue_id,
                label_name,
            } => {
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
            DocCommand::Add {
                issue_id,
                file_path,
            } => {
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
        Commands::Delete {
            issue_ids,
            force,
            cascade,
            from_file,
        } => {
            info!(
                command = "delete",
                ids = issue_ids.len(),
                force,
                cascade,
                from_file = from_file.as_deref()
            );
            commands::delete::run(repo.unwrap(), issue_ids, force, cascade, from_file)?;
        }
        Commands::Issue(issue_cmd) => match issue_cmd {
            IssueCommand::Create {
                title,
                description,
                kind,
                priority,
                label,
                depends_on,
                doc,
                data,
            } => {
                info!(command = "issue create", %title);
                commands::issue::create::run(
                    repo.unwrap(),
                    commands::issue::create::CreateParams {
                        title,
                        description,
                        kind: kind.map(|k| k.as_str().to_string()),
                        priority: priority.map(|p| p.as_u32()),
                        label,
                        depends_on,
                        doc,
                        data,
                    },
                )?;
            }
            IssueCommand::Update {
                id,
                title,
                description,
                kind,
                priority,
                status,
                add_label,
                remove_label,
                data,
            } => {
                info!(command = "issue update", %id);
                commands::issue::update::run(
                    repo.unwrap(),
                    commands::issue::update::UpdateParams {
                        id,
                        title,
                        description,
                        kind: kind.map(|k| k.as_str().to_string()),
                        priority: priority.map(|p| p.as_u32()),
                        status: status.map(|s| s.as_str().to_string()),
                        add_label,
                        remove_label,
                        data,
                    },
                )?;
            }
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
                    commands::issue::list::ListParams {
                        show_all: all,
                        status_filter: status.map(|s| s.as_str().to_string()),
                        priority_filter: priority.map(|p| p.as_u32()),
                        kind_filter: kind.map(|k| k.as_str().to_string()),
                        label_filter: label,
                        label_any_filter: None,
                        flat,
                        json_output: json,
                        show_labels: labels,
                        created_after,
                        created_before,
                        timezone,
                    },
                )?;
            }
            IssueCommand::Show { id } => {
                info!(command = "issue show", %id);
                commands::issue::show::run(repo.unwrap(), &id)?;
            }
        },
    }

    Ok(())
}
