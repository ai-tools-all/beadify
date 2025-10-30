use anyhow::{anyhow, Result};
use beads_core::{get_issue, repo::BeadsRepo};

pub fn run(repo: BeadsRepo, id: &str) -> Result<()> {
    let issue = get_issue(&repo, id)?
        .ok_or_else(|| anyhow!("Issue '{}' not found", id))?;

    println!("ID:       {}", issue.id);
    println!("Title:    {}", issue.title);
    println!("Status:   {}", issue.status);
    println!("Kind:     {}", issue.kind);
    println!("Priority: {}", issue.priority);

    Ok(())
}
