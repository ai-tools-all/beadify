use anyhow::Result;
use beads_core::{add_issue_dependency, get_dependencies, get_dependents, get_issue, remove_issue_dependency, repo::BeadsRepo};

pub fn show(repo: BeadsRepo, issue_id: &str) -> Result<()> {
    let issue = get_issue(&repo, issue_id)?
        .ok_or_else(|| anyhow::anyhow!("Issue '{}' not found", issue_id))?;
    
    println!("Dependencies for {} - {}", issue.id, issue.title);
    println!();
    
    // Show blockers (issues this depends on)
    let blockers = get_dependencies(&repo, issue_id)?;
    if !blockers.is_empty() {
        println!("Blockers (Issues this depends on):");
        for blocker_id in blockers {
            if let Ok(Some(blocker)) = get_issue(&repo, &blocker_id) {
                println!("  ↳ {} [{}] p{} - {}", blocker_id, blocker.status, blocker.priority, blocker.title);
            } else {
                println!("  ↳ {} [not found]", blocker_id);
            }
        }
        println!();
    } else {
        println!("No blockers");
        println!();
    }
    
    // Show dependents (issues that depend on this)
    let dependents = get_dependents(&repo, issue_id)?;
    if !dependents.is_empty() {
        println!("Dependents (Issues that depend on this):");
        for dependent_id in dependents {
            if let Ok(Some(dependent)) = get_issue(&repo, &dependent_id) {
                println!("  ↳ {} [{}] p{} - {}", dependent_id, dependent.status, dependent.priority, dependent.title);
            } else {
                println!("  ↳ {} [not found]", dependent_id);
            }
        }
    } else {
        println!("No dependents");
    }
    
    Ok(())
}

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
