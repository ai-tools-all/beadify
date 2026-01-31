pub mod blob;
pub mod db;
pub mod error;
pub mod log;
pub mod model;
pub mod repo;
pub mod tz;
pub mod utils;

pub use error::{Error, Result};
pub use model::{Event, Issue, IssueUpdate, Label, OpKind};
pub use repo::{find_repo, init_repo, BeadsRepo, BEADS_DIR, DB_FILE, EVENTS_FILE};

use db::{
    add_dependency, add_issue_label, apply_issue_update, create_schema,
    delete_issue as db_delete_issue, get_all_issues as db_get_all,
    get_all_labels as db_get_all_labels, get_dependencies as db_get_deps,
    get_dependents as db_get_dependents, get_issue as db_get_issue,
    get_issue_labels as db_get_issue_labels, get_issues_by_label as db_get_issues_by_label,
    get_open_dependencies as db_get_open_deps, remove_dependency, remove_issue_label, set_meta,
    update_text_references, upsert_issue,
};
use rusqlite::{Connection, OptionalExtension};

fn validate_label_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(Error::Io {
            action: "Label name cannot be empty".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty label name"),
        });
    }
    if name.len() > 50 {
        return Err(Error::Io {
            action: "Label name cannot exceed 50 characters".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "label name too long"),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::Io {
            action: "Label name can only contain alphanumeric characters, hyphens, and underscores"
                .to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid label name characters",
            ),
        });
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
    let created_at = chrono::Utc::now().to_rfc3339();
    let issue = Issue {
        id: issue_id.clone(),
        title: title.to_string(),
        kind: kind.to_string(),
        priority,
        status: "open".to_string(),
        created_at,
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
        return Err(Error::Io {
            action: format!("Issue '{}' not found", issue_id),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "issue not found"),
        });
    }
    if get_issue(repo, depends_on_id)?.is_none() {
        return Err(Error::Io {
            action: format!("Issue '{}' not found", depends_on_id),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "issue not found"),
        });
    }

    // Prevent self-dependency
    if issue_id == depends_on_id {
        return Err(Error::Io {
            action: "An issue cannot depend on itself".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "self-dependency"),
        });
    }

    let mut conn = repo.open_db()?;
    create_schema(&conn)?;

    let tx = conn.transaction()?;
    add_dependency(&tx, issue_id, depends_on_id)?;
    tx.commit()?;

    Ok(())
}

pub fn remove_issue_dependency(
    repo: &BeadsRepo,
    issue_id: &str,
    depends_on_id: &str,
) -> Result<()> {
    let mut conn = repo.open_db()?;
    create_schema(&conn)?;

    let tx = conn.transaction()?;
    remove_dependency(&tx, issue_id, depends_on_id)?;
    tx.commit()?;

    Ok(())
}

pub fn update_issue(repo: &BeadsRepo, id: &str, update: IssueUpdate) -> Result<Event> {
    if update.is_empty() {
        return Err(Error::empty_issue_update(id));
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
        return Err(Error::Io {
            action: format!("Issue '{}' not found", issue_id),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "issue not found"),
        });
    }

    let mut conn = repo.open_db()?;
    create_schema(&conn)?;

    // Check if label already exists by name
    let existing_label = {
        let mut stmt =
            conn.prepare("SELECT id, name, color, description FROM labels WHERE name = ?1")?;
        stmt.query_row(rusqlite::params![label_name], |row| {
            Ok(Label {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
            })
        })
        .optional()?
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
        let mut stmt = conn.prepare("SELECT id FROM labels WHERE name = ?1")?;
        stmt.query_row(rusqlite::params![label_name], |row| row.get(0))
            .map_err(|_| Error::Io {
                action: format!("Label '{}' not found", label_name),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "label not found"),
            })?
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
        let mut stmt = conn.prepare("SELECT id FROM labels WHERE name = ?1")?;
        stmt.query_row(rusqlite::params![label_name], |row| row.get(0))
            .map_err(|_| Error::Io {
                action: format!("Label '{}' not found", label_name),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "label not found"),
            })?
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
    let issue = get_issue(repo, issue_id)?.ok_or_else(|| Error::Io {
        action: format!("Issue '{}' not found", issue_id),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "issue not found"),
    })?;

    // Parse or create documents map
    let mut data = issue.data.unwrap_or_else(|| serde_json::json!({}));
    let documents = data
        .as_object_mut()
        .ok_or_else(|| Error::Io {
            action: "Issue data is not a JSON object".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, "not a JSON object"),
        })?
        .entry("documents")
        .or_insert_with(|| serde_json::json!({}));

    let documents_map = documents.as_object_mut().ok_or_else(|| Error::Io {
        action: "Documents field is not a JSON object".to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, "not a JSON object"),
    })?;

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
pub fn get_issue_documents(
    repo: &BeadsRepo,
    issue_id: &str,
) -> Result<std::collections::HashMap<String, String>> {
    use std::collections::HashMap;

    let issue = get_issue(repo, issue_id)?.ok_or_else(|| Error::Io {
        action: format!("Issue '{}' not found", issue_id),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "issue not found"),
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

/// Delete a single issue by setting status="deleted"
pub fn delete_issue(repo: &BeadsRepo, issue_id: &str) -> Result<DeleteResult> {
    let mut conn = repo.open_db()?;
    create_schema(&conn)?;

    // Check if issue exists and is not already deleted
    let issue = get_issue(repo, issue_id)?.ok_or_else(|| Error::Io {
        action: format!("Issue '{}' not found or already deleted", issue_id),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "issue not found or deleted"),
    })?;

    // Get dependents (issues that depend on this)
    let dependents = get_dependents(repo, issue_id)?;

    // Create update event with status="deleted"
    let update = IssueUpdate {
        status: Some("deleted".to_string()),
        ..Default::default()
    };

    let (event, new_offset) = log::append_update_event(repo, &conn, issue_id, &update)?;

    // Apply deletion in transaction
    let tx = conn.transaction()?;
    db_delete_issue(&tx, issue_id)?;
    let refs_updated = update_text_references(&tx, issue_id)?;
    set_meta(&tx, "last_event_id", event.event_id.clone())?;
    set_meta(&tx, "last_processed_offset", new_offset.to_string())?;
    tx.commit()?;

    Ok(DeleteResult {
        issue_id: issue_id.to_string(),
        title: issue.title,
        dependents,
        references_updated: refs_updated,
    })
}

/// Get issues that would be affected by deletion (for preview)
pub fn get_delete_impact(repo: &BeadsRepo, issue_id: &str, cascade: bool) -> Result<DeleteImpact> {
    let issue = get_issue(repo, issue_id)?.ok_or_else(|| Error::Io {
        action: format!("Issue '{}' not found or already deleted", issue_id),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "issue not found or deleted"),
    })?;

    let dependents = get_dependents(repo, issue_id)?;
    let text_refs = find_text_references(repo, issue_id)?;

    let mut all_issues = vec![ImpactItem {
        id: issue_id.to_string(),
        title: issue.title.clone(),
    }];

    if cascade {
        let recursive_deps = get_all_dependents_recursive_sorted(repo, issue_id)?;
        all_issues.extend(recursive_deps);
    }

    Ok(DeleteImpact {
        issues_to_delete: all_issues,
        blocked_issues: dependents,
        text_references: text_refs,
    })
}

/// Cascade delete: recursively delete all dependents
pub fn delete_issue_cascade(repo: &BeadsRepo, issue_id: &str) -> Result<Vec<DeleteResult>> {
    let mut results = Vec::new();

    // Get all dependents recursively (topologically sorted)
    let all_dependents = get_all_dependents_recursive_sorted(repo, issue_id)?;

    // Delete in reverse dependency order (leaves first)
    for dependent in all_dependents.iter().rev() {
        match delete_issue(repo, &dependent.id) {
            Ok(result) => results.push(result),
            Err(e) => {
                eprintln!("Warning: Failed to delete {}: {}", dependent.id, e);
            }
        }
    }

    // Finally delete the root issue
    results.push(delete_issue(repo, issue_id)?);

    Ok(results)
}

/// Batch delete multiple issues
pub fn delete_issues_batch(
    repo: &BeadsRepo,
    issue_ids: Vec<String>,
    cascade: bool,
) -> Result<BatchDeleteResult> {
    let mut successes = Vec::new();
    let mut failures = Vec::new();

    for issue_id in issue_ids {
        let result = if cascade {
            delete_issue_cascade(repo, &issue_id)
                .map(|results| results.into_iter().map(|r| r.issue_id).collect())
        } else {
            delete_issue(repo, &issue_id).map(|r| vec![r.issue_id])
        };

        match result {
            Ok(deleted_ids) => successes.extend(deleted_ids),
            Err(e) => {
                failures.push(DeleteFailure {
                    issue_id: issue_id.clone(),
                    error: e.to_string(),
                });
            }
        }
    }

    Ok(BatchDeleteResult {
        successes,
        failures,
    })
}

/// Find all text references to an issue (for preview)
fn find_text_references(repo: &BeadsRepo, issue_id: &str) -> Result<Vec<String>> {
    let conn = repo.open_db()?;
    let pattern = format!("%{}%", issue_id);

    let mut stmt = conn.prepare(
        r#"
        SELECT id FROM issues 
        WHERE title LIKE ?1 
           OR data LIKE ?1
        "#,
    )?;

    let refs = stmt
        .query_map(rusqlite::params![pattern], |row| row.get(0))?
        .collect::<std::result::Result<Vec<String>, _>>()?;

    Ok(refs)
}

/// Get all dependents recursively, topologically sorted
fn get_all_dependents_recursive_sorted(
    repo: &BeadsRepo,
    issue_id: &str,
) -> Result<Vec<ImpactItem>> {
    use std::collections::HashSet;

    let mut visited = HashSet::new();
    let mut result = Vec::new();

    fn visit(
        repo: &BeadsRepo,
        id: &str,
        visited: &mut HashSet<String>,
        result: &mut Vec<ImpactItem>,
    ) -> Result<()> {
        if visited.contains(id) {
            return Ok(()); // Already processed or cycle detected
        }
        visited.insert(id.to_string());

        let dependents = get_dependents(repo, id)?;
        for dep_id in dependents {
            visit(repo, &dep_id, visited, result)?;
        }

        // Add after processing dependents (post-order for deletion)
        if let Some(issue) = get_issue(repo, id)? {
            result.push(ImpactItem {
                id: id.to_string(),
                title: issue.title,
            });
        }

        Ok(())
    }

    let dependents = get_dependents(repo, issue_id)?;
    for dep_id in dependents {
        visit(repo, &dep_id, &mut visited, &mut result)?;
    }

    Ok(result)
}

#[derive(Debug, Clone)]
pub struct DeleteResult {
    pub issue_id: String,
    pub title: String,
    pub dependents: Vec<String>,
    pub references_updated: usize,
}

#[derive(Debug, Clone)]
pub struct ImpactItem {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct DeleteImpact {
    pub issues_to_delete: Vec<ImpactItem>,
    pub blocked_issues: Vec<String>,
    pub text_references: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteFailure {
    pub issue_id: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct BatchDeleteResult {
    pub successes: Vec<String>,
    pub failures: Vec<DeleteFailure>,
}

fn next_issue_id(conn: &mut Connection) -> Result<String> {
    let tx = conn.transaction()?;
    let prefix = db::get_meta(&tx, "id_prefix")?.ok_or_else(|| Error::Io {
        action: "missing repository configuration: id_prefix".to_string(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing config"),
    })?;
    let last_serial = db::get_meta(&tx, "last_issue_serial")?
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let next = last_serial + 1;
    db::set_meta(&tx, "last_issue_serial", next.to_string())?;
    tx.commit()?;
    Ok(format!("{prefix}-{next:03}"))
}
