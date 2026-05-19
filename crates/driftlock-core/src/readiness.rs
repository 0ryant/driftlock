//! Readiness queries.

use crate::model::{ConflictSeverity, TaskGraph, TaskStatus, WorkOrder};
use std::collections::BTreeSet;

/// Returns tasks ready for a lane against the graph's recorded base ref.
pub fn ready_tasks<'a>(graph: &'a TaskGraph, lane: &str) -> Vec<&'a WorkOrder> {
    ready_tasks_for_base(graph, lane, &graph.base_ref)
}

/// Returns tasks ready for a lane when `current_base_ref` matches the graph (or stale check disabled).
pub fn ready_tasks_for_base<'a>(
    graph: &'a TaskGraph,
    lane: &str,
    current_base_ref: &str,
) -> Vec<&'a WorkOrder> {
    let stale = graph.base_ref != current_base_ref
        && !graph.base_ref.is_empty()
        && graph.base_ref != "scaffold";
    let complete: BTreeSet<&str> = graph
        .tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Complete)
        .map(|task| task.id.as_str())
        .collect();

    graph
        .tasks
        .iter()
        .filter(|task| task.lane == lane)
        .filter(|task| matches!(task.status, TaskStatus::Ready | TaskStatus::Claimed))
        .filter(|_| !stale)
        .filter(|task| task.source.evidence.is_some())
        .filter(|task| task.confidence.is_ready_grade())
        .filter(|task| !task.write_set.is_empty())
        .filter(|task| task.deps.iter().all(|dep| complete.contains(dep.as_str())))
        .filter(|task| {
            !task
                .conflicts
                .iter()
                .any(|c| matches!(c.severity, ConflictSeverity::Hard | ConflictSeverity::Unknown))
        })
        .collect()
}

/// Task ids blocked by incomplete dependencies.
pub fn blocked_by_deps(graph: &TaskGraph, task_id: &str) -> Vec<String> {
    let Some(task) = graph.tasks.iter().find(|t| t.id == task_id) else {
        return Vec::new();
    };
    let complete: BTreeSet<_> = graph
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Complete)
        .map(|t| t.id.as_str())
        .collect();
    task.deps.iter().filter(|d| !complete.contains(d.as_str())).cloned().collect()
}

/// Downstream tasks unlocked when `task_id` completes.
pub fn unlocks_for(graph: &TaskGraph, task_id: &str) -> Vec<String> {
    graph
        .tasks
        .iter()
        .filter(|t| t.deps.iter().any(|d| d == task_id))
        .map(|t| t.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::ready_tasks_for_base;
    use crate::model::*;
    use std::collections::BTreeMap;

    fn task(id: &str, status: TaskStatus, deps: Vec<String>) -> WorkOrder {
        WorkOrder {
            id: id.into(),
            title: id.into(),
            source: EvidenceSpan {
                adr: "a".into(),
                adr_revision: "r".into(),
                section: "s".into(),
                start_line: 1,
                end_line: 1,
                evidence: Some("e".into()),
            },
            intent: "i".into(),
            lane: "core".into(),
            status,
            write_set: vec![format!("{id}.rs")],
            read_set: vec![],
            exclusive_resources: vec![],
            deps,
            unlocks: vec![],
            conflicts: vec![],
            acceptance: vec![],
            non_goals: vec![],
            confidence: Confidence::high(),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn dependency_must_be_complete() {
        let graph = TaskGraph {
            schema_version: "0.1.0".into(),
            graph_id: "g".into(),
            repo_root: ".".into(),
            base_ref: "main".into(),
            generated_at: "now".into(),
            tasks: vec![
                task("adr-0001:T01", TaskStatus::Ready, vec![]),
                task("adr-0001:T02", TaskStatus::Ready, vec!["adr-0001:T01".into()]),
            ],
            edges: vec![],
            lanes: vec![],
            metadata: BTreeMap::new(),
        };
        assert_eq!(ready_tasks_for_base(&graph, "core", "main").len(), 1);
    }

    #[test]
    fn stale_base_ref_blocks_ready() {
        let graph = TaskGraph {
            schema_version: "0.1.0".into(),
            graph_id: "g".into(),
            repo_root: ".".into(),
            base_ref: "abc123".into(),
            generated_at: "now".into(),
            tasks: vec![task("adr-0001:T01", TaskStatus::Ready, vec![])],
            edges: vec![],
            lanes: vec![],
            metadata: BTreeMap::new(),
        };
        assert!(ready_tasks_for_base(&graph, "core", "def456").is_empty());
    }
}
