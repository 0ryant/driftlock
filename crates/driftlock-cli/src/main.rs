#![allow(clippy::too_many_lines, missing_docs)]

mod doctor;
mod support;

use axiom_exit::Exit;
use clap::{Parser, Subcommand};
use driftlock_core::{
    blocked_by_deps, build_task_graph, detect_graph_conflicts, extract_work_orders_from_adr,
    find_task, load_lane_manifest, promote_to_ready, ready_tasks_for_base, render_agent_brief,
    unlocks_for, verify_changed_files_with_gates, TaskGraph, TaskStatus, WorkOrder,
};
use driftlock_store::{
    append_event, complete_claim, generate_operator_key, init_state_dir, load_graph, new_claim,
    record_claim, release_claim, save_graph, trust_operator_key, verify_audit_chain, verify_events,
    Artifact, ChainVerdict, EventKind, StatePaths,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use support::{io_err, json_err, record_operation, CliError, CliResult};

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
        /// Delegate command-gate execution to an external runner. Driftlock
        /// itself never spawns a process; this only changes how command
        /// obligations are described.
        #[arg(long)]
        allow_exec: bool,
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
        /// Delegate command-gate execution to an external runner. Driftlock
        /// itself never spawns a process; this only changes how command
        /// obligations are described.
        #[arg(long)]
        allow_exec: bool,
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
    /// Audit event ledger and the doctrine audit trail.
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
}

#[derive(Debug, Subcommand)]
enum KeyCommand {
    /// Generate `.driftlock/keys/active.ed25519` (does not auto-trust).
    Generate {
        #[arg(default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Add the active key to the trust store after confirming its fingerprint.
    Trust {
        /// Fingerprint (`fp:...`) printed by `key generate`, confirmed out-of-band.
        fingerprint: String,
        #[arg(default_value = ".")]
        repo: PathBuf,
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
    /// Verify the `audit-trail.jsonl` BLAKE3 hash chain (axiom.audit.v1).
    VerifyChain {
        #[arg(default_value = ".")]
        repo: PathBuf,
    },
}

fn main() -> ExitCode {
    // clap handles its own usage/help exit (code 2) before we run.
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(exit) => exit.into(),
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(err.exit)
        }
    }
}

fn run(command: Command) -> CliResult<Exit> {
    match command {
        Command::Init { repo } => {
            let paths = init_state_dir(&repo).map_err(preflight("init state dir"))?;
            seed_graph_from_canonical_ledger(&paths)?;
            println!("initialized {}", paths.state_dir.display());
            Ok(Exit::Ok)
        }
        Command::Doctor { strict, repo } => match doctor::run(strict, &repo) {
            Ok(()) => Ok(Exit::Ok),
            // doctor failures are preflight/environment problems, not assertions.
            Err(e) => Err(CliError::preflight(e.to_string())),
        },
        Command::ExportSchemas { out_dir } => {
            driftlock_contracts::write_schemas(out_dir).map_err(preflight("export schemas"))?;
            Ok(Exit::Ok)
        }
        Command::Extract { adr, lanes, lane, out } => {
            let lane_manifest =
                load_lane_manifest(&lanes).map_err(preflight("load lane manifest"))?;
            let text = std::fs::read_to_string(&adr).map_err(io_err("read ADR"))?;
            let base = driftlock_git::current_head(".").unwrap_or_else(|_| "working-tree".into());
            let tasks = extract_work_orders_from_adr(
                &adr.to_string_lossy(),
                &base,
                &text,
                &lane,
                Some(&lane_manifest),
            );
            let json = serde_json::to_string_pretty(&tasks).map_err(json_err("serialize tasks"))?;
            if let Some(out) = out {
                std::fs::write(&out, &json).map_err(io_err("write extract output"))?;
            } else {
                println!("{json}");
            }
            Ok(Exit::Ok)
        }
        Command::BuildGraph { adr, lanes, lane, graph, repo } => {
            let paths = init_state_dir(&repo).map_err(preflight("init state dir"))?;
            let lane_manifest =
                load_lane_manifest(repo.join(&lanes)).map_err(preflight("load lane manifest"))?;
            let text = std::fs::read_to_string(&adr).map_err(io_err("read ADR"))?;
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
            save_graph(&paths, &built).map_err(preflight("save graph"))?;
            if let Some(path) = &graph {
                write_graph_file(path, &built)?;
            }
            append_event(&paths, EventKind::GraphBuilt, "cli", None, BTreeMap::new())
                .map_err(preflight("append graph-built event"))?;
            let graph_path = default_graph_path(&paths);
            record_operation(
                &paths,
                "build-graph",
                "ok",
                Exit::Ok.as_i32(),
                vec![artifact_of_file(&paths.repo_root, &adr)?],
                vec![artifact_of_file(&paths.repo_root, &graph_path)?],
                "cli",
            )?;
            println!("wrote {}", graph_path.display());
            Ok(Exit::Ok)
        }
        Command::Promote { graph, task } => {
            let mut g = read_graph_file(&graph)?;
            let t = find_task_mut(&mut g, &task)?;
            promote_to_ready(t);
            write_graph_file(&graph, &g)?;
            Ok(Exit::Ok)
        }
        Command::Ready { graph, lane, repo } => {
            let g = load_graph_for_repo(&repo, graph)?;
            let base = driftlock_git::current_head(&repo).unwrap_or_else(|_| g.base_ref.clone());
            let tasks = ready_tasks_for_base(&g, &lane, &base);
            println!("{}", pretty(&tasks)?);
            Ok(Exit::Ok)
        }
        Command::Conflicts { graph, task } => {
            let g = read_graph_file(&graph)?;
            let report = detect_graph_conflicts(&g);
            if let Some(task) = task {
                println!("{}", pretty(&report.get(&task))?);
            } else {
                println!("{}", pretty(&report)?);
            }
            Ok(Exit::Ok)
        }
        Command::Deps { graph, task } => {
            let g = read_graph_file(&graph)?;
            println!("{}", pretty(&blocked_by_deps(&g, &task))?);
            Ok(Exit::Ok)
        }
        Command::Unlocks { graph, task } => {
            let g = read_graph_file(&graph)?;
            println!("{}", pretty(&unlocks_for(&g, &task))?);
            Ok(Exit::Ok)
        }
        Command::Brief { graph, task } => {
            let g = read_graph_file(&graph)?;
            let task = find_task(&g, &task).ok_or_else(|| CliError::usage("task not found"))?;
            println!("{}", render_agent_brief(task));
            Ok(Exit::Ok)
        }
        Command::CheckDiff { graph, task, diff_file, changed, repo, allow_exec } => {
            let g = read_graph_file(&graph)?;
            let task = find_task(&g, &task).ok_or_else(|| CliError::usage("task not found"))?;
            let touched = touched_files(&repo, diff_file, changed)?;
            let report = verify_changed_files_with_gates(task, &touched, &repo, allow_exec);
            println!("{}", pretty(&report)?);
            // A write-set escape is a checked assertion, not a crash: exit 1.
            if report.allowed {
                Ok(Exit::Ok)
            } else {
                Err(CliError::assertion(format!(
                    "diff verification failed: {:?}",
                    report.violations
                )))
            }
        }
        Command::Claim { graph, task, actor, repo } => {
            let paths = init_state_dir(&repo).map_err(preflight("init state dir"))?;
            let g = read_graph_file(&graph)?;
            let wo = find_task(&g, &task).ok_or_else(|| CliError::usage("task not found"))?;
            let base = driftlock_git::current_head(&repo).unwrap_or_else(|_| g.base_ref.clone());
            let claim = new_claim(&task, &actor, &base, wo.write_set.clone());
            record_claim(&paths, &claim, &actor).map_err(preflight("record claim"))?;
            record_operation(
                &paths,
                "claim",
                "ok",
                Exit::Ok.as_i32(),
                vec![Artifact::of_bytes("task", &task, task.as_bytes())],
                vec![],
                &actor,
            )?;
            Ok(Exit::Ok)
        }
        Command::Release { task, actor, repo } => {
            let paths = init_state_dir(&repo).map_err(preflight("init state dir"))?;
            release_claim(&paths, &task, &actor).map_err(preflight("release claim"))?;
            record_operation(
                &paths,
                "release",
                "ok",
                Exit::Ok.as_i32(),
                vec![Artifact::of_bytes("task", &task, task.as_bytes())],
                vec![],
                &actor,
            )?;
            Ok(Exit::Ok)
        }
        Command::Complete { graph, task, actor, diff_file, changed, repo, allow_exec } => {
            let paths = init_state_dir(&repo).map_err(preflight("init state dir"))?;
            let g = read_graph_file(&graph)?;
            let wo = find_task(&g, &task).ok_or_else(|| CliError::usage("task not found"))?;
            let touched = touched_files(&repo, diff_file, changed)?;
            let report = verify_changed_files_with_gates(wo, &touched, &repo, allow_exec);
            if !report.allowed {
                // A write-set escape on complete is a preflight gate: it stops the
                // mutation before it lands (FAILED_PREFLIGHT = 3).
                record_operation(
                    &paths,
                    "complete",
                    "failed",
                    Exit::Preflight.as_i32(),
                    vec![Artifact::of_bytes("task", &task, task.as_bytes())],
                    vec![],
                    &actor,
                )?;
                return Err(CliError::preflight(format!(
                    "diff verification failed (write-set escape): {:?}",
                    report.violations
                )));
            }
            complete_claim(&paths, &task, &actor).map_err(preflight("complete claim"))?;
            let mut g = g;
            if let Ok(t) = find_task_mut(&mut g, &task) {
                t.status = TaskStatus::Complete;
            }
            save_graph(&paths, &g).map_err(preflight("save graph"))?;
            let graph_path = default_graph_path(&paths);
            record_operation(
                &paths,
                "complete",
                "ok",
                Exit::Ok.as_i32(),
                vec![Artifact::of_bytes("task", &task, task.as_bytes())],
                vec![artifact_of_file(&paths.repo_root, &graph_path)?],
                &actor,
            )?;
            Ok(Exit::Ok)
        }
        Command::Refresh { repo } => {
            let paths = init_state_dir(&repo).map_err(preflight("init state dir"))?;
            let mut g = load_graph(&paths).map_err(preflight("load graph"))?;
            let conflicts = detect_graph_conflicts(&g);
            driftlock_core::attach_conflicts_to_tasks(&mut g.tasks, &conflicts);
            g.generated_at = built_graph_timestamp();
            save_graph(&paths, &g).map_err(preflight("save graph"))?;
            append_event(&paths, EventKind::ConflictDetected, "cli", None, BTreeMap::new())
                .map_err(preflight("append conflict event"))?;
            let graph_path = default_graph_path(&paths);
            record_operation(
                &paths,
                "refresh",
                "ok",
                Exit::Ok.as_i32(),
                vec![],
                vec![artifact_of_file(&paths.repo_root, &graph_path)?],
                "cli",
            )?;
            println!("refreshed graph at {}", graph_path.display());
            Ok(Exit::Ok)
        }
        Command::Skills => {
            let names: Vec<_> = driftlock_skills::skills().iter().map(|s| s.name).collect();
            println!("{}", pretty(&names)?);
            Ok(Exit::Ok)
        }
        Command::Index { repo } => {
            let index = driftlock_git::index_repo(repo).map_err(preflight("index repo"))?;
            println!("{}", pretty(&index)?);
            Ok(Exit::Ok)
        }
        Command::EmitHostConfig { repo, out } => {
            std::fs::create_dir_all(&out).map_err(io_err("create host config dir"))?;
            let abs = repo.canonicalize().map_err(io_err("canonicalize repo"))?;
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
                std::fs::write(out.join(format!("{host}.json")), pretty(&cfg)?)
                    .map_err(io_err("write host config"))?;
            }
            println!("wrote host configs to {}", out.display());
            Ok(Exit::Ok)
        }
        Command::Key { command } => match command {
            KeyCommand::Generate { repo, force } => {
                let info = generate_operator_key(&repo, force).map_err(preflight("generate key"))?;
                println!("{}", pretty(&info)?);
                eprintln!(
                    "key generated but NOT trusted. To trust it run:\n  driftlock key trust {} {}",
                    info.key_id,
                    repo.display()
                );
                Ok(Exit::Ok)
            }
            KeyCommand::Trust { fingerprint, repo } => {
                // A fingerprint mismatch is the operator passing the wrong value:
                // a usage error (exit 2), not a runtime crash.
                let info = trust_operator_key(&repo, &fingerprint)
                    .map_err(|e| CliError::usage(e.to_string()))?;
                println!("{}", pretty(&info)?);
                Ok(Exit::Ok)
            }
        },
        Command::Audit { command } => match command {
            AuditCommand::Verify { repo, signed } => {
                let report = verify_events(&repo, signed).map_err(preflight("verify events"))?;
                println!("{}", pretty(&report)?);
                if report.is_pass() {
                    Ok(Exit::Ok)
                } else {
                    // A failed ledger verification is the canonical assertion
                    // failure (exit 1).
                    Err(CliError::assertion("audit verify failed"))
                }
            }
            AuditCommand::VerifyChain { repo } => {
                match verify_audit_chain(&repo).map_err(preflight("read audit trail"))? {
                    ChainVerdict::Valid { rows, head_hash } => {
                        println!(
                            "{}",
                            pretty(&serde_json::json!({
                                "status": "valid",
                                "rows": rows,
                                "head_hash": head_hash,
                            }))?
                        );
                        Ok(Exit::Ok)
                    }
                    ChainVerdict::Broken(why) => Err(CliError::assertion(format!(
                        "audit-trail.jsonl chain broken: {why}"
                    ))),
                }
            }
        },
    }
}

/// Map any displayable error to a preflight failure (exit 3) with context. Used
/// for the store/core helpers that return either `anyhow::Error` or the crates'
/// own typed error.
fn preflight<E: std::fmt::Display>(context: &'static str) -> impl Fn(E) -> CliError {
    move |e| CliError::preflight(format!("{context}: {e}"))
}

fn pretty<T: serde::Serialize>(value: &T) -> CliResult<String> {
    serde_json::to_string_pretty(value).map_err(json_err("serialize output"))
}

fn artifact_of_file(repo_root: &Path, abs_or_rel: &Path) -> CliResult<Artifact> {
    let rel = abs_or_rel
        .strip_prefix(repo_root)
        .unwrap_or(abs_or_rel)
        .to_string_lossy()
        .replace('\\', "/");
    Artifact::of_file(&rel, abs_or_rel).map_err(|e| CliError::preflight(e.to_string()))
}

fn default_graph_path(paths: &StatePaths) -> PathBuf {
    driftlock_store::graph_path(paths)
}

fn seed_graph_from_canonical_ledger(paths: &StatePaths) -> CliResult<bool> {
    let graph_path = default_graph_path(paths);
    if graph_path.exists() {
        return Ok(false);
    }
    let canonical = paths.repo_root.join("tasks/taskgraph.json");
    if !canonical.exists() {
        return Ok(false);
    }
    let graph = read_graph_file(&canonical)?;
    save_graph(paths, &graph).map_err(preflight("seed graph"))?;
    Ok(true)
}

fn load_graph_for_repo(repo: &Path, graph: Option<PathBuf>) -> CliResult<TaskGraph> {
    if let Some(path) = graph {
        return read_graph_file(&path);
    }
    let paths = init_state_dir(repo).map_err(preflight("init state dir"))?;
    load_graph(&paths).map_err(preflight("load graph"))
}

fn read_graph_file(path: &Path) -> CliResult<TaskGraph> {
    let text = std::fs::read_to_string(path).map_err(io_err("read graph file"))?;
    serde_json::from_str(&text).map_err(json_err("parse graph file"))
}

fn write_graph_file(path: &Path, graph: &TaskGraph) -> CliResult<()> {
    std::fs::write(path, pretty(graph)?).map_err(io_err("write graph file"))?;
    Ok(())
}

fn find_task_mut<'a>(graph: &'a mut TaskGraph, task_id: &str) -> CliResult<&'a mut WorkOrder> {
    graph
        .tasks
        .iter_mut()
        .find(|t| t.id == task_id)
        .ok_or_else(|| CliError::usage("task not found"))
}

fn built_graph_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn touched_files(
    repo: &Path,
    diff_file: Option<PathBuf>,
    changed: Vec<String>,
) -> CliResult<Vec<String>> {
    if let Some(diff_file) = diff_file {
        let diff = std::fs::read_to_string(diff_file).map_err(io_err("read diff file"))?;
        return Ok(driftlock_git::changed_files_from_diff(&diff));
    }
    if !changed.is_empty() {
        return Ok(changed);
    }
    driftlock_git::git_changed_files(repo, "HEAD").map_err(preflight("list changed files"))
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
        std::fs::create_dir_all(repo.join("tasks")).unwrap();
        std::fs::write(
            repo.join("tasks/taskgraph.json"),
            include_str!("../../../tasks/taskgraph.json"),
        )
        .unwrap();

        let paths = init_state_dir(&repo).unwrap();
        assert!(seed_graph_from_canonical_ledger(&paths).unwrap());
        assert!(paths.state_dir.join("graph.json").is_file());
        let graph = load_graph(&paths).unwrap();
        assert!(!graph.tasks.is_empty());
        assert!(!seed_graph_from_canonical_ledger(&paths).unwrap());

        let _ = std::fs::remove_dir_all(repo);
    }
}
