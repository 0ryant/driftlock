#![allow(clippy::too_many_lines, missing_docs)]

mod doctor;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use driftlock_core::{
    blocked_by_deps, build_task_graph, detect_graph_conflicts, extract_work_orders_from_adr,
    find_task, load_lane_manifest, promote_to_ready, ready_tasks_for_base, render_agent_brief,
    unlocks_for, verify_changed_files, TaskGraph, TaskStatus, WorkOrder,
};
use driftlock_store::{
    append_event, complete_claim, generate_operator_key, init_state_dir, load_graph, new_claim,
    record_claim, release_claim, save_graph, verify_events, EventKind, StatePaths,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "driftlock", version, about = "ADR-derived work orders and safe lanes for agents")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Initialize `.driftlock/` state directory.
    Init {
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// Health checks.
    Doctor {
        #[arg(long)]
        strict: bool,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// Export generated JSON Schemas.
    ExportSchemas { out_dir: PathBuf },
    /// Extract candidate tasks from an ADR.
    Extract {
        #[arg(long)]
        adr: PathBuf,
        #[arg(long, default_value = "lanes/default.toml")]
        lanes: PathBuf,
        #[arg(long, default_value = "core")]
        lane: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Build a task graph from one ADR.
    BuildGraph {
        #[arg(long)]
        adr: PathBuf,
        #[arg(long, default_value = "lanes/default.toml")]
        lanes: PathBuf,
        #[arg(long, default_value = "core")]
        lane: String,
        #[arg(long)]
        graph: Option<PathBuf>,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// Promote a reviewed task to ready.
    Promote {
        #[arg(long)]
        graph: PathBuf,
        #[arg(long)]
        task: String,
    },
    /// Return ready tasks for a lane.
    Ready {
        #[arg(long)]
        graph: Option<PathBuf>,
        #[arg(long)]
        lane: String,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// List conflicts.
    Conflicts {
        #[arg(long)]
        graph: PathBuf,
        #[arg(long)]
        task: Option<String>,
    },
    /// List incomplete dependencies.
    Deps {
        #[arg(long)]
        graph: PathBuf,
        #[arg(long)]
        task: String,
    },
    /// List tasks unlocked when task completes.
    Unlocks {
        #[arg(long)]
        graph: PathBuf,
        #[arg(long)]
        task: String,
    },
    /// Render an agent brief.
    Brief {
        #[arg(long)]
        graph: PathBuf,
        #[arg(long)]
        task: String,
    },
    /// Verify touched files against a task write set.
    CheckDiff {
        #[arg(long)]
        graph: PathBuf,
        #[arg(long)]
        task: String,
        #[arg(long)]
        diff_file: Option<PathBuf>,
        #[arg(long)]
        changed: Vec<String>,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// Claim a ready task.
    Claim {
        #[arg(long)]
        graph: PathBuf,
        #[arg(long)]
        task: String,
        #[arg(long)]
        actor: String,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// Release a claim.
    Release {
        #[arg(long)]
        task: String,
        #[arg(long)]
        actor: String,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// Complete a claimed task after diff verification.
    Complete {
        #[arg(long)]
        graph: PathBuf,
        #[arg(long)]
        task: String,
        #[arg(long)]
        actor: String,
        #[arg(long)]
        diff_file: Option<PathBuf>,
        #[arg(long)]
        changed: Vec<String>,
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// Recompute conflicts and refresh graph metadata.
    Refresh {
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// List embedded skills.
    Skills,
    /// Index a repository.
    Index {
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
    /// Emit MCP host config snippets.
    EmitHostConfig {
        #[arg(default_value = ".")]
        repo: PathBuf,
        #[arg(long, default_value = ".driftlock/hosts")]
        out: PathBuf,
    },
    /// Operator signing keys.
    Key {
        #[command(subcommand)]
        command: KeyCommand,
    },
    /// Audit event ledger.
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
}

#[derive(Debug, Subcommand)]
enum KeyCommand {
    /// Generate `.driftlock/keys/active.ed25519`.
    Generate {
        #[arg(default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
enum AuditCommand {
    /// Verify `events.jsonl` (optionally require signatures).
    Verify {
        #[arg(default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        signed: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { repo } => {
            let paths = init_state_dir(&repo)?;
            seed_graph_from_canonical_ledger(&paths)?;
            println!("initialized {}", paths.state_dir.display());
            Ok(())
        }
        Command::Doctor { strict, repo } => doctor::run(strict, &repo),
        Command::ExportSchemas { out_dir } => driftlock_contracts::write_schemas(out_dir),
        Command::Extract { adr, lanes, lane, out } => {
            let lane_manifest = load_lane_manifest(&lanes)?;
            let text = fs::read_to_string(&adr)?;
            let base = driftlock_git::current_head(".").unwrap_or_else(|_| "working-tree".into());
            let tasks = extract_work_orders_from_adr(
                &adr.to_string_lossy(),
                &base,
                &text,
                &lane,
                Some(&lane_manifest),
            );
            let json = serde_json::to_string_pretty(&tasks)?;
            if let Some(out) = out {
                fs::write(&out, &json)?;
            } else {
                println!("{json}");
            }
            Ok(())
        }
        Command::BuildGraph { adr, lanes, lane, graph, repo } => {
            let paths = init_state_dir(&repo)?;
            let lane_manifest = load_lane_manifest(repo.join(&lanes))?;
            let text = fs::read_to_string(&adr)?;
            let base = driftlock_git::current_head(&repo).unwrap_or_else(|_| "working-tree".into());
            let tasks = extract_work_orders_from_adr(
                &adr.to_string_lossy(),
                &base,
                &text,
                &lane,
                Some(&lane_manifest),
            );
            let built = build_task_graph(
                "driftlock-graph",
                repo.to_string_lossy().into_owned(),
                &base,
                tasks,
                lane_manifest,
            );
            save_graph(&paths, &built)?;
            if let Some(path) = graph {
                write_graph_file(&path, &built)?;
            }
            append_event(&paths, EventKind::GraphBuilt, "cli", None, BTreeMap::new())?;
            println!("wrote {}", default_graph_path(&paths).display());
            Ok(())
        }
        Command::Promote { graph, task } => {
            let mut g = read_graph_file(&graph)?;
            let t = find_task_mut(&mut g, &task)?;
            promote_to_ready(t);
            write_graph_file(&graph, &g)?;
            Ok(())
        }
        Command::Ready { graph, lane, repo } => {
            let g = load_graph_for_repo(&repo, graph)?;
            let base = driftlock_git::current_head(&repo).unwrap_or_else(|_| g.base_ref.clone());
            let tasks = ready_tasks_for_base(&g, &lane, &base);
            println!("{}", serde_json::to_string_pretty(&tasks)?);
            Ok(())
        }
        Command::Conflicts { graph, task } => {
            let g = read_graph_file(&graph)?;
            let report = detect_graph_conflicts(&g);
            if let Some(task) = task {
                println!("{}", serde_json::to_string_pretty(&report.get(&task))?);
            } else {
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            Ok(())
        }
        Command::Deps { graph, task } => {
            let g = read_graph_file(&graph)?;
            println!("{}", serde_json::to_string_pretty(&blocked_by_deps(&g, &task))?);
            Ok(())
        }
        Command::Unlocks { graph, task } => {
            let g = read_graph_file(&graph)?;
            println!("{}", serde_json::to_string_pretty(&unlocks_for(&g, &task))?);
            Ok(())
        }
        Command::Brief { graph, task } => {
            let g = read_graph_file(&graph)?;
            let task = find_task(&g, &task).context("task not found")?;
            println!("{}", render_agent_brief(task));
            Ok(())
        }
        Command::CheckDiff { graph, task, diff_file, changed, repo } => {
            let g = read_graph_file(&graph)?;
            let task = find_task(&g, &task).context("task not found")?;
            let touched = touched_files(&repo, diff_file, changed)?;
            let report = verify_changed_files(task, &touched);
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Command::Claim { graph, task, actor, repo } => {
            let paths = init_state_dir(&repo)?;
            let g = read_graph_file(&graph)?;
            let wo = find_task(&g, &task).context("task not found")?;
            let base = driftlock_git::current_head(&repo).unwrap_or_else(|_| g.base_ref.clone());
            let claim = new_claim(&task, &actor, &base, wo.write_set.clone());
            record_claim(&paths, &claim, &actor)?;
            Ok(())
        }
        Command::Release { task, actor, repo } => {
            let paths = init_state_dir(&repo)?;
            release_claim(&paths, &task, &actor)?;
            Ok(())
        }
        Command::Complete { graph, task, actor, diff_file, changed, repo } => {
            let paths = init_state_dir(&repo)?;
            let g = read_graph_file(&graph)?;
            let wo = find_task(&g, &task).context("task not found")?;
            let touched = touched_files(&repo, diff_file, changed)?;
            let report = verify_changed_files(wo, &touched);
            if !report.allowed {
                bail!("diff verification failed: {:?}", report.violations);
            }
            complete_claim(&paths, &task, &actor)?;
            let mut g = g;
            if let Ok(t) = find_task_mut(&mut g, &task) {
                t.status = TaskStatus::Complete;
            }
            save_graph(&paths, &g)?;
            Ok(())
        }
        Command::Refresh { repo } => {
            let paths = init_state_dir(&repo)?;
            let mut g = load_graph(&paths)?;
            let conflicts = detect_graph_conflicts(&g);
            driftlock_core::attach_conflicts_to_tasks(&mut g.tasks, &conflicts);
            g.generated_at = built_graph_timestamp();
            save_graph(&paths, &g)?;
            append_event(&paths, EventKind::ConflictDetected, "cli", None, BTreeMap::new())?;
            println!("refreshed graph at {}", default_graph_path(&paths).display());
            Ok(())
        }
        Command::Skills => {
            let names: Vec<_> = driftlock_skills::skills().iter().map(|s| s.name).collect();
            println!("{}", serde_json::to_string_pretty(&names)?);
            Ok(())
        }
        Command::Index { repo } => {
            let index = driftlock_git::index_repo(repo)?;
            println!("{}", serde_json::to_string_pretty(&index)?);
            Ok(())
        }
        Command::EmitHostConfig { repo, out } => {
            fs::create_dir_all(&out)?;
            let abs = repo.canonicalize()?;
            let cmd = format!(
                "cargo run -p driftlock-mcp --manifest-path {} -- stdio --repo {}",
                abs.join("Cargo.toml").display(),
                abs.display()
            );
            for host in ["cursor", "claude", "codex"] {
                let cfg = serde_json::json!({
                    "mcpServers": {
                        "driftlock": {
                            "command": "sh",
                            "args": ["-c", cmd.clone()]
                        }
                    }
                });
                fs::write(out.join(format!("{host}.json")), serde_json::to_string_pretty(&cfg)?)?;
            }
            println!("wrote host configs to {}", out.display());
            Ok(())
        }
        Command::Key { command } => match command {
            KeyCommand::Generate { repo, force } => {
                let info = generate_operator_key(&repo, force)?;
                println!("{}", serde_json::to_string_pretty(&info)?);
                Ok(())
            }
        },
        Command::Audit { command } => match command {
            AuditCommand::Verify { repo, signed } => {
                let report = verify_events(&repo, signed)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
                if !report.is_pass() {
                    bail!("audit verify failed");
                }
                Ok(())
            }
        },
    }
}

fn default_graph_path(paths: &StatePaths) -> PathBuf {
    driftlock_store::graph_path(paths)
}

fn seed_graph_from_canonical_ledger(paths: &StatePaths) -> Result<bool> {
    let graph_path = default_graph_path(paths);
    if graph_path.exists() {
        return Ok(false);
    }
    let canonical = paths.repo_root.join("tasks/taskgraph.json");
    if !canonical.exists() {
        return Ok(false);
    }
    let graph = read_graph_file(&canonical)
        .with_context(|| format!("read canonical taskgraph {}", canonical.display()))?;
    save_graph(paths, &graph)?;
    Ok(true)
}

fn load_graph_for_repo(repo: &Path, graph: Option<PathBuf>) -> Result<TaskGraph> {
    if let Some(path) = graph {
        return read_graph_file(&path);
    }
    let paths = init_state_dir(repo)?;
    load_graph(&paths)
}

fn read_graph_file(path: &Path) -> Result<TaskGraph> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn write_graph_file(path: &Path, graph: &TaskGraph) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(graph)?)?;
    Ok(())
}

fn find_task_mut<'a>(graph: &'a mut TaskGraph, task_id: &str) -> Result<&'a mut WorkOrder> {
    graph.tasks.iter_mut().find(|t| t.id == task_id).context("task not found")
}

fn built_graph_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn touched_files(
    repo: &Path,
    diff_file: Option<PathBuf>,
    changed: Vec<String>,
) -> Result<Vec<String>> {
    if let Some(diff_file) = diff_file {
        let diff = fs::read_to_string(diff_file)?;
        return Ok(driftlock_git::changed_files_from_diff(&diff));
    }
    if !changed.is_empty() {
        return Ok(changed);
    }
    driftlock_git::git_changed_files(repo, "HEAD")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn init_seed_graph_from_canonical_ledger_when_missing() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let repo = std::env::temp_dir()
            .join(format!("driftlock-init-seed-test-{}-{nonce}", std::process::id()));
        fs::create_dir_all(repo.join("tasks")).unwrap();
        fs::write(repo.join("tasks/taskgraph.json"), include_str!("../../../tasks/taskgraph.json"))
            .unwrap();

        let paths = init_state_dir(&repo).unwrap();
        assert!(seed_graph_from_canonical_ledger(&paths).unwrap());
        assert!(paths.state_dir.join("graph.json").is_file());
        let graph = load_graph(&paths).unwrap();
        assert!(!graph.tasks.is_empty());
        assert!(!seed_graph_from_canonical_ledger(&paths).unwrap());

        let _ = fs::remove_dir_all(repo);
    }
}
