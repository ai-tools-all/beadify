//! Issue show command - displays issue details
//!
//! Simple wrapper around the existing show command functionality.

use anyhow::Result;
use beads_core::{get_issue, get_dependencies, get_dependents, get_issue_labels, repo::BeadsRepo};

/// Run the issue show command
///
/// # Arguments
/// * `repo` - The beads repository
/// * `id` - Issue ID to show
pub fn run(repo: BeadsRepo, id: &str) -> Result<()> {
    // Get the issue
    let issue = match get_issue(&repo, id)? {
        Some(issue) => issue,
        None => {
            eprintln!("Issue '{}' not found", id);
            return Ok(());
        }
    };

    // Get related data
    let labels = get_issue_labels(&repo, id).unwrap_or_default();
    let dependencies = get_dependencies(&repo, id).unwrap_or_default();
    let dependents = get_dependents(&repo, id).unwrap_or_default();

    // Print issue details
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ {:<59} │", format!("{}: {}", issue.id, issue.title));
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ {:<15} {:<43} │", "Kind:", issue.kind);
    println!("│ {:<15} {:<43} │", "Priority:", format!("p{}", issue.priority));
    println!("│ {:<15} {:<43} │", "Status:", issue.status);

    // Labels
    if !labels.is_empty() {
        let label_names: Vec<String> = labels.iter().map(|l| l.name.clone()).collect();
        println!("│ {:<15} {:<43} │", "Labels:", label_names.join(", "));
    }

    // Dependencies
    if !dependencies.is_empty() {
        println!("├─────────────────────────────────────────────────────────────┤");
        println!("│ Dependencies:                                               │");
        for dep_id in &dependencies {
            if let Ok(Some(dep_issue)) = get_issue(&repo, dep_id) {
                println!("│   • {:<8} {:<10} - {:<30} │", dep_id, dep_issue.kind, dep_issue.title);
            } else {
                println!("│   • {:<8} (unknown)                              │", dep_id);
            }
        }
    }

    // Dependents
    if !dependents.is_empty() {
        println!("├─────────────────────────────────────────────────────────────┤");
        println!("│ Dependents:                                                 │");
        for dep_id in &dependents {
            if let Ok(Some(dep_issue)) = get_issue(&repo, dep_id) {
                println!("│   • {:<8} {:<10} - {:<30} │", dep_id, dep_issue.kind, dep_issue.title);
            } else {
                println!("│   • {:<8} (unknown)                              │", dep_id);
            }
        }
    }

    // Description
    if let Some(desc) = &issue.description {
        println!("├─────────────────────────────────────────────────────────────┤");
        println!("│ Description:                                                │");
        for line in desc.lines() {
            // Truncate long lines
            let display_line = if line.len() > 59 {
                format!("{}...", &line[..56])
            } else {
                line.to_string()
            };
            println!("│ {:<59} │", display_line);
        }
    }

    // Design
    if let Some(design) = &issue.design {
        println!("├─────────────────────────────────────────────────────────────┤");
        println!("│ Design:                                                     │");
        for line in design.lines() {
            let display_line = if line.len() > 59 {
                format!("{}...", &line[..56])
            } else {
                line.to_string()
            };
            println!("│ {:<59} │", display_line);
        }
    }

    // Acceptance Criteria
    if let Some(ac) = &issue.acceptance_criteria {
        println!("├─────────────────────────────────────────────────────────────┤");
        println!("│ Acceptance Criteria:                                        │");
        for line in ac.lines() {
            let display_line = if line.len() > 59 {
                format!("{}...", &line[..56])
            } else {
                line.to_string()
            };
            println!("│ {:<59} │", display_line);
        }
    }

    // Notes
    if let Some(notes) = &issue.notes {
        println!("├─────────────────────────────────────────────────────────────┤");
        println!("│ Notes:                                                      │");
        for line in notes.lines() {
            let display_line = if line.len() > 59 {
                format!("{}...", &line[..56])
            } else {
                line.to_string()
            };
            println!("│ {:<59} │", display_line);
        }
    }

    println!("└─────────────────────────────────────────────────────────────┘");

    Ok(())
}
