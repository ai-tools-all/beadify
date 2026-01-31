//! Issue update command with natural CLI interface
//!
//! Supports both individual flags and JSON --data flag for backward compatibility.
/// Individual flags override JSON values.

use anyhow::{anyhow, Result};
use beads_core::{
    add_label_to_issue, remove_label_from_issue, repo::BeadsRepo, update_issue, IssueUpdate,
};

/// Run the issue update command
///
/// # Arguments
/// * `repo` - The beads repository
/// * `id` - Issue ID to update
/// * `title` - New title (optional)
/// * `description` - New description (optional)
/// * `kind` - New kind (optional)
/// * `priority` - New priority as u32 (optional)
/// * `status` - New status (optional)
/// * `add_label` - Comma-separated labels to add (optional)
/// * `remove_label` - Comma-separated labels to remove (optional)
/// * `data` - JSON data for backward compatibility (optional, flags override this)
pub fn run(
    repo: BeadsRepo,
    id: &str,
    title: Option<String>,
    description: Option<String>,
    kind: Option<String>,
    priority: Option<u32>,
    status: Option<String>,
    add_label: Option<String>,
    remove_label: Option<String>,
    data: Option<String>,
) -> Result<()> {
    let mut update = IssueUpdate::default();

    // Set direct fields
    update.title = title;
    update.kind = kind;
    update.description = description;
    update.status = status;

    // Priority is now pre-validated by clap as u32
    update.priority = priority;

    // Parse JSON --data if provided
    if let Some(data_str) = data {
        let json = serde_json::from_str::<serde_json::Value>(&data_str)
            .map_err(|e| anyhow!("Invalid JSON data: {}", e))?;

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
    let has_label_operations = add_label.is_some() || remove_label.is_some();

    if !has_field_updates && !has_label_operations {
        return Err(anyhow!("No updates specified"));
    }

    // Apply field updates if any
    if has_field_updates {
        let event = update_issue(&repo, id, update)?;
        println!("Updated issue {} via event {}", id, event.event_id);
    }

    // Handle label operations
    if let Some(labels_str) = add_label {
        let label_names: Vec<&str> = labels_str.split(',').map(|s| s.trim()).collect();
        for label_name in label_names {
            if !label_name.is_empty() {
                match add_label_to_issue(&repo, id, label_name) {
                    Ok(_) => println!("Added label '{}'", label_name),
                    Err(e) => eprintln!("Failed to add label '{}': {}", label_name, e),
                }
            }
        }
    }

    if let Some(labels_str) = remove_label {
        let label_names: Vec<&str> = labels_str.split(',').map(|s| s.trim()).collect();
        for label_name in label_names {
            if !label_name.is_empty() {
                match remove_label_from_issue(&repo, id, label_name) {
                    Ok(_) => println!("Removed label '{}'", label_name),
                    Err(e) => eprintln!("Failed to remove label '{}': {}", label_name, e),
                }
            }
        }
    }

    Ok(())
}
