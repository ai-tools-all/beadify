use anyhow::Result;
use beads_core::{repo::BeadsRepo, update_issue, Error, IssueUpdate};
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
}

pub fn run(
    repo: BeadsRepo,
    id: &str,
    title: Option<String>,
    kind: Option<String>,
    priority: Option<u32>,
    status: Option<String>,
    data: Option<String>,
) -> Result<()> {
    let mut update = IssueUpdate {
        title,
        kind,
        priority,
        status,
        ..Default::default()
    };

    if let Some(data_str) = data {
        let issue_data: IssueData =
            serde_json::from_str(&data_str).map_err(|e| Error::InvalidJson {
                context: "update --data (legacy)".to_string(),
                expected_format: r#"{"description":"string","design":"string"}"#.to_string(),
                example: r#"beads update bd-042 --data '{"description":"Updated"}'"#.to_string(),
                source: e,
            })?;
        update.description = issue_data.description;
        update.design = issue_data.design;
        update.acceptance_criteria = issue_data.acceptance_criteria;
        update.notes = issue_data.notes;
    }

    let event = update_issue(&repo, id, update)?;
    println!("Updated issue {} via event {}", event.id, event.event_id);
    Ok(())
}
