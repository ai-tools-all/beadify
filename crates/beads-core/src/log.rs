use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::str::FromStr;

use chrono::Utc;
use rusqlite::{Connection, Transaction};
use serde::Deserialize;
use serde_json::{json, Value};
use ulid::Ulid;

use crate::{
    db,
    error::Result,
    model::{Event, Issue, IssueUpdate, OpKind},
    repo::BeadsRepo,
};

pub fn append_create_event(
    repo: &BeadsRepo,
    conn: &Connection,
    issue: &Issue,
) -> Result<(Event, u64)> {
    let data = if let Some(issue_data) = &issue.data {
        let mut data_obj = issue_data.clone();
        if let Some(obj) = data_obj.as_object_mut() {
            obj.insert("title".to_string(), json!(issue.title));
            obj.insert("status".to_string(), json!(issue.status));
        }
        data_obj
    } else {
        json!({
            "title": issue.title,
            "kind": issue.kind,
            "priority": issue.priority,
            "status": issue.status,
        })
    };

    let event = build_event(conn, issue.id.clone(), OpKind::Create, data)?;
    let offset = write_event(repo, &event)?;

    Ok((event, offset))
}

pub fn append_update_event(
    repo: &BeadsRepo,
    conn: &Connection,
    issue_id: &str,
    update: &IssueUpdate,
) -> Result<(Event, u64)> {
    let data = serde_json::to_value(update.clone())?;
    let event = build_event(conn, issue_id.to_string(), OpKind::Update, data)?;
    let offset = write_event(repo, &event)?;
    Ok((event, offset))
}

pub fn apply_all_events(
    repo: &BeadsRepo,
    conn: &mut Connection,
) -> Result<(usize, u64, Option<String>)> {
    let file = match File::open(repo.log_path()) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok((0, 0, None));
        }
        Err(err) => return Err(err.into()),
    };

    let reader = BufReader::new(file);

    let mut events = Vec::new();
    let mut offset = 0u64;

    for line in reader.lines() {
        let line = line?;
        let line_len = line.as_bytes().len() as u64 + 1;
        offset += line_len;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let event: Event = serde_json::from_str(trimmed)?;
        events.push(event);
    }

    events.sort_by(|a, b| a.event_id.cmp(&b.event_id));

    let tx = conn.transaction()?;
    db::clear_state(&tx)?;

    let mut last_event = None;
    for event in &events {
        apply_event(&tx, event)?;
        last_event = Some(event.event_id.clone());
    }

    if let Some(ref event_id) = last_event {
        db::set_meta(&tx, "last_event_id", event_id.clone())?;
    }
    db::set_meta(&tx, "last_processed_offset", offset.to_string())?;
    tx.commit()?;

    Ok((events.len(), offset, last_event))
}

pub fn apply_incremental(
    repo: &BeadsRepo,
    conn: &mut Connection,
) -> Result<(usize, u64, Option<String>)> {
    let start_offset = db::get_meta(conn, "last_processed_offset")?
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut file = match File::open(repo.log_path()) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok((0, start_offset, db::get_meta(conn, "last_event_id")?));
        }
        Err(err) => return Err(err.into()),
    };

    file.seek(SeekFrom::Start(start_offset))?;
    let reader = BufReader::new(file);

    let mut events = Vec::new();
    let mut offset = start_offset;

    for line in reader.lines() {
        let line = line?;
        let line_len = line.as_bytes().len() as u64 + 1;
        offset += line_len;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let event: Event = serde_json::from_str(trimmed)?;
        events.push(event);
    }

    if events.is_empty() {
        return Ok((0, offset, db::get_meta(conn, "last_event_id")?));
    }

    events.sort_by(|a, b| a.event_id.cmp(&b.event_id));

    let mut last_ulid = db::get_meta(conn, "last_event_id")?
        .map(|s| Ulid::from_string(&s))
        .transpose()?;

    for event in &events {
        let current = Ulid::from_string(&event.event_id)?;
        if let Some(last) = last_ulid {
            if current <= last {
                return apply_all_events(repo, conn);
            }
        }
        last_ulid = Some(current);
    }

    let tx = conn.transaction()?;
    let mut last_event = None;
    for event in &events {
        apply_event(&tx, event)?;
        last_event = Some(event.event_id.clone());
    }

    if let Some(ref event_id) = last_event {
        db::set_meta(&tx, "last_event_id", event_id.clone())?;
    }
    db::set_meta(&tx, "last_processed_offset", offset.to_string())?;
    tx.commit()?;

    Ok((events.len(), offset, last_event))
}

fn apply_event(tx: &Transaction<'_>, event: &Event) -> Result<()> {
    match event.op {
        OpKind::Create => {
            #[derive(Deserialize)]
            struct CreatePayload {
                title: String,
                kind: String,
                priority: u32,
                #[serde(default)]
                status: Option<String>,
            }

            let payload: CreatePayload = serde_json::from_value(event.data.clone())?;
            let status = payload.status.unwrap_or_else(|| "open".to_string());
            
            // Skip issues with status="deleted" - don't insert into SQLite
            if status == "deleted" {
                return Ok(());
            }
            
            let issue = Issue {
                id: event.id.clone(),
                title: payload.title,
                kind: payload.kind,
                priority: payload.priority,
                status,
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
                data: Some(event.data.clone()),
            };
            db::upsert_issue(tx, &issue)
        }
        OpKind::Update => {
            let update: IssueUpdate = serde_json::from_value(event.data.clone())?;
            
            // Check if this is a deletion (status="deleted")
            if let Some(ref status) = update.status {
                if status == "deleted" {
                    // Remove issue from SQLite
                    db::delete_issue(tx, &event.id)?;
                    // Update text references in remaining issues
                    db::update_text_references(tx, &event.id)?;
                    return Ok(());
                }
            }
            
            // Normal update handling
            db::apply_issue_update(tx, &event.id, &update)
        }
        _ => Ok(()),
    }
}

fn build_event(conn: &Connection, id: String, op: OpKind, data: Value) -> Result<Event> {
    let event_id = next_event_ulid(conn)?;
    Ok(Event {
        event_id: event_id.to_string(),
        ts: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        op,
        id,
        actor: current_actor(),
        data,
    })
}

fn write_event(repo: &BeadsRepo, event: &Event) -> Result<u64> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(repo.log_path())?;
    let start = file.metadata()?.len();
    let encoded = serde_json::to_vec(event)?;
    file.write_all(&encoded)?;
    file.write_all(b"\n")?;
    Ok(start + encoded.len() as u64 + 1)
}

fn next_event_ulid(conn: &Connection) -> Result<Ulid> {
    let last = db::get_meta(conn, "last_event_id")?;
    let last_ulid = last.as_deref().map(Ulid::from_str).transpose()?;

    loop {
        let candidate = Ulid::new();
        match last_ulid.as_ref() {
            Some(previous) if candidate <= *previous => continue,
            _ => return Ok(candidate),
        }
    }
}

fn current_actor() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}
