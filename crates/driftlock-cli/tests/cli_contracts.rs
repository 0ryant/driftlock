#![allow(missing_docs)]

#[test]
fn taskgraph_fixture_is_valid_json() {
    let json = include_str!("../../../tasks/taskgraph.json");
    let graph: driftlock_core::TaskGraph =
        serde_json::from_str(json).expect("taskgraph fixture parses");
    assert!(graph.tasks.iter().all(|task| task.status == driftlock_core::TaskStatus::Complete));
}
