use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::{
    error::Result,
    model::{Issue, IssueUpdate},
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
