//! Core domain model and safety logic for Driftlock.

pub mod adr;
pub mod brief;
pub mod conflict;
pub mod diff;
pub mod error;
pub mod extractor;
pub mod graph;
pub mod model;
pub mod readiness;
pub mod schema;

pub use brief::render_agent_brief;
pub use conflict::{attach_conflicts_to_tasks, detect_conflicts, detect_graph_conflicts};
pub use diff::verify_changed_files;
pub use error::{DriftlockError, Result};
pub use extractor::{extract_work_orders_from_adr, load_lane_manifest};
pub use graph::{build_task_graph, find_task, promote_to_ready};
pub use model::*;
pub use readiness::{blocked_by_deps, ready_tasks, ready_tasks_for_base, unlocks_for};
