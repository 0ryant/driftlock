#![allow(missing_docs)]

use driftlock_store::{
    append_event, generate_operator_key, init_state_dir, verify_events, EventKind,
};
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn signed_event_roundtrip() {
    let dir = tempdir().unwrap();
    let paths = init_state_dir(dir.path()).unwrap();
    generate_operator_key(dir.path(), false).unwrap();
    append_event(&paths, EventKind::TaskClaimed, "test", Some("t-1"), BTreeMap::new()).unwrap();
    let report = verify_events(dir.path(), true).unwrap();
    assert!(report.is_pass(), "{report:?}");
    assert_eq!(report.rows_scanned, 1);
}
