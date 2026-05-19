#![allow(missing_docs)]

#[test]
fn schema_bundle_contains_core_contracts() {
    let names: Vec<_> =
        driftlock_contracts::schema_bundle().into_iter().map(|s| s.file_name).collect();
    assert!(names.contains(&"work-order.generated.schema.json"));
    assert!(names.contains(&"taskgraph.generated.schema.json"));
}
