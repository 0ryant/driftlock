#![allow(missing_docs)]

use driftlock_core::{EvidenceSpan, TaskGraph, TaskStatus, WorkOrder};
use driftlock_store::{append_event, init_state_dir, load_graph, save_graph, EventKind};
use std::collections::BTreeMap;

#[test]
fn graph_round_trip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let paths = init_state_dir(tmp.path()).expect("init");
    let graph = TaskGraph {
        schema_version: "0.1.0".into(),
        graph_id: "g".into(),
        repo_root: ".".into(),
        base_ref: "abc".into(),
        generated_at: "now".into(),
        tasks: vec![WorkOrder {
            id: "adr-0001:T01".into(),
            title: "t".into(),
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
            status: TaskStatus::Ready,
            write_set: vec!["x.rs".into()],
            read_set: vec![],
            exclusive_resources: vec![],
            deps: vec![],
            unlocks: vec![],
            conflicts: vec![],
            acceptance: vec![],
            non_goals: vec![],
            confidence: driftlock_core::Confidence::high(),
            metadata: BTreeMap::new(),
        }],
        edges: vec![],
        lanes: vec![],
        metadata: BTreeMap::new(),
    };
    save_graph(&paths, &graph).expect("save");
    let loaded = load_graph(&paths).expect("load");
    assert_eq!(loaded.graph_id, "g");
    append_event(&paths, EventKind::GraphBuilt, "test", None, BTreeMap::new()).expect("event");
}
