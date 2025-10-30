use anyhow::{anyhow, Result};
use beads_core::{get_open_dependencies, get_issue, repo::BeadsRepo};

pub fn run(repo: BeadsRepo, id: &str) -> Result<()> {
    let issue = get_issue(&repo, id)?
        .ok_or_else(|| anyhow!("Issue '{}' not found", id))?;

    println!("ID:       {}", issue.id);
    println!("Title:    {}", issue.title);
    println!("Status:   {}", issue.status);
    println!("Kind:     {}", issue.kind);
    println!("Priority: {}", issue.priority);

    if let Some(data) = &issue.data {
        println!("Data:");
        if let Some(obj) = data.as_object() {
            for (key, value) in obj.iter() {
                let formatted_value = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => value.to_string(),
                };
                println!("  {}:  {}", key, formatted_value);
            }
        } else {
            println!("  {}", data);
        }
    }

    let deps = get_open_dependencies(&repo, id)?;
    if !deps.is_empty() {
        println!("\nBlocked By:");
        for dep_id in deps {
            if let Ok(Some(dep_issue)) = get_issue(&repo, &dep_id) {
                println!("  ↳ {} [{}] p{} - {}", dep_id, dep_issue.status, dep_issue.priority, dep_issue.title);
            } else {
                println!("  ↳ {} [not found]", dep_id);
            }
        }
    }

    Ok(())
}
