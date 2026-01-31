use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::{
    db,
    error::{BeadsError, Result},
};

pub const BEADS_DIR: &str = ".beads";
pub const EVENTS_FILE: &str = "events.jsonl";
pub const DB_FILE: &str = "beads.db";
pub const BLOBS_DIR: &str = "blobs";
pub const DOCS_DIR: &str = "docs";

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

pub fn init_repo(path: &Path, prefix: &str) -> Result<BeadsRepo> {
    let repo = BeadsRepo::new(path.to_path_buf());
    if repo.beads_dir().exists() {
        return Err(BeadsError::AlreadyInitialized);
    }
    fs::create_dir_all(repo.beads_dir())?;
    fs::create_dir_all(repo.beads_dir().join(BLOBS_DIR))?;
    if !repo.log_path().exists() {
        File::create(repo.log_path())?;
    }
    ensure_gitignore_entries(path)?;
    let mut conn = repo.open_db()?;
    crate::db::create_schema(&conn)?;
    let tx = conn.transaction()?;
    db::set_meta(&tx, "id_prefix", prefix.to_string())?;
    db::set_meta(&tx, "last_issue_serial", "0".to_string())?;
    tx.commit()?;
    Ok(repo)
}

fn ensure_gitignore_entries(path: &Path) -> Result<()> {
    let gitignore_path = path.join(".gitignore");
    let db_entry = format!("{}/{}", BEADS_DIR, DB_FILE);
    let docs_entry = format!("{}/{}/", BEADS_DIR, DOCS_DIR);

    let contents = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    let mut entries_to_add = Vec::new();
    if !contents.lines().any(|line| line.trim() == db_entry) {
        entries_to_add.push(db_entry);
    }
    if !contents.lines().any(|line| line.trim() == docs_entry) {
        entries_to_add.push(docs_entry);
    }

    if !entries_to_add.is_empty() {
        if gitignore_path.exists() {
            let mut file = OpenOptions::new().append(true).open(&gitignore_path)?;
            if !contents.ends_with('\n') {
                writeln!(file)?;
            }
            for entry in entries_to_add {
                writeln!(file, "{}", entry)?;
            }
        } else {
            let mut content = String::new();
            for entry in entries_to_add {
                content.push_str(&format!("{}\n", entry));
            }
            fs::write(&gitignore_path, content)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_repo_creates_gitignore_with_db_entry() -> Result<()> {
        let temp = tempfile::tempdir()?;
        init_repo(temp.path(), "test")?;
        let gitignore_path = temp.path().join(".gitignore");
        assert!(gitignore_path.exists());
        let contents = fs::read_to_string(gitignore_path)?;
        assert!(contents
            .lines()
            .any(|line| line.trim() == ".beads/beads.db"));
        Ok(())
    }

    #[test]
    fn ensure_gitignore_does_not_duplicate_entry() -> Result<()> {
        let temp = tempfile::tempdir()?;
        fs::write(temp.path().join(".gitignore"), "/target\n")?;
        ensure_gitignore_entries(temp.path())?;
        ensure_gitignore_entries(temp.path())?;
        let contents = fs::read_to_string(temp.path().join(".gitignore"))?;
        let matches = contents
            .lines()
            .filter(|line| line.trim() == ".beads/beads.db")
            .count();
        assert_eq!(matches, 1);
        Ok(())
    }

    #[test]
    fn init_repo_creates_gitignore_with_docs_entry() -> Result<()> {
        let temp = tempfile::tempdir()?;
        init_repo(temp.path(), "test")?;
        let gitignore_path = temp.path().join(".gitignore");
        assert!(gitignore_path.exists());
        let contents = fs::read_to_string(gitignore_path)?;
        assert!(contents
            .lines()
            .any(|line| line.trim() == ".beads/docs/"));
        Ok(())
    }

    #[test]
    fn init_repo_creates_blobs_directory() -> Result<()> {
        let temp = tempfile::tempdir()?;
        init_repo(temp.path(), "test")?;
        let blobs_dir = temp.path().join(".beads/blobs");
        assert!(blobs_dir.exists());
        assert!(blobs_dir.is_dir());
        Ok(())
    }
}
