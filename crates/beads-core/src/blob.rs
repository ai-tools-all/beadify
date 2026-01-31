use std::fs;
use std::io::Write;

use sha2::{Digest, Sha256};

use crate::{
    error_v2::{Error, Result},
    repo::{BeadsRepo, BLOBS_DIR},
};

/// Write content to the blob store and return its SHA-256 hash.
/// The hash is returned as a lowercase hexadecimal string.
/// If the blob already exists, this function still succeeds (idempotent).
pub fn write_blob(repo: &BeadsRepo, content: &[u8]) -> Result<String> {
    let hash = calculate_hash(content);
    let blob_path = repo.beads_dir().join(BLOBS_DIR).join(&hash);

    if blob_path.exists() {
        return Ok(hash);
    }

    let mut file = fs::File::create(&blob_path)?;
    file.write_all(content)?;
    file.sync_all()?;

    Ok(hash)
}

/// Read content from the blob store by hash.
/// Returns an error if the blob does not exist or cannot be read.
pub fn read_blob(repo: &BeadsRepo, hash: &str) -> Result<Vec<u8>> {
    validate_hash(hash)?;
    let blob_path = repo.beads_dir().join(BLOBS_DIR).join(hash);

    if !blob_path.exists() {
        return Err(Error::BlobNotFound {
            hash: hash.to_string(),
        });
    }

    Ok(fs::read(&blob_path)?)
}

fn calculate_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

fn validate_hash(hash: &str) -> Result<()> {
     if hash.len() != 64 {
         return Err(Error::InvalidHash {
             hash: format!("Hash must be 64 characters, got {}", hash.len()),
         });
     }
     if !hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
         return Err(Error::InvalidHash {
             hash: "Hash must contain only lowercase hexadecimal characters".to_string(),
         });
     }
     Ok(())
 }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo;

    #[test]
    fn test_write_and_read_blob() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = repo::init_repo(temp.path(), "test")?;

        let content = b"Hello, blob store!";
        let hash = write_blob(&repo, content)?;

        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        let read_content = read_blob(&repo, &hash)?;
        assert_eq!(read_content, content);

        Ok(())
    }

    #[test]
    fn test_write_blob_is_idempotent() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = repo::init_repo(temp.path(), "test")?;

        let content = b"Same content";
        let hash1 = write_blob(&repo, content)?;
        let hash2 = write_blob(&repo, content)?;

        assert_eq!(hash1, hash2);

        Ok(())
    }

    #[test]
    fn test_read_nonexistent_blob() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = repo::init_repo(temp.path(), "test")?;

        let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";
         let result = read_blob(&repo, fake_hash);
        
         assert!(matches!(result, Err(Error::BlobNotFound { .. })));

        Ok(())
    }

    #[test]
    fn test_validate_hash_invalid_length() {
        let result = validate_hash("tooshort");
        assert!(matches!(result, Err(Error::InvalidHash { .. })));
    }
    
    #[test]
    fn test_validate_hash_invalid_chars() {
        let result = validate_hash("ZZZZ000000000000000000000000000000000000000000000000000000000000");
        assert!(matches!(result, Err(Error::InvalidHash { .. })));
    }
    
    #[test]
    fn test_validate_hash_uppercase() {
        let result = validate_hash("AAAA000000000000000000000000000000000000000000000000000000000000");
        assert!(matches!(result, Err(Error::InvalidHash { .. })));
    }

    #[test]
    fn test_different_content_different_hash() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = repo::init_repo(temp.path(), "test")?;

        let content1 = b"First content";
        let content2 = b"Second content";

        let hash1 = write_blob(&repo, content1)?;
        let hash2 = write_blob(&repo, content2)?;

        assert_ne!(hash1, hash2);

        Ok(())
    }

    #[test]
    fn test_hash_format() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = repo::init_repo(temp.path(), "test")?;

        let content = b"test";
        let hash = write_blob(&repo, content)?;

        let expected = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
        assert_eq!(hash, expected);

        Ok(())
    }
}
