use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use beads_core::{add_document_to_issue, blob, get_issue_documents, repo::BeadsRepo};

pub fn add(repo: BeadsRepo, issue_id: &str, file_path: &str) -> Result<()> {
    let path = Path::new(file_path);
    
    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    let content = fs::read(path)
        .with_context(|| format!("Failed to read file: {}", file_path))?;
    
    let doc_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

    add_document_to_issue(&repo, issue_id, doc_name, &content)?;

    println!("✓ Attached '{}' to {}", doc_name, issue_id);

    Ok(())
}

pub fn edit(repo: BeadsRepo, issue_id: &str, doc_name: &str) -> Result<()> {
    let documents = get_issue_documents(&repo, issue_id)?;
    
    let hash = documents
        .get(doc_name)
        .ok_or_else(|| anyhow::anyhow!("Document '{}' not found on issue {}", doc_name, issue_id))?;

    let content = blob::read_blob(&repo, hash)?;

    let docs_workspace = repo.beads_dir().join("docs").join(issue_id);
    fs::create_dir_all(&docs_workspace)
        .with_context(|| format!("Failed to create workspace directory: {:?}", docs_workspace))?;

    let workspace_file = docs_workspace.join(doc_name);
    fs::write(&workspace_file, &content)
        .with_context(|| format!("Failed to write to workspace: {:?}", workspace_file))?;

    println!("✓ Exported '{}' for {} to {}", doc_name, issue_id, workspace_file.display());
    println!("  Edit the file and run 'beads doc sync {} {}' when done", issue_id, doc_name);

    Ok(())
}

pub fn sync(repo: BeadsRepo, issue_id: &str, doc_name: &str) -> Result<()> {
    let docs_workspace = repo.beads_dir().join("docs").join(issue_id);
    let workspace_file = docs_workspace.join(doc_name);

    if !workspace_file.exists() {
        anyhow::bail!(
            "Workspace file not found: {}\nRun 'beads doc edit {} {}' first",
            workspace_file.display(),
            issue_id,
            doc_name
        );
    }

    let content = fs::read(&workspace_file)
        .with_context(|| format!("Failed to read workspace file: {:?}", workspace_file))?;

    let new_hash = blob::write_blob(&repo, &content)?;

    let documents = get_issue_documents(&repo, issue_id)?;
    
    if let Some(old_hash) = documents.get(doc_name) {
        if old_hash == &new_hash {
            println!("✓ No changes detected for '{}'", doc_name);
            return Ok(());
        }
    }

    add_document_to_issue(&repo, issue_id, doc_name, &content)?;

    println!("✓ Synced changes for '{}' on {}", doc_name, issue_id);
    
    print!("  Clean up workspace file? [y/N]: ");
    use std::io::{self, Write};
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().eq_ignore_ascii_case("y") {
        fs::remove_file(&workspace_file)?;
        println!("  Removed {}", workspace_file.display());
        
        if docs_workspace.read_dir()?.next().is_none() {
            fs::remove_dir(&docs_workspace)?;
            println!("  Removed empty workspace directory");
        }
    }

    Ok(())
}

pub fn list(repo: BeadsRepo, issue_id: &str) -> Result<()> {
    let documents = get_issue_documents(&repo, issue_id)?;

    if documents.is_empty() {
        println!("No documents attached to {}", issue_id);
        return Ok(());
    }

    println!("Documents for {}:", issue_id);
    println!();
    println!("{:<30} Hash (first 8 chars)", "Document");
    println!("{}", "-".repeat(50));

    let mut doc_list: Vec<_> = documents.iter().collect();
    doc_list.sort_by_key(|(name, _)| *name);

    for (name, hash) in doc_list {
        let short_hash = &hash[..8.min(hash.len())];
        println!("{:<30} {}", name, short_hash);
    }

    Ok(())
}
