use anyhow::{anyhow, Result};
use beads_core::{create_issue_with_data, add_label_to_issue, repo::BeadsRepo};
use serde::Deserialize;

#[derive(Deserialize)]
struct IssueData {
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

pub fn run(repo: BeadsRepo, title: &str, data: &str, depends_on: Vec<String>, labels: Option<String>) -> Result<()> {
    if title.trim().is_empty() {
        return Err(anyhow!("Title is required and cannot be empty"));
    }

    let issue_data: IssueData = serde_json::from_str(data)
        .map_err(|e| anyhow!("Invalid JSON data: {}", e))?;

    let data_json: serde_json::Value = serde_json::from_str(data)
        .map_err(|e| anyhow!("Invalid JSON data: {}", e))?;

    let event = create_issue_with_data(&repo, title, &issue_data.kind, issue_data.priority, depends_on, Some(data_json))?;
    
    println!("Created issue {}", event.id);
    
    // Add labels if provided
    if let Some(label_str) = labels {
        let label_names: Vec<&str> = label_str.split(',').map(|s| s.trim()).collect();
        for label_name in label_names {
            if !label_name.is_empty() {
                match add_label_to_issue(&repo, &event.id, label_name) {
                    Ok(_) => println!("Added label '{}' to {}", label_name, event.id),
                    Err(e) => eprintln!("Failed to add label '{}': {}", label_name, e),
                }
            }
        }
    }
    
    Ok(())
}
