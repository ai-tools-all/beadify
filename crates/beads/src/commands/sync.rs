use anyhow::{anyhow, Context, Result};
use beads_core::{repo::BeadsRepo, sync_repo};
use std::process::Command;

pub fn run(repo: BeadsRepo, full: bool) -> Result<()> {
    Command::new("git")
        .arg("pull")
        .current_dir(repo.root())
        .status()
        .with_context(|| "failed to run git pull")?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("git pull failed"))?;

    let applied = sync_repo(&repo, full)?;
    println!("Applied {applied} events");

    Command::new("git")
        .arg("push")
        .current_dir(repo.root())
        .status()
        .with_context(|| "failed to run git push")?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("git push failed"))?;

    Ok(())
}
