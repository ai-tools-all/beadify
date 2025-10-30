use anyhow::{bail, Result};

pub fn run(prefix: &str) -> Result<()> {
    if prefix.trim().is_empty() {
        bail!("prefix must not be empty");
    }
    let cwd = std::env::current_dir()?;
    let repo = beads_core::init_repo(&cwd, prefix)?;
    println!("Initialized beads repository at {}", repo.root().display());
    println!("Issue prefix: {prefix}");
    Ok(())
}
