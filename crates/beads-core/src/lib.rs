pub mod blob;
pub mod db;
pub mod error;
pub mod log;
pub mod model;
pub mod repo;

pub use error::{BeadsError, Result};
pub use model::{Event, Issue, IssueUpdate, Label, OpKind};
pub use repo::{find_repo, init_repo, BeadsRepo, BEADS_DIR, DB_FILE, EVENTS_FILE};

use db::{add_dependency, add_issue_label, apply_issue_update, create_schema, delete_issue as db_delete_issue, get_all_issues as db_get_all, get_all_labels as db_get_all_labels, get_dependencies as db_get_deps, get_dependents as db_get_dependents, get_open_dependencies as db_get_open_deps, get_issue as db_get_issue, get_issue_labels as db_get_issue_labels, get_issues_by_label as db_get_issues_by_label, is_issue_deleted, remove_dependency, remove_issue_label, set_meta, update_text_references, upsert_issue};
use rusqlite::{Connection, OptionalExtension};

fn validate_label_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(BeadsError::Custom("Label name cannot be empty".to_string()));
    }
    if name.len() > 50 {
        return Err(BeadsError::Custom(
            "Label name cannot exceed 50 characters".to_string(),
        ));
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(BeadsError::Custom(
            "Label name can only contain alphanumeric characters, hyphens, and underscores".to_string(),
        ));
    }
    Ok(())
}

pub fn create_issue(
    repo: &BeadsRepo,
    title: &str,
    kind: &str,
    priority: u32,
    depends_on: Vec<String>,
) -> Result<Event> {
    create_issue_with_data(repo, title, kind, priority, depends_on, None)
}

pub fn create_issue_with_data(
    repo: &BeadsRepo,
    title: &str,
    kind: &str,
    priority: u32,
    depends_on: Vec<String>,
    data: Option<serde_json::Value>,
) -> Result<Event> {
    let mut conn = repo.open_db()?;
    create_schema(&conn)?;

    let issue_id = next_issue_id(&mut conn)?;
    let issue = Issue {
        id: issue_id.clone(),
        title: title.to_string(),
        kind: kind.to_string(),
        priority,
        status: "open".to_string(),
        description: None,
        design: None,
        acceptance_criteria: None,
        notes: None,
        data,
    };

    let (event, new_offset) = log::append_create_event(repo, &conn, &issue)?;

    let tx = conn.transaction()?;
    upsert_issue(&tx, &issue)?;
    
    // Add dependencies
    for dep_id in depends_on {
        add_dependency(&tx, &issue_id, &dep_id)?;
    }
    
    set_meta(&tx, "last_event_id", event.event_id.clone())?;
    set_meta(&tx, "last_processed_offset", new_offset.to_string())?;
    tx.commit()?;

    Ok(event)
}

pub fn get_dependencies(repo: &BeadsRepo, issue_id: &str) -> Result<Vec<String>> {
    let conn = repo.open_db()?;
    create_schema(&conn)?;
    db_get_deps(&conn, issue_id)
}

pub fn get_dependents(repo: &BeadsRepo, issue_id: &str) -> Result<Vec<String>> {
    let conn = repo.open_db()?;
    create_schema(&conn)?;
    db_get_dependents(&conn, issue_id)
}

pub fn get_open_dependencies(repo: &BeadsRepo, issue_id: &str) -> Result<Vec<String>> {
    let conn = repo.open_db()?;
    create_schema(&conn)?;
    db_get_open_deps(&conn, issue_id)
}

pub fn add_issue_dependency(repo: &BeadsRepo, issue_id: &str, depends_on_id: &str) -> Result<()> {
    // Validate both issues exist
    if get_issue(repo, issue_id)?.is_none() {
        return Err(BeadsError::Custom(format!("Issue '{}' not found", issue_id)));
    }
    if get_issue(repo, depends_on_id)?.is_none() {
        return Err(BeadsError::Custom(format!("Issue '{}' not found", depends_on_id)));
    }
    
    // Prevent self-dependency
    if issue_id == depends_on_id {
        return Err(BeadsError::Custom("An issue cannot depend on itself".to_string()));
    }
    
    let mut conn = repo.open_db()?;
    create_schema(&conn)?;
    
    let tx = conn.transaction()?;
    add_dependency(&tx, issue_id, depends_on_id)?;
    tx.commit()?;
    
    Ok(())
}

pub fn remove_issue_dependency(repo: &BeadsRepo, issue_id: &str, depends_on_id: &str) -> Result<()> {
    let mut conn = repo.open_db()?;
    create_schema(&conn)?;
    
    let tx = conn.transaction()?;
    remove_dependency(&tx, issue_id, depends_on_id)?;
    tx.commit()?;
    
    Ok(())
}

pub fn update_issue(repo: &BeadsRepo, id: &str, update: IssueUpdate) -> Result<Event> {
    if update.is_empty() {
        return Err(BeadsError::EmptyUpdate);
    }

    let mut conn = repo.open_db()?;
    create_schema(&conn)?;

    let (event, new_offset) = log::append_update_event(repo, &conn, id, &update)?;

    let tx = conn.transaction()?;
    apply_issue_update(&tx, id, &update)?;
    set_meta(&tx, "last_event_id", event.event_id.clone())?;
    set_meta(&tx, "last_processed_offset", new_offset.to_string())?;
    tx.commit()?;

    Ok(event)
}

pub fn get_issue(repo: &BeadsRepo, id: &str) -> Result<Option<Issue>> {
    let conn = repo.open_db()?;
    create_schema(&conn)?;
    db_get_issue(&conn, id)
}

pub fn get_all_issues(repo: &BeadsRepo) -> Result<Vec<Issue>> {
    let conn = repo.open_db()?;
    create_schema(&conn)?;
    db_get_all(&conn)
}

pub fn sync_repo(repo: &BeadsRepo, full: bool) -> Result<usize> {
    let mut conn = repo.open_db()?;
    create_schema(&conn)?;

    let (applied, _, _) = (if full {
        log::apply_all_events(repo, &mut conn)
    } else {
        log::apply_incremental(repo, &mut conn)
    })?;

    Ok(applied)
}

pub fn add_label_to_issue(repo: &BeadsRepo, issue_id: &str, label_name: &str) -> Result<Label> {
    // Validate label name format
    validate_label_name(label_name)?;
    
    // Validate issue exists
    if get_issue(repo, issue_id)?.is_none() {
        return Err(BeadsError::Custom(format!("Issue '{}' not found", issue_id)));
    }

    let mut conn = repo.open_db()?;
    create_schema(&conn)?;
    
    // Check if label already exists by name
    let existing_label = {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, description FROM labels WHERE name = ?1"
        )?;
        stmt.query_row(rusqlite::params![label_name], |row| {
            Ok(Label {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
            })
        }).optional()?
    };
    
    // Create label if doesn't exist
    let label = if let Some(label) = existing_label {
        label
    } else {
        let label_id = ulid::Ulid::new().to_string();
        let new_label = Label {
            id: label_id,
            name: label_name.to_string(),
            color: None,
            description: None,
        };
        
        let tx = conn.transaction()?;
        db::create_label(&tx, &new_label)?;
        tx.commit()?;
        
        new_label
    };
    
    // Add label to issue
    let tx = conn.transaction()?;
    add_issue_label(&tx, issue_id, &label.id)?;
    tx.commit()?;
    
    Ok(label)
}

pub fn remove_label_from_issue(repo: &BeadsRepo, issue_id: &str, label_name: &str) -> Result<()> {
    let mut conn = repo.open_db()?;
    create_schema(&conn)?;
    
    // Find label by name
    let label_id: String = {
        let mut stmt = conn.prepare(
            "SELECT id FROM labels WHERE name = ?1"
        )?;
        stmt.query_row(rusqlite::params![label_name], |row| row.get(0))
            .map_err(|_| BeadsError::Custom(format!("Label '{}' not found", label_name)))?
    };
    
    let tx = conn.transaction()?;
    remove_issue_label(&tx, issue_id, &label_id)?;
    tx.commit()?;
    
    Ok(())
}

pub fn get_issue_labels(repo: &BeadsRepo, issue_id: &str) -> Result<Vec<Label>> {
    let conn = repo.open_db()?;
    create_schema(&conn)?;
    db_get_issue_labels(&conn, issue_id)
}

pub fn get_all_labels(repo: &BeadsRepo) -> Result<Vec<Label>> {
    let conn = repo.open_db()?;
    create_schema(&conn)?;
    db_get_all_labels(&conn)
}

pub fn get_issues_by_label(repo: &BeadsRepo, label_name: &str) -> Result<Vec<String>> {
    let conn = repo.open_db()?;
    create_schema(&conn)?;
    
    // Find label by name
    let label_id: String = {
        let mut stmt = conn.prepare(
            "SELECT id FROM labels WHERE name = ?1"
        )?;
        stmt.query_row(rusqlite::params![label_name], |row| row.get(0))
            .map_err(|_| BeadsError::Custom(format!("Label '{}' not found", label_name)))?
    };
    
    db_get_issues_by_label(&conn, &label_id)
}

/// Add or update a document attachment to an issue.
/// The document is stored in the blob store and its hash is added to the issue's data field.
pub fn add_document_to_issue(
    repo: &BeadsRepo,
    issue_id: &str,
    doc_name: &str,
    content: &[u8],
) -> Result<()> {
    // Write content to blob store
    let hash = blob::write_blob(repo, content)?;

    // Get current issue to retrieve existing data
    let issue = get_issue(repo, issue_id)?.ok_or_else(|| {
        BeadsError::Custom(format!("Issue '{}' not found", issue_id))
    })?;

    // Parse or create documents map
    let mut data = issue.data.unwrap_or_else(|| serde_json::json!({}));
    let documents = data
        .as_object_mut()
        .ok_or_else(|| BeadsError::Custom("Issue data is not a JSON object".to_string()))?
        .entry("documents")
        .or_insert_with(|| serde_json::json!({}));

    let documents_map = documents
        .as_object_mut()
        .ok_or_else(|| BeadsError::Custom("Documents field is not a JSON object".to_string()))?;

    // Add or update document hash
    documents_map.insert(doc_name.to_string(), serde_json::json!(hash));

    // Update issue with new data
    let update = IssueUpdate {
        data: Some(data),
        ..Default::default()
    };

    update_issue(repo, issue_id, update)?;

    Ok(())
}

/// Get all documents attached to an issue.
/// Returns a map of document name to blob hash.
pub fn get_issue_documents(repo: &BeadsRepo, issue_id: &str) -> Result<std::collections::HashMap<String, String>> {
    use std::collections::HashMap;

    let issue = get_issue(repo, issue_id)?.ok_or_else(|| {
        BeadsError::Custom(format!("Issue '{}' not found", issue_id))
    })?;

    let mut documents = HashMap::new();

    if let Some(data) = issue.data {
        if let Some(docs_value) = data.get("documents") {
            if let Some(docs_obj) = docs_value.as_object() {
                for (name, hash_value) in docs_obj {
                    if let Some(hash) = hash_value.as_str() {
                        documents.insert(name.clone(), hash.to_string());
                    }
                }
            }
        }
    }

    Ok(documents)
}

fn next_issue_id(conn: &mut Connection) -> Result<String> {
    let tx = conn.transaction()?;
    let prefix = db::get_meta(&tx, "id_prefix")?.ok_or(BeadsError::MissingConfig("id_prefix"))?;
    let last_serial = db::get_meta(&tx, "last_issue_serial")?
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let next = last_serial + 1;
    db::set_meta(&tx, "last_issue_serial", next.to_string())?;
    tx.commit()?;
    Ok(format!("{prefix}-{next:03}"))
}
