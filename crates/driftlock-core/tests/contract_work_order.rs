#![allow(missing_docs)]

use driftlock_core::{Confidence, EvidenceSpan, TaskStatus, WorkOrder};
use std::collections::BTreeMap;

#[test]
fn contract_work_order_roundtrip() {
    let json = include_str!("../../../contracts/examples/work-order.complete.json");
    let parsed: WorkOrder = serde_json::from_str(json).expect("example work order must parse");
    assert_eq!(parsed.id, "adr-0002:T01");
    assert_eq!(parsed.status, TaskStatus::Complete);
    assert!(!parsed.write_set.is_empty());
}

#[test]
fn work_order_requires_evidence_for_canonicality() {
    let task = WorkOrder {
        id: "adr-0001:T01".into(),
        title: "Test".into(),
        source: EvidenceSpan {
            adr: "docs/adrs/0001.md".into(),
            adr_revision: "r".into(),
            section: "Obligations".into(),
            start_line: 1,
            end_line: 1,
            evidence: Some("e".into()),
        },
        intent: "i".into(),
        lane: "core".into(),
        status: TaskStatus::Ready,
        write_set: vec!["src/**".into()],
        read_set: vec![],
        exclusive_resources: vec![],
        deps: vec![],
        unlocks: vec![],
        conflicts: vec![],
        acceptance: vec![],
        non_goals: vec![],
        confidence: Confidence::high(),
        metadata: BTreeMap::new(),
    };
    assert!(task.source.evidence.is_some());
}
