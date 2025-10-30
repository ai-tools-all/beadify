pub mod db;
pub mod error;
pub mod log;
pub mod model;
pub mod repo;

pub use error::{BeadsError, Result};
pub use model::{Event, Issue, IssueUpdate, OpKind};
pub use repo::{find_repo, init_repo, BeadsRepo, BEADS_DIR, DB_FILE, EVENTS_FILE};

use db::{add_dependency, apply_issue_update, create_schema, get_all_issues as db_get_all, get_dependencies as db_get_deps, get_issue as db_get_issue, remove_dependency, set_meta, upsert_issue};
use rusqlite::Connection;

pub fn create_issue(
    repo: &BeadsRepo,
    title: &str,
    kind: &str,
    priority: u32,
    depends_on: Vec<String>,
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
