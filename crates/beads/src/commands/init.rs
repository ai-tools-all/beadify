use anyhow::Result;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = beads_core::init_repo(&cwd)?;
    println!("Initialized beads repository at {}", repo.root().display());
    Ok(())
}
