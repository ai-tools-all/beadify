use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{BeadsError, Result};

pub const BEADS_DIR: &str = ".beads";
pub const EVENTS_FILE: &str = "events.jsonl";
pub const DB_FILE: &str = "beads.db";

#[derive(Debug, Clone)]
pub struct BeadsRepo {
    root: PathBuf,
    beads_dir: PathBuf,
    log_path: PathBuf,
    db_path: PathBuf,
}

impl BeadsRepo {
    pub(crate) fn new(root: PathBuf) -> Self {
        let beads_dir = root.join(BEADS_DIR);
        let log_path = beads_dir.join(EVENTS_FILE);
        let db_path = beads_dir.join(DB_FILE);
        Self {
            root,
            beads_dir,
            log_path,
            db_path,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn beads_dir(&self) -> &Path {
        &self.beads_dir
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn open_db(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }
}

pub fn find_repo() -> Result<BeadsRepo> {
    let mut current = std::env::current_dir()?;
    loop {
        let candidate = current.join(BEADS_DIR);
        if candidate.is_dir() {
            return Ok(BeadsRepo::new(current));
        }
        if !current.pop() {
            break;
        }
    }
    Err(BeadsError::RepoNotFound)
}

pub fn init_repo(path: &Path) -> Result<BeadsRepo> {
    let repo = BeadsRepo::new(path.to_path_buf());
    if repo.beads_dir().exists() {
        return Err(BeadsError::AlreadyInitialized);
    }
    std::fs::create_dir_all(repo.beads_dir())?;
    if !repo.log_path().exists() {
        std::fs::File::create(repo.log_path())?;
    }
    let conn = repo.open_db()?;
    crate::db::create_schema(&conn)?;
    Ok(repo)
}
