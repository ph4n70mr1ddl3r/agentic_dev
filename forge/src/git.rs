//! Thin local-`git` CLI wrappers for PR write-back (branch, add, commit, push).
//! The harness operates on the current working repo (run from the repo root).
//! Git credentials come from the configured helper (e.g. `gh auth login`).

use anyhow::{bail, Context, Result};

fn run(args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("spawn git {:?}", args))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        bail!("git {:?} failed: {stderr}", args);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn current_branch() -> Result<String> {
    run(&["rev-parse", "--abbrev-ref", "HEAD"])
}

/// True if the working tree has no uncommitted changes.
pub fn is_clean() -> Result<bool> {
    Ok(run(&["status", "--porcelain"])?.is_empty())
}

pub fn branch_exists(name: &str) -> Result<bool> {
    let st = std::process::Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ])
        .status()
        .context("spawn git show-ref")?;
    Ok(st.success())
}

pub fn checkout(name: &str) -> Result<()> {
    run(&["checkout", name]).map(|_| ())
}

/// Create `branch` from `base` and switch to it. Uncommitted changes (e.g. the
/// just-written artifact) are carried onto the new branch.
pub fn create_branch_from(base: &str, branch: &str) -> Result<()> {
    run(&["checkout", "-b", branch, base]).map(|_| ())
}

pub fn add(path: &str) -> Result<()> {
    run(&["add", "--", path]).map(|_| ())
}

/// Commit staged changes. Returns Ok(false) for "nothing to commit".
pub fn commit(message: &str) -> Result<bool> {
    let out = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .context("spawn git commit")?;
    if out.status.success() {
        return Ok(true);
    }
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    if combined.contains("nothing to commit") || combined.contains("no changes") {
        return Ok(false);
    }
    bail!("git commit failed: {combined}");
}

pub fn push(branch: &str, remote: &str) -> Result<()> {
    run(&["push", "-u", remote, branch]).map(|_| ())
}

pub fn delete_local_branch(name: &str) -> Result<()> {
    run(&["branch", "-D", name]).map(|_| ())
}
