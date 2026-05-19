//! Schema generation helpers.

use crate::model::{Claim, DiffReport, LaneManifest, TaskGraph, WorkOrder};
use schemars::{schema::RootSchema, schema_for};

/// Returns `WorkOrder` schema.
pub fn work_order_schema() -> RootSchema {
    schema_for!(WorkOrder)
}

/// Returns `TaskGraph` schema.
pub fn taskgraph_schema() -> RootSchema {
    schema_for!(TaskGraph)
}

/// Returns `LaneManifest` schema.
pub fn lane_manifest_schema() -> RootSchema {
    schema_for!(LaneManifest)
}

/// Returns `Claim` schema.
pub fn claim_schema() -> RootSchema {
    schema_for!(Claim)
}

/// Returns `DiffReport` schema.
pub fn diff_report_schema() -> RootSchema {
    schema_for!(DiffReport)
}
