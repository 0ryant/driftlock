//! `TaskGraph` helpers.

use crate::conflict::{attach_conflicts_to_tasks, detect_conflicts};
use crate::model::{
    GraphEdge, Lane, LaneManifest, TaskGraph, TaskStatus, WorkOrder, SCHEMA_VERSION,
};
use chrono::Utc;
use std::collections::BTreeMap;

/// Builds a graph from work orders and lanes; attaches conflict edges.
pub fn build_task_graph(
    graph_id: impl Into<String>,
    repo_root: impl Into<String>,
    base_ref: impl Into<String>,
    mut tasks: Vec<WorkOrder>,
    lanes: LaneManifest,
) -> TaskGraph {
    let conflicts = detect_conflicts(&tasks);
    attach_conflicts_to_tasks(&mut tasks, &conflicts);
    let edges = edges_from_tasks(&tasks);
    TaskGraph {
        schema_version: SCHEMA_VERSION.to_string(),
        graph_id: graph_id.into(),
        repo_root: repo_root.into(),
        base_ref: base_ref.into(),
        generated_at: Utc::now().to_rfc3339(),
        tasks,
        edges,
        lanes: lanes.lanes,
        metadata: BTreeMap::new(),
    }
}

/// Promotes a task to `ready` when confidence and write set allow.
pub fn promote_to_ready(task: &mut WorkOrder) {
    if task.write_set.is_empty() || task.source.evidence.is_none() {
        return;
    }
    if task.has_blocking_conflict() {
        return;
    }
    if task.confidence.is_ready_grade() {
        task.status = TaskStatus::Ready;
    }
}

/// Finds a task by ID.
pub fn find_task<'a>(graph: &'a TaskGraph, task_id: &str) -> Option<&'a WorkOrder> {
    graph.tasks.iter().find(|task| task.id == task_id)
}

/// Finds a lane by ID.
pub fn find_lane<'a>(lanes: &'a [Lane], lane_id: &str) -> Option<&'a Lane> {
    lanes.iter().find(|lane| lane.id == lane_id)
}

fn edges_from_tasks(tasks: &[WorkOrder]) -> Vec<GraphEdge> {
    let mut edges = Vec::new();
    for task in tasks {
        for dep in &task.deps {
            edges.push(GraphEdge {
                from: dep.clone(),
                to: task.id.clone(),
                kind: crate::model::EdgeKind::DependsOn,
                reason: "Declared dependency".to_string(),
            });
        }
        for unlock in &task.unlocks {
            edges.push(GraphEdge {
                from: task.id.clone(),
                to: unlock.clone(),
                kind: crate::model::EdgeKind::Unlocks,
                reason: "Declared downstream unlock".to_string(),
            });
        }
    }
    edges
}
