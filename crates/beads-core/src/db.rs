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
            status TEXT NOT NULL DEFAULT 'open',
            description TEXT,
            design TEXT,
            acceptance_criteria TEXT,
            notes TEXT,
            data TEXT
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

    // Add columns to existing table if they don't exist
    let mut stmt = conn.prepare("PRAGMA table_info(issues)")?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if !columns.contains(&"data".to_string()) {
        conn.execute("ALTER TABLE issues ADD COLUMN data TEXT", [])?;
    }
    if !columns.contains(&"description".to_string()) {
        conn.execute("ALTER TABLE issues ADD COLUMN description TEXT", [])?;
    }
    if !columns.contains(&"design".to_string()) {
        conn.execute("ALTER TABLE issues ADD COLUMN design TEXT", [])?;
    }
    if !columns.contains(&"acceptance_criteria".to_string()) {
        conn.execute("ALTER TABLE issues ADD COLUMN acceptance_criteria TEXT", [])?;
    }
    if !columns.contains(&"notes".to_string()) {
        conn.execute("ALTER TABLE issues ADD COLUMN notes TEXT", [])?;
    }

    Ok(())
}

pub fn upsert_issue(tx: &Transaction<'_>, issue: &Issue) -> Result<()> {
    let data = issue.data.as_ref().map(|v| v.to_string());
    tx.execute(
        r#"
        INSERT INTO issues (id, title, kind, priority, status, description, design, acceptance_criteria, notes, data)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            kind = excluded.kind,
            priority = excluded.priority,
            status = excluded.status,
            description = excluded.description,
            design = excluded.design,
            acceptance_criteria = excluded.acceptance_criteria,
            notes = excluded.notes,
            data = excluded.data
        "#,
        params![
            issue.id,
            issue.title,
            issue.kind,
            issue.priority,
            issue.status,
            issue.description,
            issue.design,
            issue.acceptance_criteria,
            issue.notes,
            data
        ],
    )?;
    Ok(())
}

pub fn get_issue(conn: &Connection, id: &str) -> Result<Option<Issue>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, kind, priority, status, description, design, acceptance_criteria, notes, data FROM issues WHERE id = ?1",
    )?;
    let issue = stmt
        .query_row(params![id], |row| {
            let data_str: Option<String> = row.get(9)?;
            let data = data_str.and_then(|s| serde_json::from_str(&s).ok());
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: row.get(2)?,
                priority: row.get::<_, i64>(3)? as u32,
                status: row.get(4)?,
                description: row.get(5)?,
                design: row.get(6)?,
                acceptance_criteria: row.get(7)?,
                notes: row.get(8)?,
                data,
            })
        })
        .optional()?;
    Ok(issue)
}

pub fn get_all_issues(conn: &Connection) -> Result<Vec<Issue>> {
    let mut stmt =
        conn.prepare("SELECT id, title, kind, priority, status, description, design, acceptance_criteria, notes, data FROM issues ORDER BY id ASC")?;
    let issues = stmt
        .query_map([], |row| {
            let data_str: Option<String> = row.get(9)?;
            let data = data_str.and_then(|s| serde_json::from_str(&s).ok());
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: row.get(2)?,
                priority: row.get::<_, i64>(3)? as u32,
                status: row.get(4)?,
                description: row.get(5)?,
                design: row.get(6)?,
                acceptance_criteria: row.get(7)?,
                notes: row.get(8)?,
                data,
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

pub fn get_open_dependencies(conn: &Connection, issue_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT d.depends_on_id 
        FROM dependencies d
        JOIN issues i ON d.depends_on_id = i.id
        WHERE d.issue_id = ?1 AND i.status != 'closed'
        ORDER BY d.depends_on_id
        "#
    )?;
    let deps = stmt
        .query_map(params![issue_id], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(deps)
}

pub fn get_dependents(conn: &Connection, issue_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT issue_id FROM dependencies WHERE depends_on_id = ?1 ORDER BY issue_id"
    )?;
    let dependents = stmt
        .query_map(params![issue_id], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(dependents)
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
    tx.execute("DELETE FROM _meta", [])?;
    Ok(())
}

/// Delete issue and all related data from SQLite
/// Note: This is for removing from the local cache only
/// The issue still exists in events.jsonl with status="deleted"
pub fn delete_issue(tx: &Transaction<'_>, issue_id: &str) -> Result<()> {
    // Dependencies and issue_labels automatically deleted by CASCADE
    tx.execute("DELETE FROM issues WHERE id = ?1", params![issue_id])?;
    Ok(())
}

/// Update text references to deleted issues
/// Replace "bd-001" with "[deleted:bd-001]" in all text fields
pub fn update_text_references(tx: &Transaction<'_>, deleted_id: &str) -> Result<usize> {
    let replacement = format!("[deleted:{}]", deleted_id);
    let search_pattern = format!("%{}%", deleted_id);
    
    let mut total_updated = 0;
    
    // Update title field
    let updated = tx.execute(
        "UPDATE issues SET title = REPLACE(title, ?1, ?2) WHERE title LIKE ?3",
        params![deleted_id, &replacement, &search_pattern],
    )?;
    total_updated += updated;
    
    // Update data field (JSON text)
    let updated = tx.execute(
        "UPDATE issues SET data = REPLACE(data, ?1, ?2) WHERE data IS NOT NULL AND data LIKE ?3",
        params![deleted_id, &replacement, &search_pattern],
    )?;
    total_updated += updated;
    
    Ok(total_updated)
}

/// Check if an issue is deleted (by checking if it exists in SQLite)
/// Issues with status="deleted" are not loaded into SQLite
pub fn is_issue_deleted(conn: &Connection, issue_id: &str) -> Result<bool> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM issues WHERE id = ?1",
            params![issue_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    
    Ok(!exists)
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
    if let Some(description) = &update.description {
        tx.execute(
            "UPDATE issues SET description = ?1 WHERE id = ?2",
            params![description, id],
        )?;
    }
    if let Some(design) = &update.design {
        tx.execute(
            "UPDATE issues SET design = ?1 WHERE id = ?2",
            params![design, id],
        )?;
    }
    if let Some(acceptance_criteria) = &update.acceptance_criteria {
        tx.execute(
            "UPDATE issues SET acceptance_criteria = ?1 WHERE id = ?2",
            params![acceptance_criteria, id],
        )?;
    }
    if let Some(notes) = &update.notes {
        tx.execute(
            "UPDATE issues SET notes = ?1 WHERE id = ?2",
            params![notes, id],
        )?;
    }
    if let Some(data) = &update.data {
        let data_json = serde_json::to_string(data)?;
        tx.execute(
            "UPDATE issues SET data = ?1 WHERE id = ?2",
            params![data_json, id],
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        create_schema(&conn)?;
        Ok(conn)
    }

    #[test]
    fn test_create_and_get_label() -> Result<()> {
        let mut conn = setup_test_db()?;
        let tx = conn.transaction()?;

        let label = Label {
            id: "label-1".to_string(),
            name: "backend".to_string(),
            color: Some("#ff0000".to_string()),
            description: Some("Backend tasks".to_string()),
        };

        create_label(&tx, &label)?;
        tx.commit()?;

        let retrieved = get_label(&conn, "label-1")?;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "backend");
        assert_eq!(retrieved.color, Some("#ff0000".to_string()));
        assert_eq!(retrieved.description, Some("Backend tasks".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_nonexistent_label() -> Result<()> {
        let conn = setup_test_db()?;
        let retrieved = get_label(&conn, "nonexistent")?;
        assert!(retrieved.is_none());
        Ok(())
    }

    #[test]
    fn test_get_all_labels() -> Result<()> {
        let mut conn = setup_test_db()?;
        let tx = conn.transaction()?;

        for i in 0..3 {
            let label = Label {
                id: format!("label-{}", i),
                name: format!("label-{}", i),
                color: None,
                description: None,
            };
            create_label(&tx, &label)?;
        }
        tx.commit()?;

        let labels = get_all_labels(&conn)?;
        assert_eq!(labels.len(), 3);
        assert!(labels.iter().any(|l| l.name == "label-0"));
        assert!(labels.iter().any(|l| l.name == "label-1"));
        assert!(labels.iter().any(|l| l.name == "label-2"));

        Ok(())
    }

    #[test]
    fn test_add_and_get_issue_labels() -> Result<()> {
        let mut conn = setup_test_db()?;
        let tx = conn.transaction()?;

        // Create an issue first
        let issue = Issue {
            id: "issue-1".to_string(),
            title: "Test".to_string(),
            kind: "task".to_string(),
            priority: 1,
            status: "open".to_string(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            data: None,
        };
        upsert_issue(&tx, &issue)?;

        let label = Label {
            id: "label-1".to_string(),
            name: "backend".to_string(),
            color: None,
            description: None,
        };
        create_label(&tx, &label)?;
        add_issue_label(&tx, "issue-1", "label-1")?;
        tx.commit()?;

        let labels = get_issue_labels(&conn, "issue-1")?;
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "backend");

        Ok(())
    }

    #[test]
    fn test_multiple_labels_on_issue() -> Result<()> {
        let mut conn = setup_test_db()?;
        let tx = conn.transaction()?;

        let issue = Issue {
            id: "issue-1".to_string(),
            title: "Test".to_string(),
            kind: "task".to_string(),
            priority: 1,
            status: "open".to_string(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            data: None,
        };
        upsert_issue(&tx, &issue)?;

        let labels = vec![
            Label {
                id: "label-1".to_string(),
                name: "backend".to_string(),
                color: None,
                description: None,
            },
            Label {
                id: "label-2".to_string(),
                name: "urgent".to_string(),
                color: None,
                description: None,
            },
            Label {
                id: "label-3".to_string(),
                name: "database".to_string(),
                color: None,
                description: None,
            },
        ];

        for label in labels {
            create_label(&tx, &label)?;
            add_issue_label(&tx, "issue-1", &label.id)?;
        }
        tx.commit()?;

        let retrieved = get_issue_labels(&conn, "issue-1")?;
        assert_eq!(retrieved.len(), 3);
        assert!(retrieved.iter().any(|l| l.name == "backend"));
        assert!(retrieved.iter().any(|l| l.name == "urgent"));
        assert!(retrieved.iter().any(|l| l.name == "database"));

        Ok(())
    }

    #[test]
    fn test_remove_issue_label() -> Result<()> {
        let mut conn = setup_test_db()?;
        let tx = conn.transaction()?;

        let issue = Issue {
            id: "issue-1".to_string(),
            title: "Test".to_string(),
            kind: "task".to_string(),
            priority: 1,
            status: "open".to_string(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            data: None,
        };
        upsert_issue(&tx, &issue)?;

        let label = Label {
            id: "label-1".to_string(),
            name: "backend".to_string(),
            color: None,
            description: None,
        };
        create_label(&tx, &label)?;
        add_issue_label(&tx, "issue-1", "label-1")?;
        tx.commit()?;

        let mut tx2 = conn.transaction()?;
        remove_issue_label(&mut tx2, "issue-1", "label-1")?;
        tx2.commit()?;

        let labels = get_issue_labels(&conn, "issue-1")?;
        assert_eq!(labels.len(), 0);

        Ok(())
    }

    #[test]
    fn test_remove_nonexistent_issue_label_fails() -> Result<()> {
        let mut conn = setup_test_db()?;
        let mut tx = conn.transaction()?;

        let result = remove_issue_label(&mut tx, "issue-1", "label-1");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_delete_label() -> Result<()> {
        let mut conn = setup_test_db()?;
        let tx = conn.transaction()?;

        let label = Label {
            id: "label-1".to_string(),
            name: "backend".to_string(),
            color: None,
            description: None,
        };
        create_label(&tx, &label)?;
        tx.commit()?;

        let mut tx2 = conn.transaction()?;
        delete_label(&mut tx2, "label-1")?;
        tx2.commit()?;

        let retrieved = get_label(&conn, "label-1")?;
        assert!(retrieved.is_none());

        Ok(())
    }

    #[test]
    fn test_delete_nonexistent_label_fails() -> Result<()> {
        let mut conn = setup_test_db()?;
        let mut tx = conn.transaction()?;

        let result = delete_label(&mut tx, "label-1");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_get_issues_by_label() -> Result<()> {
        let mut conn = setup_test_db()?;
        let tx = conn.transaction()?;

        for i in 0..3 {
            let issue = Issue {
                id: format!("issue-{}", i),
                title: "Test".to_string(),
                kind: "task".to_string(),
                priority: 1,
                status: "open".to_string(),
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
                data: None,
            };
            upsert_issue(&tx, &issue)?;
        }

        let label = Label {
            id: "label-1".to_string(),
            name: "backend".to_string(),
            color: None,
            description: None,
        };
        create_label(&tx, &label)?;

        for i in 0..3 {
            add_issue_label(&tx, &format!("issue-{}", i), "label-1")?;
        }
        tx.commit()?;

        let issues = get_issues_by_label(&conn, "label-1")?;
        assert_eq!(issues.len(), 3);
        assert!(issues.contains(&"issue-0".to_string()));
        assert!(issues.contains(&"issue-1".to_string()));
        assert!(issues.contains(&"issue-2".to_string()));

        Ok(())
    }

    #[test]
    fn test_label_ignore_duplicate_add() -> Result<()> {
        let mut conn = setup_test_db()?;
        let tx = conn.transaction()?;

        let issue = Issue {
            id: "issue-1".to_string(),
            title: "Test".to_string(),
            kind: "task".to_string(),
            priority: 1,
            status: "open".to_string(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            data: None,
        };
        upsert_issue(&tx, &issue)?;

        let label = Label {
            id: "label-1".to_string(),
            name: "backend".to_string(),
            color: None,
            description: None,
        };
        create_label(&tx, &label)?;
        add_issue_label(&tx, "issue-1", "label-1")?;
        add_issue_label(&tx, "issue-1", "label-1")?;
        tx.commit()?;

        let labels = get_issue_labels(&conn, "issue-1")?;
        assert_eq!(labels.len(), 1);

        Ok(())
    }

    #[test]
    fn test_get_empty_issue_labels() -> Result<()> {
        let mut conn = setup_test_db()?;
        let tx = conn.transaction()?;

        let issue = Issue {
            id: "issue-1".to_string(),
            title: "Test".to_string(),
            kind: "task".to_string(),
            priority: 1,
            status: "open".to_string(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            data: None,
        };
        upsert_issue(&tx, &issue)?;
        tx.commit()?;

        let labels = get_issue_labels(&conn, "issue-1")?;
        assert_eq!(labels.len(), 0);

        Ok(())
    }
}
