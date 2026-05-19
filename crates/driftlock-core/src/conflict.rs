//! Conflict detection.

use crate::graph::find_lane;
use crate::model::{
    Conflict, ConflictKind, ConflictSeverity, LaneManifest, TaskGraph, TaskStatus, WorkOrder,
};
use globset::{Glob, GlobSetBuilder};
use std::collections::{BTreeMap, BTreeSet};

/// Detects graph-level conflicts and returns a map keyed by task ID.
pub fn detect_graph_conflicts(graph: &TaskGraph) -> BTreeMap<String, Vec<Conflict>> {
    let mut map = detect_conflicts(&graph.tasks);
    let lanes =
        LaneManifest { schema_version: graph.schema_version.clone(), lanes: graph.lanes.clone() };
    merge_lane_violations(&mut map, &graph.tasks, &lanes);
    map
}

/// Detects same-write, glob-overlap, exclusive-resource, and contract-path conflicts.
pub fn detect_conflicts(tasks: &[WorkOrder]) -> BTreeMap<String, Vec<Conflict>> {
    let mut by_task: BTreeMap<String, Vec<Conflict>> = BTreeMap::new();

    for (idx, left) in tasks.iter().enumerate() {
        for right in tasks.iter().skip(idx + 1) {
            if left.status == TaskStatus::Complete && right.status == TaskStatus::Complete {
                continue;
            }

            let shared_writes = intersect(&left.write_set, &right.write_set);
            if !shared_writes.is_empty() {
                push_pair(
                    &mut by_task,
                    left,
                    right,
                    ConflictKind::SharedWrite,
                    ConflictSeverity::Hard,
                    format!(
                        "Shared write paths: {}",
                        shared_writes.into_iter().collect::<Vec<_>>().join(", ")
                    ),
                );
            } else if write_sets_overlap_glob(&left.write_set, &right.write_set) {
                push_pair(
                    &mut by_task,
                    left,
                    right,
                    ConflictKind::SharedWrite,
                    ConflictSeverity::Hard,
                    "Overlapping write-set globs".to_string(),
                );
            }

            let shared_exclusive = intersect(&left.exclusive_resources, &right.exclusive_resources);
            if !shared_exclusive.is_empty() {
                push_pair(
                    &mut by_task,
                    left,
                    right,
                    ConflictKind::ExclusiveResource,
                    ConflictSeverity::Hard,
                    format!(
                        "Shared exclusive resources: {}",
                        shared_exclusive.into_iter().collect::<Vec<_>>().join(", ")
                    ),
                );
            }

            if contract_paths_overlap(&left.write_set, &right.write_set) {
                push_pair(
                    &mut by_task,
                    left,
                    right,
                    ConflictKind::ApiContract,
                    ConflictSeverity::Hard,
                    "Both tasks may modify contract/schema paths".to_string(),
                );
            }
        }
    }

    by_task
}

/// Merges detected conflicts into task records (in place).
pub fn attach_conflicts_to_tasks(
    tasks: &mut [WorkOrder],
    conflicts: &BTreeMap<String, Vec<Conflict>>,
) {
    for task in tasks.iter_mut() {
        if let Some(c) = conflicts.get(&task.id) {
            task.conflicts = c.clone();
        }
    }
}

fn merge_lane_violations(
    map: &mut BTreeMap<String, Vec<Conflict>>,
    tasks: &[WorkOrder],
    lanes: &LaneManifest,
) {
    for task in tasks {
        let Some(lane) = find_lane(&lanes.lanes, &task.lane) else {
            map.entry(task.id.clone()).or_default().push(Conflict {
                task: task.id.clone(),
                kind: ConflictKind::LaneViolation,
                severity: ConflictSeverity::Unknown,
                reason: format!("Unknown lane {}", task.lane),
            });
            continue;
        };
        for path in &task.write_set {
            if !path_allowed(path, &lane.write_allow) {
                map.entry(task.id.clone()).or_default().push(Conflict {
                    task: task.id.clone(),
                    kind: ConflictKind::LaneViolation,
                    severity: ConflictSeverity::Hard,
                    reason: format!("Write path `{path}` outside lane `{}` allowlist", lane.id),
                });
            }
        }
    }
}

fn path_allowed(path: &str, allow: &[String]) -> bool {
    if allow.is_empty() {
        return false;
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in allow {
        if let Ok(g) = Glob::new(pattern) {
            builder.add(g);
        }
    }
    if let Ok(set) = builder.build() {
        return set.is_match(path);
    }
    false
}

fn write_sets_overlap_glob(left: &[String], right: &[String]) -> bool {
    let mut builder = GlobSetBuilder::new();
    for pattern in left {
        if let Ok(g) = Glob::new(pattern) {
            builder.add(g);
        }
    }
    let Ok(set) = builder.build() else {
        return false;
    };
    right.iter().any(|p| set.is_match(p))
}

fn contract_paths_overlap(left: &[String], right: &[String]) -> bool {
    let is_contract = |p: &str| p.contains("contracts/schemas") || p.contains("contracts/");
    left.iter().any(|l| is_contract(l) && right.iter().any(|r| is_contract(r)))
}

fn intersect(left: &[String], right: &[String]) -> BTreeSet<String> {
    let l: BTreeSet<_> = left.iter().cloned().collect();
    let r: BTreeSet<_> = right.iter().cloned().collect();
    l.intersection(&r).cloned().collect()
}

fn push_pair(
    map: &mut BTreeMap<String, Vec<Conflict>>,
    left: &WorkOrder,
    right: &WorkOrder,
    kind: ConflictKind,
    severity: ConflictSeverity,
    reason: String,
) {
    map.entry(left.id.clone()).or_default().push(Conflict {
        task: right.id.clone(),
        kind,
        severity,
        reason: reason.clone(),
    });
    map.entry(right.id.clone()).or_default().push(Conflict {
        task: left.id.clone(),
        kind,
        severity,
        reason,
    });
}

#[cfg(test)]
mod tests {
    use super::detect_conflicts;
    use crate::model::*;
    use std::collections::BTreeMap;

    fn task(id: &str, write: &str) -> WorkOrder {
        WorkOrder {
            id: id.to_string(),
            title: id.to_string(),
            source: EvidenceSpan {
                adr: "a".into(),
                adr_revision: "r".into(),
                section: "s".into(),
                start_line: 1,
                end_line: 1,
                evidence: None,
            },
            intent: "i".into(),
            lane: "core".into(),
            status: TaskStatus::Ready,
            write_set: vec![write.into()],
            read_set: vec![],
            exclusive_resources: vec![],
            deps: vec![],
            unlocks: vec![],
            conflicts: vec![],
            acceptance: vec![],
            non_goals: vec![],
            confidence: Confidence::high(),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn same_write_is_hard_conflict() {
        let conflicts = detect_conflicts(&[
            task("adr-0001:T01", "Cargo.toml"),
            task("adr-0001:T02", "Cargo.toml"),
        ]);
        assert_eq!(conflicts["adr-0001:T01"][0].severity, ConflictSeverity::Hard);
    }

    #[test]
    fn glob_overlap_detected() {
        let conflicts =
            detect_conflicts(&[task("adr-0001:T01", "src/**"), task("adr-0001:T02", "src/lib.rs")]);
        assert!(!conflicts.is_empty());
    }
}
