use anyhow::Result;
use beads_core::{get_all_issues, repo::BeadsRepo};

fn status_indicator(status: &str) -> &'static str {
    match status {
        "closed" => "●",
        _ => "☐",
    }
}

pub fn run(repo: BeadsRepo, show_all: bool, status_filter: Option<String>) -> Result<()> {
    let mut issues = get_all_issues(&repo)?;
    
    // Filter issues based on status
    if let Some(status) = status_filter {
        // Explicit status filter
        issues.retain(|issue| issue.status == status);
    } else if !show_all {
        // Default: show only open issues
        issues.retain(|issue| issue.status == "open");
    }
    
    if issues.is_empty() {
        println!("No issues found.");
    } else {
        // Print table header
        println!("{:<2} {:<8} {:<10} {:<4} {}", " ", "ID", "Kind", "Prio", "Title");
        println!("{}", "─".repeat(80));
        
        for issue in issues {
            let indicator = status_indicator(&issue.status);
            let priority_str = format!("p{}", issue.priority);
            println!(
                "{} {:<8} {:<10} {:<4} {}",
                indicator, issue.id, issue.kind, priority_str, issue.title
            );
        }
    }
    Ok(())
}
