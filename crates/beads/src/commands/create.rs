use anyhow::Result;
use beads_core::{create_issue, repo::BeadsRepo};

pub fn run(
    repo: BeadsRepo,
    title: &str,
    description: Option<String>,
    _design: Option<String>,
    _acceptance_criteria: Option<String>,
    _notes: Option<String>,
    kind: &str,
    priority: u32,
) -> Result<()> {
    let event = create_issue(&repo, title, kind, priority)?;
    println!("Created issue {}", event.id);
    if description.is_some() {
        eprintln!("Note: Extended fields (description, design, etc.) not yet stored in database");
    }
    Ok(())
}
