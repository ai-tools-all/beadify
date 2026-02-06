//! Issue update command with natural CLI interface
//!
//! Supports both individual flags and JSON --data flag for backward compatibility.
//! Individual flags override JSON values.

use anyhow::Result;
use beads_core::{
    add_label_to_issue, remove_label_from_issue, repo::BeadsRepo, update_issue, Error, IssueUpdate,
};

/// Parameters for updating an issue
pub struct UpdateParams {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub priority: Option<u32>,
    pub status: Option<String>,
    pub add_label: Vec<String>,
    pub remove_label: Vec<String>,
    pub data: Option<String>,
}

/// Run the issue update command
pub fn run(repo: BeadsRepo, params: UpdateParams) -> Result<()> {
    let UpdateParams {
        id,
        title,
        description,
        kind,
        priority,
        status,
        add_label,
        remove_label,
        data,
    } = params;
    let mut update = IssueUpdate {
        title,
        kind,
        description,
        status,
        priority,
        ..Default::default()
    };

    // Parse JSON --data if provided
    if let Some(data_str) = data {
        let json = serde_json::from_str::<serde_json::Value>(&data_str).map_err(|e| {
            Error::InvalidJson {
                context: "issue update --data".to_string(),
                expected_format: r#"{
  "title": "string",
  "description": "string",
  "priority": 0-3,
  "status": "open|in_progress|review|closed",
  "kind": "bug|feature|refactor|docs|chore|task"
}"#
                .to_string(),
                example: r#"beads issue update bd-042 --data '{"status":"closed"}'"#.to_string(),
                source: e,
            }
        })?;

        // Apply JSON values only if not already set by flags
        if update.description.is_none() {
            update.description = json
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        if update.design.is_none() {
            update.design = json
                .get("design")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        if update.acceptance_criteria.is_none() {
            update.acceptance_criteria = json
                .get("acceptance_criteria")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        if update.notes.is_none() {
            update.notes = json
                .get("notes")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        update.data = Some(json);
    }

    // Check if we have any updates to apply
    let has_field_updates = !update.is_empty();
    let has_label_operations = !add_label.is_empty() || !remove_label.is_empty();

    if !has_field_updates && !has_label_operations {
        return Err(Error::empty_issue_update(id).into());
    }

    if has_field_updates {
        let event = update_issue(&repo, &id, update)?;
        println!("Updated issue {} via event {}", id, event.event_id);
    }

    for raw in &add_label {
        for label_name in raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            match add_label_to_issue(&repo, &id, label_name) {
                Ok(_) => println!("Added label '{}'", label_name),
                Err(e) => eprintln!("Failed to add label '{}': {}", label_name, e),
            }
        }
    }

    for raw in &remove_label {
        for label_name in raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            match remove_label_from_issue(&repo, &id, label_name) {
                Ok(_) => println!("Removed label '{}'", label_name),
                Err(e) => eprintln!("Failed to remove label '{}': {}", label_name, e),
            }
        }
    }

    Ok(())
}
