use anyhow::Result;
use beads_core::{repo::BeadsRepo, update_issue, IssueUpdate};

pub fn run(
    repo: BeadsRepo,
    id: &str,
    title: Option<String>,
    kind: Option<String>,
    priority: Option<u32>,
    status: Option<String>,
) -> Result<()> {
    let mut update = IssueUpdate::default();
    update.title = title;
    update.kind = kind;
    update.priority = priority;
    update.status = status;

    let event = update_issue(&repo, id, update)?;
    println!("Updated issue {} via event {}", event.id, event.event_id);
    Ok(())
}
