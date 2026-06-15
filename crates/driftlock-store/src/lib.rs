//! `.driftlock` state directory: graph, claims, and audit events.

mod claims;
mod events;
mod paths;
mod persistence;
mod signing;

pub use claims::{
    active_claim_for_task, complete_claim, load_claims, new_claim, record_claim, release_claim,
};
pub use events::{append_event, provenance_from_env, DriftlockEvent, EventKind, GENESIS_PREV_HASH};
pub use paths::{claims_path, events_path, graph_path, init_state_dir, StatePaths};
pub use persistence::{load_graph, save_graph};
pub use signing::{
    chain_head, generate_operator_key, load_active_signing_key, record_hash, sign_event_line,
    trust_operator_key, verify_events, KeyInfo, SignedEventLine, VerifyReport,
};
