use thiserror::Error;

#[derive(Debug, Error)]
pub enum BeadsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("ulid decode error: {0}")]
    UlidDecode(#[from] ulid::DecodeError),
    #[error("beads repository already initialized")]
    AlreadyInitialized,
    #[error("beads repository not found. Run 'beads init --prefix <prefix>' to create one.")]
    RepoNotFound,
    #[error("missing repository configuration: {0}")]
    MissingConfig(&'static str),
    #[error("update requires at least one field")]
    EmptyUpdate,
    #[error("blob not found: {0}")]
    BlobNotFound(String),
    #[error("invalid hash: {0}")]
    InvalidHash(String),
    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, BeadsError>;
