//! Repository indexing and Git diff helpers.

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Repository file inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoIndex {
    /// Root path indexed.
    pub root: String,
    /// Files found.
    pub files: Vec<String>,
}

/// Indexes repository files using ignore rules.
pub fn index_repo(root: impl AsRef<Path>) -> Result<RepoIndex> {
    let root = root.as_ref();
    let mut files = Vec::new();
    for entry in WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(|entry| entry.file_name().to_string_lossy() != ".git")
        .build()
    {
        let entry = entry?;
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
            files.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    files.sort();
    Ok(RepoIndex { root: root.to_string_lossy().to_string(), files })
}

/// Returns changed files from `git diff --name-only`.
pub fn git_changed_files(repo_root: impl AsRef<Path>, base_ref: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root.as_ref())
        .arg("diff")
        .arg("--name-only")
        .arg(base_ref)
        .output()
        .context("running git diff --name-only")?;
    if !output.status.success() {
        anyhow::bail!("git diff failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().replace('\\', "/"))
        .collect())
}

/// Returns current `HEAD` SHA for a repository.
pub fn current_head(repo_root: impl AsRef<Path>) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root.as_ref())
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .context("running git rev-parse HEAD")?;
    if !output.status.success() {
        anyhow::bail!("git rev-parse failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Returns true when `ancestor` is an ancestor of `head` (or equal).
pub fn is_ancestor(repo_root: impl AsRef<Path>, ancestor: &str, head: &str) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root.as_ref())
        .arg("merge-base")
        .arg("--is-ancestor")
        .arg(ancestor)
        .arg(head)
        .output()
        .context("running git merge-base --is-ancestor")?;
    Ok(output.status.success())
}

/// Extracts changed file paths from a unified diff.
pub fn changed_files_from_diff(diff: &str) -> Vec<String> {
    let mut files = BTreeSet::new();
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            if path != "/dev/null" {
                files.insert(path.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("diff --git a/") {
            if let Some((_left, right)) = rest.split_once(" b/") {
                files.insert(right.to_string());
            }
        } else if let Some(path) = line.strip_prefix("rename to ") {
            files.insert(path.to_string());
        }
    }
    files.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{changed_files_from_diff, index_repo};
    use std::fs;

    #[test]
    fn extracts_paths_from_unified_diff() {
        let diff = "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n";
        assert_eq!(changed_files_from_diff(diff), vec!["src/lib.rs"]);
    }

    #[test]
    fn index_repo_excludes_git_object_inventory() {
        let root =
            std::env::temp_dir().join(format!("driftlock-index-repo-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".git/objects/aa")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join(".git/objects/aa/blob"), "object").unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn ok() {}\n").unwrap();

        let index = index_repo(&root).unwrap();

        assert!(index.files.contains(&"src/lib.rs".to_string()));
        assert!(
            index.files.iter().all(|path| !path.starts_with(".git/")),
            "index included .git entries: {:?}",
            index.files
        );
        let _ = fs::remove_dir_all(root);
    }
}
