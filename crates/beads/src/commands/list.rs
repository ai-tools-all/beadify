use anyhow::Result;
use beads_core::{get_all_issues, get_dependencies, get_issue, repo::BeadsRepo};

fn status_indicator(status: &str) -> &'static str {
    match status {
        "closed" => "●",
        _ => "☐",
    }
}

pub fn run(repo: BeadsRepo, show_all: bool, status_filter: Option<String>, dep_graph: bool) -> Result<()> {
    let mut issues = get_all_issues(&repo)?;
    
    // Filter issues based on status
    if let Some(status) = status_filter {
        issues.retain(|issue| issue.status == status);
    } else if !show_all {
        issues.retain(|issue| issue.status == "open");
    }
    
    if issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }
    
    if dep_graph {
        // Tree view for dependency graph
        println!("Dependency Graph:");
        println!();
        
        for issue in issues {
            print_tree_node(&repo, &issue, 0)?;
        }
    } else {
        // Table view (default)
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

fn print_tree_node(repo: &BeadsRepo, issue: &beads_core::Issue, depth: usize) -> Result<()> {
    let prefix = if depth == 0 {
        "".to_string()
    } else {
        "  ".repeat(depth - 1) + "└─ "
    };
    
    let indicator = status_indicator(&issue.status);
    println!("{}{} {} [{}] p{} - {}", prefix, indicator, issue.id, issue.status, issue.priority, issue.title);
    
    // Show dependencies
    if let Ok(deps) = get_dependencies(repo, &issue.id) {
        for (idx, dep_id) in deps.iter().enumerate() {
            if let Ok(Some(dep_issue)) = get_issue(repo, dep_id) {
                let is_last = idx == deps.len() - 1;
                let sub_prefix = if depth == 0 {
                    if is_last {
                        "└─ ".to_string()
                    } else {
                        "├─ ".to_string()
                    }
                } else {
                    if is_last {
                        "  ".repeat(depth) + "└─ "
                    } else {
                        "  ".repeat(depth) + "├─ "
                    }
                };
                
                let indicator = status_indicator(&dep_issue.status);
                println!("{}{} {} [{}] p{} - {}", sub_prefix, indicator, dep_issue.id, dep_issue.status, dep_issue.priority, dep_issue.title);
            }
        }
    }
    
    Ok(())
}
