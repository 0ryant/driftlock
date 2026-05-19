//! `driftlock doctor` health checks.

use anyhow::{bail, Result};
use std::path::Path;

/// Runs repository health checks.
pub fn run(strict: bool, repo: &Path) -> Result<()> {
    let mut errors = Vec::new();

    for required in [
        "Cargo.toml",
        "contracts/schemas/taskgraph.schema.json",
        "lanes/default.toml",
        "metadata/mcp.manifest.json",
        "skills/driftlock-worker/SKILL.md",
    ] {
        if !repo.join(required).exists() {
            errors.push(format!("missing {required}"));
        }
    }

    if !repo.join(".git").exists() {
        errors.push("not a git repository (.git missing)".into());
    }

    if strict && errors.is_empty() {
        let status = std::process::Command::new("cargo")
            .arg("check")
            .arg("--workspace")
            .current_dir(repo)
            .status()?;
        if !status.success() {
            errors.push("cargo check --workspace failed".into());
        }
    }

    if errors.is_empty() {
        println!("ok: driftlock doctor passed");
        Ok(())
    } else {
        for e in &errors {
            eprintln!("error: {e}");
        }
        bail!("doctor found {} issue(s)", errors.len())
    }
}
