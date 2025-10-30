use anyhow::{anyhow, Result};
use beads_core::{create_issue, repo::BeadsRepo};
use serde::Deserialize;

#[derive(Deserialize)]
struct IssueData {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    design: Option<String>,
    #[serde(default)]
    acceptance_criteria: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default = "default_priority")]
    priority: u32,
}

fn default_kind() -> String {
    "task".to_string()
}

fn default_priority() -> u32 {
    2
}

pub fn run(repo: BeadsRepo, title: &str, data: &str, depends_on: Vec<String>) -> Result<()> {
    if title.trim().is_empty() {
        return Err(anyhow!("Title is required and cannot be empty"));
    }

    let issue_data: IssueData = serde_json::from_str(data)
        .map_err(|e| anyhow!("Invalid JSON data: {}", e))?;

    let event = create_issue(&repo, title, &issue_data.kind, issue_data.priority, depends_on)?;
    
    println!("Created issue {}", event.id);
    
    if issue_data.description.is_some() || issue_data.design.is_some() 
        || issue_data.acceptance_criteria.is_some() || issue_data.notes.is_some() {
        eprintln!("Note: Extended fields (description, design, etc.) not yet stored in database");
    }
    
    Ok(())
}
