use anyhow::Result;
use beads_core::{get_all_issues, repo::BeadsRepo};
use std::collections::HashMap;

pub fn run(repo: BeadsRepo) -> Result<()> {
    let mut issues = get_all_issues(&repo)?;

    // Filter only open issues
    issues.retain(|issue| issue.status == "open");

    if issues.is_empty() {
        println!("No open issues found.");
        return Ok(());
    }

    // Sort by priority (1 first), then by ID for deterministic ordering
    issues.sort_by(|a, b| match a.priority.cmp(&b.priority) {
        std::cmp::Ordering::Equal => a.id.cmp(&b.id),
        other => other,
    });

    // Group issues by priority
    let mut priority_groups: HashMap<u32, Vec<_>> = HashMap::new();
    for issue in &issues {
        priority_groups
            .entry(issue.priority)
            .or_default()
            .push(issue);
    }

    // Find the next issue (first in priority 1, or lowest priority if no p1)
    let next_issue = issues.first().unwrap();
    let next_indicator = "👉 NEXT";

    // Display grouped by priority
    let priorities = [1, 2, 3]; // Define priority order
    let mut found_next = false;

    for priority in priorities {
        if let Some(priority_issues) = priority_groups.get(&priority) {
            println!(
                "Priority {} ({} issue(s)):",
                priority,
                priority_issues.len()
            );

            for issue in priority_issues {
                let indicator = if !found_next && issue.id == next_issue.id {
                    found_next = true;
                    next_indicator
                } else {
                    "     "
                };

                println!(
                    "{} {} [{}] {} p{} - {}",
                    indicator, issue.id, issue.status, issue.kind, issue.priority, issue.title
                );
            }
            println!();
        }
    }

    Ok(())
}
