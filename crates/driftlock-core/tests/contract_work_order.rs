#![allow(missing_docs)]

use driftlock_core::{AcceptanceGate, Confidence, EvidenceSpan, TaskStatus, WorkOrder};
use std::collections::BTreeMap;

#[test]
fn contract_work_order_roundtrip() {
    let json = include_str!("../../../contracts/examples/work-order.complete.json");
    let parsed: WorkOrder = serde_json::from_str(json).expect("example work order must parse");
    assert_eq!(parsed.id, "adr-0002:T01");
    assert_eq!(parsed.status, TaskStatus::Complete);
    assert!(!parsed.write_set.is_empty());
    // Legacy string acceptance entries deserialize to Advisory (back-compat).
    assert!(matches!(parsed.acceptance.first(), Some(AcceptanceGate::Advisory(_))));
}

#[test]
fn contract_typed_acceptance_gates_roundtrip() {
    let json = include_str!("../../../contracts/examples/work-order.gates.json");
    let parsed: WorkOrder = serde_json::from_str(json).expect("gates example must parse");
    assert_eq!(parsed.acceptance.len(), 4);
    assert!(matches!(parsed.acceptance[0], AcceptanceGate::FileExists { .. }));
    assert!(matches!(parsed.acceptance[1], AcceptanceGate::FileContains { .. }));
    assert!(matches!(parsed.acceptance[2], AcceptanceGate::Command { .. }));
    assert!(matches!(parsed.acceptance[3], AcceptanceGate::Advisory(_)));
    // Re-serialize and re-parse to prove the untagged round-trip is stable.
    let s = serde_json::to_string(&parsed.acceptance).unwrap();
    let back: Vec<AcceptanceGate> = serde_json::from_str(&s).unwrap();
    assert_eq!(back, parsed.acceptance);
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
