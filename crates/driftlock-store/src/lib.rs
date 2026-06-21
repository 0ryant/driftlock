//! `.driftlock` state directory: graph, claims, and audit events.

mod audit;
mod checkpoint;
mod claims;
mod events;
mod paths;
mod persistence;
mod receipt;
mod signing;

pub use audit::{
    append as append_audit, read_trail, trail_path, verify_chain as verify_audit_chain, AuditEntry,
    AuditRow, ChainVerdict, AUDIT_SCHEMA, GENESIS_HASH, TRAIL_FILENAME,
};
pub use checkpoint::{
    build_checkpoint, checkpoint_path, checkpoints_dir, load_checkpoint, resume_status,
    save_checkpoint, Checkpoint, FileResume, FileState, ResumeStatus, VerifiedFile,
    CHECKPOINT_SCHEMA,
};
pub use claims::{
    active_claim_for_task, complete_claim, load_claims, new_claim, record_claim, release_claim,
};
pub use events::{append_event, provenance_from_env, DriftlockEvent, EventKind, GENESIS_PREV_HASH};
pub use paths::{claims_path, events_path, graph_path, init_state_dir, StatePaths};
pub use persistence::{load_graph, save_graph};
pub use receipt::{
    build_signed as build_receipt, verify as verify_receipt, Artifact, AuditLink, Receipt,
    ReceiptBody, ReceiptInput, Verdict as ReceiptVerdict, RECEIPT_SCHEMA,
};
pub use signing::{
    chain_head, generate_operator_key, load_active_signing_key, record_hash, sign_event_line,
    trust_operator_key, verify_events, KeyInfo, SignedEventLine, VerifyReport,
};
