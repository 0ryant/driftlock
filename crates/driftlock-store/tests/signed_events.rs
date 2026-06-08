#![allow(missing_docs)]

use driftlock_store::{
    append_event, generate_operator_key, init_state_dir, trust_operator_key, verify_events,
    EventKind,
};
use std::collections::BTreeMap;
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
