//! Issue create command with natural CLI interface
//!
//! Supports both individual flags (--title, --priority, etc.) and
//! JSON --data flag for backward compatibility. Individual flags override JSON values.

use std::fs;

use anyhow::{anyhow, Result};
use beads_core::{
    add_document_to_issue, add_label_to_issue, create_issue_with_data, repo::BeadsRepo,
    update_issue, IssueUpdate,
};

/// Run the issue create command
///
/// # Arguments
/// * `repo` - The beads repository
/// * `title` - Issue title (required)
/// * `description` - Issue description (optional)
/// * `kind` - Issue kind (optional, defaults to "task")
/// * `priority` - Priority as u32 (optional, defaults to 1/medium)
/// * `label` - Comma-separated labels to add (optional)
/// * `depends_on` - Dependencies (can be used multiple times)
/// * `doc` - Documents in "name:path" format (can be used multiple times)
/// * `data` - JSON data for backward compatibility (optional, flags override this)
pub fn run(
    repo: BeadsRepo,
    title: &str,
    description: Option<String>,
    kind: Option<String>,
    priority: Option<u32>,
    label: Option<String>,
    depends_on: Vec<String>,
    doc: Vec<String>,
    data: Option<String>,
) -> Result<()> {
    // 1. Validate title
    if title.trim().is_empty() {
        return Err(anyhow!("Title is required and cannot be empty"));
    }

    // 2. Parse JSON --data if provided
    let (json_kind, json_priority, json_desc): (Option<String>, Option<u32>, Option<String>) =
        if let Some(data_str) = &data {
            let parsed = serde_json::from_str::<serde_json::Value>(data_str)
                .map_err(|e| anyhow!("Invalid JSON data: {}\n\nExpected format: '{{\"description\":\"...\",\"priority\":1,\"kind\":\"bug\"}}'", e))?;
            let kind = parsed.get("kind").and_then(|v| v.as_str()).map(|s| s.to_string());
            let priority = parsed.get("priority").and_then(|v| v.as_u64()).map(|p| p as u32);
            let desc = parsed
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (kind, priority, desc)
        } else {
            (None, None, None)
        };

    // 3. Merge flags (flags override JSON)
    let final_kind = kind.or(json_kind).unwrap_or_else(|| "task".to_string());

    // Priority is now pre-validated by clap as u32
    let final_priority = priority.or(json_priority).unwrap_or(1); // default to "medium"

    let final_description = description.or(json_desc);

    // 4. Create issue
    let data_json: Option<serde_json::Value> = data
        .as_ref()
        .map(|s| serde_json::from_str(s).ok())
        .flatten();

    let event = create_issue_with_data(
        &repo,
        title,
        &final_kind,
        final_priority,
        depends_on,
        data_json,
    )?;

    println!("Created issue {}", event.id);

    // 5. Update description if provided
    if let Some(desc) = final_description {
        let mut update = IssueUpdate::default();
        update.description = Some(desc);
        update_issue(&repo, &event.id, update)?;
    }

    // 6. Add labels
    if let Some(label_str) = label {
        let label_names: Vec<&str> = label_str.split(',').map(|s| s.trim()).collect();
        for label_name in label_names {
            if !label_name.is_empty() {
                match add_label_to_issue(&repo, &event.id, label_name) {
                    Ok(_) => println!("Added label '{}'", label_name),
                    Err(e) => eprintln!("Failed to add label '{}': {}", label_name, e),
                }
            }
        }
    }

    // 7. Attach documents
    for doc_spec in doc {
        let parts: Vec<&str> = doc_spec.splitn(2, ':').collect();
        if parts.len() != 2 {
            eprintln!("Invalid doc format '{}'. Expected 'name:path'", doc_spec);
            continue;
        }

        let doc_name = parts[0];
        let file_path = parts[1];

        match fs::read(file_path) {
            Ok(content) => {
                match add_document_to_issue(&repo, &event.id, doc_name, &content) {
                    Ok(_) => println!("Attached '{}' to {}", doc_name, event.id),
                    Err(e) => eprintln!("Failed to attach '{}': {}", doc_name, e),
                }
            }
            Err(e) => eprintln!("Failed to read file '{}': {}", file_path, e),
        }
    }

    Ok(())
}
