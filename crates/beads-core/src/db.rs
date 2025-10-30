use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::{
    error::Result,
    model::{Issue, IssueUpdate, Label},
};

pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS issues (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            kind TEXT NOT NULL,
            priority INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'open'
        );

        CREATE TABLE IF NOT EXISTS dependencies (
            issue_id TEXT NOT NULL,
            depends_on_id TEXT NOT NULL,
            PRIMARY KEY (issue_id, depends_on_id),
            FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
            FOREIGN KEY (depends_on_id) REFERENCES issues(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS labels (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            color TEXT,
            description TEXT
        );

        CREATE TABLE IF NOT EXISTS issue_labels (
            issue_id TEXT NOT NULL,
            label_id TEXT NOT NULL,
            PRIMARY KEY (issue_id, label_id),
            FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
            FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS _meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

pub fn upsert_issue(tx: &Transaction<'_>, issue: &Issue) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO issues (id, title, kind, priority, status)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            kind = excluded.kind,
            priority = excluded.priority,
            status = excluded.status
        "#,
        params![
            issue.id,
            issue.title,
            issue.kind,
            issue.priority,
            issue.status
        ],
    )?;
    Ok(())
}

pub fn get_issue(conn: &Connection, id: &str) -> Result<Option<Issue>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, kind, priority, status FROM issues WHERE id = ?1",
    )?;
    let issue = stmt
        .query_row(params![id], |row| {
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: row.get(2)?,
                priority: row.get::<_, i64>(3)? as u32,
                status: row.get(4)?,
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
            })
        })
        .optional()?;
    Ok(issue)
}

pub fn get_all_issues(conn: &Connection) -> Result<Vec<Issue>> {
    let mut stmt =
        conn.prepare("SELECT id, title, kind, priority, status FROM issues ORDER BY id ASC")?;
    let issues = stmt
        .query_map([], |row| {
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: row.get(2)?,
                priority: row.get::<_, i64>(3)? as u32,
                status: row.get(4)?,
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(issues)
}

pub fn set_meta(tx: &Transaction<'_>, key: &str, value: String) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO _meta (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value
        "#,
        params![key, value],
    )?;
    Ok(())
}

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM _meta WHERE key = ?1")?;
    let value = stmt.query_row(params![key], |row| row.get(0)).optional()?;
    Ok(value)
}

pub fn add_dependency(tx: &Transaction<'_>, issue_id: &str, depends_on_id: &str) -> Result<()> {
    tx.execute(
        r#"
        INSERT OR IGNORE INTO dependencies (issue_id, depends_on_id)
        VALUES (?1, ?2)
        "#,
        params![issue_id, depends_on_id],
    )?;
    Ok(())
}

pub fn get_dependencies(conn: &Connection, issue_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT depends_on_id FROM dependencies WHERE issue_id = ?1 ORDER BY depends_on_id"
    )?;
    let deps = stmt
        .query_map(params![issue_id], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(deps)
}

pub fn remove_dependency(tx: &Transaction<'_>, issue_id: &str, depends_on_id: &str) -> Result<()> {
    let rows = tx.execute(
        "DELETE FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2",
        params![issue_id, depends_on_id],
    )?;
    if rows == 0 {
        return Err(crate::error::BeadsError::Custom(format!(
            "Dependency not found: {} does not depend on {}",
            issue_id, depends_on_id
        )));
    }
    Ok(())
}

pub fn clear_state(tx: &Transaction<'_>) -> Result<()> {
    tx.execute("DELETE FROM issues", [])?;
    Ok(())
}

pub fn apply_issue_update(tx: &Transaction<'_>, id: &str, update: &IssueUpdate) -> Result<()> {
    if let Some(title) = &update.title {
        tx.execute(
            "UPDATE issues SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
    }
    if let Some(kind) = &update.kind {
        tx.execute(
            "UPDATE issues SET kind = ?1 WHERE id = ?2",
            params![kind, id],
        )?;
    }
    if let Some(priority) = update.priority {
        tx.execute(
            "UPDATE issues SET priority = ?1 WHERE id = ?2",
            params![priority as i64, id],
        )?;
    }
    if let Some(status) = &update.status {
        tx.execute(
            "UPDATE issues SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
    }
    Ok(())
}

pub fn create_label(tx: &Transaction<'_>, label: &Label) -> Result<()> {
    tx.execute(
        "INSERT INTO labels (id, name, color, description) VALUES (?1, ?2, ?3, ?4)",
        params![label.id, label.name, label.color, label.description],
    )?;
    Ok(())
}

pub fn get_label(conn: &Connection, id: &str) -> Result<Option<Label>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, description FROM labels WHERE id = ?1",
    )?;
    let label = stmt
        .query_row(params![id], |row| {
            Ok(Label {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
            })
        })
        .optional()?;
    Ok(label)
}

pub fn get_all_labels(conn: &Connection) -> Result<Vec<Label>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, description FROM labels ORDER BY name ASC"
    )?;
    let labels = stmt
        .query_map([], |row| {
            Ok(Label {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(labels)
}

pub fn delete_label(tx: &Transaction<'_>, id: &str) -> Result<()> {
    let rows = tx.execute("DELETE FROM labels WHERE id = ?1", params![id])?;
    if rows == 0 {
        return Err(crate::error::BeadsError::Custom(format!(
            "Label not found: {}",
            id
        )));
    }
    Ok(())
}

pub fn add_issue_label(tx: &Transaction<'_>, issue_id: &str, label_id: &str) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO issue_labels (issue_id, label_id) VALUES (?1, ?2)",
        params![issue_id, label_id],
    )?;
    Ok(())
}

pub fn remove_issue_label(tx: &Transaction<'_>, issue_id: &str, label_id: &str) -> Result<()> {
    let rows = tx.execute(
        "DELETE FROM issue_labels WHERE issue_id = ?1 AND label_id = ?2",
        params![issue_id, label_id],
    )?;
    if rows == 0 {
        return Err(crate::error::BeadsError::Custom(format!(
            "Issue label not found: {} on {}",
            label_id, issue_id
        )));
    }
    Ok(())
}

pub fn get_issue_labels(conn: &Connection, issue_id: &str) -> Result<Vec<Label>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT l.id, l.name, l.color, l.description
        FROM labels l
        JOIN issue_labels il ON l.id = il.label_id
        WHERE il.issue_id = ?1
        ORDER BY l.name ASC
        "#
    )?;
    let labels = stmt
        .query_map(params![issue_id], |row| {
            Ok(Label {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(labels)
}

pub fn get_issues_by_label(conn: &Connection, label_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT issue_id FROM issue_labels WHERE label_id = ?1 ORDER BY issue_id"
    )?;
    let issues = stmt
        .query_map(params![label_id], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(issues)
}
