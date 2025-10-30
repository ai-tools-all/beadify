use anyhow::Result;
use beads_core::{get_all_issues, repo::BeadsRepo};

pub fn run(repo: BeadsRepo) -> Result<()> {
    let issues = get_all_issues(&repo)?;
    if issues.is_empty() {
        println!("No issues found.");
    } else {
        for issue in issues {
            println!(
                "{} [{}] {} p{} - {}",
                issue.id, issue.status, issue.kind, issue.priority, issue.title
            );
        }
    }
    Ok(())
}
