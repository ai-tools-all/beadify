use anyhow::Result;
use beads_core::{create_issue, repo::BeadsRepo};

pub fn run(repo: BeadsRepo, title: &str, kind: &str, priority: u32) -> Result<()> {
    let event = create_issue(&repo, title, kind, priority)?;
    println!("Created issue {}", event.id);
    Ok(())
}
