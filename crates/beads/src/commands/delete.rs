use std::fs;
use std::io::{self, Write};

use anyhow::{Context, Result};
use beads_core::{delete_issues_batch, get_delete_impact, repo::BeadsRepo};

pub fn run(
    repo: BeadsRepo,
    issue_ids: Vec<String>,
    force: bool,
    cascade: bool,
    from_file: Option<String>,
) -> Result<()> {
    // Collect all issue IDs
    let mut all_ids = issue_ids;

    if let Some(file_path) = from_file {
        let content = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read file: {}", file_path))?;

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                all_ids.push(trimmed.to_string());
            }
        }
    }

    if all_ids.is_empty() {
        anyhow::bail!("No issue IDs provided");
    }

    // Preview mode: show what would be deleted
    if !force {
        println!("Preview mode - the following would be deleted:");
        println!("(Issues will be marked as deleted in events.jsonl)");
        println!();

        let mut total_issues = 0;
        let mut total_blocked = 0;
        let mut total_refs = 0;

        for issue_id in &all_ids {
            match get_delete_impact(&repo, issue_id, cascade) {
                Ok(impact) => {
                    println!("Issue: {}", issue_id);

                    if cascade && impact.issues_to_delete.len() > 1 {
                        println!(
                            "  Would delete {} issue(s) (cascade):",
                            impact.issues_to_delete.len()
                        );
                        for item in &impact.issues_to_delete {
                            println!("    - {} - {}", item.id, item.title);
                        }
                    } else {
                        println!(
                            "  Would delete: {} - {}",
                            impact.issues_to_delete[0].id, impact.issues_to_delete[0].title
                        );
                    }

                    if !impact.blocked_issues.is_empty() {
                        println!(
                            "  ⚠️  {} issue(s) depend on this:",
                            impact.blocked_issues.len()
                        );
                        for blocked in &impact.blocked_issues {
                            println!("    - {}", blocked);
                        }
                    }

                    if !impact.text_references.is_empty() {
                        println!(
                            "  📝 {} issue(s) reference this in text",
                            impact.text_references.len()
                        );
                    }

                    total_issues += impact.issues_to_delete.len();
                    total_blocked += impact.blocked_issues.len();
                    total_refs += impact.text_references.len();

                    println!();
                }
                Err(e) => {
                    eprintln!("✗ Error analyzing {}: {}", issue_id, e);
                    println!();
                }
            }
        }

        println!("Summary:");
        println!("  Total issues to delete: {}", total_issues);
        if total_blocked > 0 {
            println!("  ⚠️  Issues with dependents: {}", total_blocked);
        }
        if total_refs > 0 {
            println!("  📝 Text references to update: {}", total_refs);
        }
        println!();
        println!("Run with --force to confirm deletion");
        return Ok(());
    }

    // Confirm before cascade deletion
    if cascade && !all_ids.is_empty() {
        print!("⚠️  Cascade deletion will delete dependents recursively. Continue? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    // Execute deletions
    let result = delete_issues_batch(&repo, all_ids, cascade)?;

    // Report results
    if !result.successes.is_empty() {
        println!(
            "✓ Successfully deleted {} issue(s):",
            result.successes.len()
        );
        for issue_id in &result.successes {
            println!("  - {}", issue_id);
        }
    }

    if !result.failures.is_empty() {
        println!();
        println!("✗ Failed to delete {} issue(s):", result.failures.len());
        for failure in &result.failures {
            println!("  - {}: {}", failure.issue_id, failure.error);
        }
    }

    Ok(())
}
