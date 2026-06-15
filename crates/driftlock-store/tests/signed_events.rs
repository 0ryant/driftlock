#![allow(missing_docs)]

use driftlock_store::{
    append_event, events_path, generate_operator_key, init_state_dir, trust_operator_key,
    verify_events, EventKind, GENESIS_PREV_HASH,
};
use std::collections::BTreeMap;
use std::fs;
use tempfile::tempdir;

#[test]
fn signed_event_roundtrip() {
    let dir = tempdir().unwrap();
    let paths = init_state_dir(dir.path()).unwrap();
    let info = generate_operator_key(dir.path(), false).unwrap();
    // Trust is now an explicit operator action (out-of-band fingerprint pin),
    // not an automatic side effect of key generation.
    trust_operator_key(dir.path(), &info.key_id).unwrap();
    append_event(&paths, EventKind::TaskClaimed, "test", Some("t-1"), BTreeMap::new()).unwrap();
    let report = verify_events(dir.path(), true).unwrap();
    assert!(report.is_pass(), "{report:?}");
    assert_eq!(report.rows_scanned, 1);
}

#[test]
fn untrusted_key_fails_signed_verification() {
    let dir = tempdir().unwrap();
    let paths = init_state_dir(dir.path()).unwrap();
    // Key generated but never explicitly trusted: signed verification must fail
    // because the trust store is not self-attesting.
    generate_operator_key(dir.path(), false).unwrap();
    append_event(&paths, EventKind::TaskClaimed, "test", Some("t-1"), BTreeMap::new()).unwrap();
    let report = verify_events(dir.path(), true).unwrap();
    assert!(!report.is_pass(), "untrusted key must not verify: {report:?}");
}

#[test]
fn trust_rejects_fingerprint_mismatch() {
    let dir = tempdir().unwrap();
    init_state_dir(dir.path()).unwrap();
    generate_operator_key(dir.path(), false).unwrap();
    assert!(trust_operator_key(dir.path(), "fp:deadbeef").is_err());
}

/// Helper: append N unsigned events to a fresh state dir and return the dir.
fn ledger_with(n: usize) -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let paths = init_state_dir(dir.path()).unwrap();
    for i in 0..n {
        append_event(
            &paths,
            EventKind::TaskClaimed,
            "test",
            Some(&format!("t-{i}")),
            BTreeMap::new(),
        )
        .unwrap();
    }
    dir
}

#[test]
fn first_row_links_to_genesis() {
    let dir = ledger_with(1);
    let paths = init_state_dir(dir.path()).unwrap();
    let text = fs::read_to_string(events_path(&paths)).unwrap();
    let first = text.lines().next().unwrap();
    assert!(
        first.contains(GENESIS_PREV_HASH),
        "first row must link to the genesis prev_hash: {first}"
    );
}

#[test]
fn intact_unsigned_chain_verifies() {
    let dir = ledger_with(3);
    // Unsigned verification (require_signed=false) still checks the hash chain.
    let report = verify_events(dir.path(), false).unwrap();
    assert!(report.is_pass(), "intact chain must verify: {report:?}");
    assert_eq!(report.rows_scanned, 3);
}

#[test]
fn deleting_a_middle_row_breaks_the_chain() {
    let dir = ledger_with(3);
    let paths = init_state_dir(dir.path()).unwrap();
    let path = events_path(&paths);
    let text = fs::read_to_string(&path).unwrap();
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    // Drop the middle row; rows 0 and 2 survive but no longer chain.
    let kept = format!("{}\n{}\n", lines[0], lines[2]);
    fs::write(&path, kept).unwrap();
    let report = verify_events(dir.path(), false).unwrap();
    assert!(!report.is_pass(), "deleting a row must break the chain: {report:?}");
    assert!(
        report.failures.iter().any(|f| f.contains("broken hash chain")),
        "expected a broken-chain failure, got: {report:?}"
    );
}

#[test]
fn truncating_the_last_row_is_detected_only_with_an_anchor() {
    // Truncation of the tail is the classic hash-chain blind spot: a contiguous
    // prefix still chains cleanly. This test documents that property — the
    // remaining prefix verifies — so the limitation is explicit, not silent.
    let dir = ledger_with(3);
    let paths = init_state_dir(dir.path()).unwrap();
    let path = events_path(&paths);
    let text = fs::read_to_string(&path).unwrap();
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let kept = format!("{}\n{}\n", lines[0], lines[1]);
    fs::write(&path, kept).unwrap();
    let report = verify_events(dir.path(), false).unwrap();
    // Prefix still chains; rows_scanned reflects the truncation.
    assert!(report.is_pass(), "a contiguous prefix still verifies: {report:?}");
    assert_eq!(report.rows_scanned, 2);
}

#[test]
fn reordering_rows_breaks_the_chain() {
    let dir = ledger_with(3);
    let paths = init_state_dir(dir.path()).unwrap();
    let path = events_path(&paths);
    let text = fs::read_to_string(&path).unwrap();
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    // Swap rows 1 and 2.
    let reordered = format!("{}\n{}\n{}\n", lines[0], lines[2], lines[1]);
    fs::write(&path, reordered).unwrap();
    let report = verify_events(dir.path(), false).unwrap();
    assert!(!report.is_pass(), "reordering must break the chain: {report:?}");
    assert!(report.failures.iter().any(|f| f.contains("broken hash chain")));
}

#[test]
fn editing_a_row_payload_breaks_the_chain_for_following_rows() {
    let dir = ledger_with(3);
    let paths = init_state_dir(dir.path()).unwrap();
    let path = events_path(&paths);
    let text = fs::read_to_string(&path).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    // Tamper with row 0's actor; its record hash changes, so row 1's prev_hash
    // no longer matches.
    lines[0] = lines[0].replace("\"actor\":\"test\"", "\"actor\":\"attacker\"");
    fs::write(&path, format!("{}\n{}\n{}\n", lines[0], lines[1], lines[2])).unwrap();
    let report = verify_events(dir.path(), false).unwrap();
    assert!(!report.is_pass(), "editing a row must break the chain: {report:?}");
    assert!(report.failures.iter().any(|f| f.contains("broken hash chain")));
}

#[test]
fn signed_chain_also_verifies_end_to_end() {
    let dir = tempdir().unwrap();
    let paths = init_state_dir(dir.path()).unwrap();
    let info = generate_operator_key(dir.path(), false).unwrap();
    trust_operator_key(dir.path(), &info.key_id).unwrap();
    for i in 0..3 {
        append_event(
            &paths,
            EventKind::TaskClaimed,
            "test",
            Some(&format!("t-{i}")),
            BTreeMap::new(),
        )
        .unwrap();
    }
    let report = verify_events(dir.path(), true).unwrap();
    assert!(report.is_pass(), "signed + chained ledger must verify: {report:?}");
    assert_eq!(report.rows_scanned, 3);
}
