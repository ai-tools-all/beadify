use anyhow::Result;
use beads_core::repo::BeadsRepo;

pub fn add(repo: BeadsRepo, issue_id: &str, label_name: &str) -> Result<()> {
    let label = beads_core::add_label_to_issue(&repo, issue_id, label_name)?;
    println!("Added label '{}' to {}", label.name, issue_id);
    Ok(())
}

pub fn remove(repo: BeadsRepo, issue_id: &str, label_name: &str) -> Result<()> {
    beads_core::remove_label_from_issue(&repo, issue_id, label_name)?;
    println!("Removed label '{}' from {}", label_name, issue_id);
    Ok(())
}

pub fn list(repo: BeadsRepo, issue_id: &str) -> Result<()> {
    let labels = beads_core::get_issue_labels(&repo, issue_id)?;
    if labels.is_empty() {
        println!("No labels on {}", issue_id);
    } else {
        println!("Labels on {}:", issue_id);
        for label in labels {
            println!("  - {}", label.name);
        }
    }
    Ok(())
}

pub fn list_all(repo: BeadsRepo) -> Result<()> {
    let labels = beads_core::get_all_labels(&repo)?;
    
    if labels.is_empty() {
        println!("No labels in database");
        return Ok(());
    }

    // Count usage for each label
    let mut label_counts = std::collections::HashMap::new();
    for label in &labels {
        match beads_core::get_issues_by_label(&repo, &label.name) {
            Ok(issues) => {
                label_counts.insert(label.name.clone(), issues.len());
            }
            Err(_) => {
                label_counts.insert(label.name.clone(), 0);
            }
        }
    }

    println!("All labels:");
    for label in labels {
        let count = label_counts.get(&label.name).unwrap_or(&0);
        println!("  - {} ({})", label.name, count);
    }
    Ok(())
}
