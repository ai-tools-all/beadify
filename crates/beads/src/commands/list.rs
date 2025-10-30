use anyhow::Result;
use beads_core::{get_all_issues, get_dependencies, get_issue, repo::BeadsRepo};

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
            
            // Show dependencies/blockers if any
            if let Ok(deps) = get_dependencies(&repo, &issue.id) {
                for dep_id in deps {
                    if let Ok(Some(dep_issue)) = get_issue(&repo, &dep_id) {
                        let dep_priority = format!("p{}", dep_issue.priority);
                        println!(
                            "  {} ↳ {:<8} {:<10} {} - {}",
                            " ", dep_id, dep_issue.kind, dep_priority, dep_issue.title
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
