//! Graph persistence.

use crate::paths::StatePaths;
use anyhow::{Context, Result};
use driftlock_core::TaskGraph;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Loads `graph.json`.
pub fn load_graph(paths: &StatePaths) -> Result<TaskGraph> {
    let path = crate::paths::graph_path(paths);
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}

/// Saves `graph.json` atomically.
pub fn save_graph(paths: &StatePaths, graph: &TaskGraph) -> Result<()> {
    let path = crate::paths::graph_path(paths);
    write_atomic(&path, &serde_json::to_string_pretty(graph)?)
}

fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(&tmp)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}
