use anyhow::Result;
use beads_core::{add_issue_dependency, get_issue, remove_issue_dependency, repo::BeadsRepo};

pub fn add(repo: BeadsRepo, issue_id: &str, depends_on_id: &str) -> Result<()> {
    add_issue_dependency(&repo, issue_id, depends_on_id)?;
    
    if let Ok(Some(issue)) = get_issue(&repo, issue_id) {
        if let Ok(Some(dep_issue)) = get_issue(&repo, depends_on_id) {
            println!("✓ {} now depends on {}", issue.id, dep_issue.id);
            println!("  {} - {}", issue.id, issue.title);
            println!("  ↳ {} - {}", dep_issue.id, dep_issue.title);
        }
    }
    
    Ok(())
}

pub fn remove(repo: BeadsRepo, issue_id: &str, depends_on_id: &str) -> Result<()> {
    remove_issue_dependency(&repo, issue_id, depends_on_id)?;
    
    if let Ok(Some(issue)) = get_issue(&repo, issue_id) {
        println!("✓ Removed dependency: {} no longer depends on {}", issue.id, depends_on_id);
    }
    
    Ok(())
}
