use anyhow::Result;
use beads_core::{get_all_issues, repo::BeadsRepo};

pub fn run(
    repo: BeadsRepo,
    query: &str,
    kind_filter: Option<String>,
    status_filter: Option<String>,
    priority_filter: Option<u32>,
    title_only: bool,
) -> Result<()> {
    let mut issues = get_all_issues(&repo)?;
    
    // Apply filters
    issues.retain(|issue| {
        // Kind filter
        if let Some(ref kind) = kind_filter {
            if issue.kind != *kind {
                return false;
            }
        }
        
        // Status filter
        if let Some(ref status) = status_filter {
            if issue.status != *status {
                return false;
            }
        }
        
        // Priority filter
        if let Some(priority) = priority_filter {
            if issue.priority != priority {
                return false;
            }
        }
        
        // Text search (case-insensitive)
        let query_lower = query.to_lowercase();
        let title_match = issue.title.to_lowercase().contains(&query_lower);
        
        if title_match {
            return true;
        }
        
        // Search in description and other fields if not title_only
        if !title_only {
            if let Some(ref description) = issue.description {
                if description.to_lowercase().contains(&query_lower) {
                    return true;
                }
            }
            
            if let Some(ref design) = issue.design {
                if design.to_lowercase().contains(&query_lower) {
                    return true;
                }
            }
            
            if let Some(ref acceptance_criteria) = issue.acceptance_criteria {
                if acceptance_criteria.to_lowercase().contains(&query_lower) {
                    return true;
                }
            }
            
            if let Some(ref notes) = issue.notes {
                if notes.to_lowercase().contains(&query_lower) {
                    return true;
                }
            }
        }
        
        false
    });
    
    if issues.is_empty() {
        println!("No issues found.");
    } else {
        for issue in issues {
            println!(
                "{} [{}] {} p{} - {}",
                issue.id, issue.status, issue.kind, issue.priority, issue.title
            );
        }
    }
    
    Ok(())
}
