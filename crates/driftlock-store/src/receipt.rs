//! The `axiom.receipt.v1` receipt: a signed, content-addressed record of a single
//! mutating Driftlock operation.
//!
//! Per ecosystem-catalog pattern 07 (receipt-emission) the load-bearing fact a
//! tool emits is a *signed* one: a verifier can re-derive deterministic reasoning,
//! but cannot forge an Ed25519 signature under a key it does not hold. The receipt
//! is a rich `axiom.receipt.v1` object — schema/tool/operation/outcome,
//! inputs/outputs carrying BLAKE3 digests, and audit-chain linkage — NOT a 2-line
//! text file. The signed body is canonicalized with RFC 8785 (JCS) (via
//! [`axiom_receipt`]) before signing so any verifier recomputes identical bytes.
//!
//! Receipts are signed under the operator's active `.driftlock` signing key (the
//! same key the signed-event ledger uses). When no key is present the body is
//! still emitted unsigned (`signature == ""`); [`verify`] reports such a receipt
//! as `Unsigned` rather than `Valid`.

use std::path::Path;

use anyhow::{Context, Result};
use axiom_receipt::KeyClass;
use serde::{Deserialize, Serialize};

use crate::key;
use crate::signing::load_active_signing_key;

/// Receipt schema version. Verifiers reject anything else.
pub const RECEIPT_SCHEMA: &str = "axiom.receipt.v1";

/// Canonical tool name embedded in every receipt.
pub const TOOL_NAME: &str = "driftlock";

/// Tool version embedded in receipts.
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A content-addressed input or output: a path plus its BLAKE3 digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// `"file" | "dir"`.
    pub kind: String,
    /// Path (repo-relative for repo artifacts).
    pub path: String,
    /// Lowercase-hex BLAKE3 of the artifact's content.
    pub blake3: String,
}

impl Artifact {
    /// Content-address a file (BLAKE3) and build a `"file"` artifact.
    pub fn of_file(repo_relative: &str, abs_path: &Path) -> Result<Self> {
        Ok(Self {
            kind: "file".to_string(),
            path: repo_relative.to_string(),
            blake3: axiom_hash::blake3_file(abs_path)
                .with_context(|| format!("hash artifact {}", abs_path.display()))?,
        })
    }

    /// Build an artifact from already-known content bytes.
    #[must_use]
    pub fn of_bytes(kind: &str, path: &str, bytes: &[u8]) -> Self {
        Self {
            kind: kind.to_string(),
            path: path.to_string(),
            blake3: axiom_hash::blake3_hex(bytes),
        }
    }
}

/// Audit-chain linkage embedded in the receipt (pattern 07 `audit_chain`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLink {
    /// Repo-relative path to the trail file (`audit-trail.jsonl`).
    pub trail_path: String,
    /// `seq` of the audit row this operation appended.
    pub seq: u64,
    /// `row_hash` of the appended row (the trail tip after this operation).
    pub row_hash: String,
}

/// The canonical, signed body of a receipt. Everything a verifier needs to
/// reconstruct the claim lives here; the signature is over the JCS canonical
/// bytes of exactly this struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptBody {
    /// Schema tag, always [`RECEIPT_SCHEMA`].
    pub schema: String,
    /// Canonical tool name, always [`TOOL_NAME`].
    pub tool: String,
    /// Tool semver.
    pub tool_version: String,
    /// Operation that produced the receipt (e.g. `"build-graph"`, `"complete"`).
    pub operation: String,
    /// Pattern-07 outcome vocabulary: `"ok" | "failed" | "degraded"`.
    pub outcome: String,
    /// Process exit code (pattern 11) for the operation.
    pub exit_code: i32,
    /// Inputs operated on, each with a BLAKE3 digest.
    pub inputs: Vec<Artifact>,
    /// Outputs produced, each with a BLAKE3 digest.
    pub outputs: Vec<Artifact>,
    /// Audit-chain linkage; `None` if no trail row was written.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub audit_chain: Option<AuditLink>,
    /// RFC 3339 creation timestamp.
    pub created_at: String,
    /// Free-form attribution of who/what produced this receipt.
    pub created_by: String,
    /// Signing-key tier: `dev` (the operator's `.driftlock` key — mechanism, not
    /// origin) or `deployment` (a configured `DRIFTLOCK_SIGNING_SEED_HEX` key —
    /// origin-grade once its public half is pinned in the trust store). Stamped
    /// inside the signed body so it cannot be relabelled after signing. An unsigned
    /// receipt (no key at emit time) still defaults to `dev`.
    pub key_class: KeyClass,
}

impl ReceiptBody {
    /// JCS canonical bytes for signing/verification.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        Ok(axiom_receipt::jcs_signing_bytes(self)?)
    }
}

/// A complete receipt: the canonical body plus its detached signature and the
/// `key_id` of the signing key (empty when emitted unsigned).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// The signed body.
    pub body: ReceiptBody,
    /// Lowercase-hex Ed25519 signature over [`ReceiptBody::canonical_bytes`], or
    /// `""` when no active signing key was present.
    pub signature: String,
    /// Fingerprint of the signing key, or `""` when unsigned.
    pub key_id: String,
}

impl Receipt {
    /// Serialize to pretty JSON suitable for writing to disk.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parse a receipt from JSON.
    pub fn from_json(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}

/// The operation facts that go into an [`axiom.receipt.v1`](RECEIPT_SCHEMA) body.
/// The chain/identity fields (`schema`/`tool`/`tool_version`/`created_at`) are
/// filled in by [`build_signed`].
#[derive(Debug, Clone)]
pub struct ReceiptInput {
    /// Operation that produced the receipt (e.g. `"build-graph"`, `"complete"`).
    pub operation: String,
    /// Pattern-07 outcome vocabulary: `"ok" | "failed" | "degraded"`.
    pub outcome: String,
    /// Process exit code (pattern 11) for the operation.
    pub exit_code: i32,
    /// Inputs operated on, each content-addressed.
    pub inputs: Vec<Artifact>,
    /// Outputs produced, each content-addressed.
    pub outputs: Vec<Artifact>,
    /// Audit-chain linkage, if a trail row was written.
    pub audit_chain: Option<AuditLink>,
    /// Free-form attribution of who/what produced this receipt.
    pub created_by: String,
}

/// Build and sign an `axiom.receipt.v1` receipt for one operation.
///
/// The signing key is resolved through [`crate::key`]: a deployment key from
/// `DRIFTLOCK_SIGNING_SEED_HEX` takes precedence ([`KeyClass::Deployment`]), else
/// the operator's active `.driftlock` key ([`KeyClass::Dev`]). The active
/// [`KeyClass`] is stamped INTO the signed body. When neither key is available the
/// receipt is emitted unsigned (`signature`/`key_id` empty) and self-labels `dev`.
/// The `key_id` is the deployment key-id, or the BLAKE3 fingerprint of the
/// operator verifying key (matching [`crate::signing`]).
pub fn build_signed(repo_root: &Path, input: ReceiptInput) -> Result<Receipt> {
    // Resolve the active signing key. A deployment seed in the environment signs
    // over an operator-key baseline; with no operator key, only a deployment seed
    // can sign, otherwise the receipt is unsigned.
    let active = match load_active_signing_key(repo_root)? {
        Some(sk) => {
            let operator_seed = sk.to_bytes();
            let operator_key_id = key_fingerprint(&sk.verifying_key().to_bytes());
            let signer = key::active_signer(operator_seed, operator_key_id.clone());
            let key_id = key::active_key_id(operator_seed, operator_key_id.clone());
            let key_class = key::active_key_class(operator_seed, operator_key_id);
            Some((signer, key_id, key_class))
        }
        None => deployment_only_signer(),
    };

    // The body self-labels the active key class even when emitted unsigned (no
    // key present defaults to dev), so a verifier always reads an origin claim.
    let key_class = active.as_ref().map_or(KeyClass::Dev, |(_, _, c)| *c);
    let body = ReceiptBody {
        schema: RECEIPT_SCHEMA.to_string(),
        tool: TOOL_NAME.to_string(),
        tool_version: TOOL_VERSION.to_string(),
        operation: input.operation,
        outcome: input.outcome,
        exit_code: input.exit_code,
        inputs: input.inputs,
        outputs: input.outputs,
        audit_chain: input.audit_chain,
        created_at: chrono::Utc::now().to_rfc3339(),
        created_by: input.created_by,
        key_class,
    };

    match active {
        Some((signer, key_id, _)) => {
            let (sig, _) = axiom_receipt::sign_bytes(&axiom_receipt::Jcs(&body), &signer)
                .context("sign receipt body")?;
            Ok(Receipt { body, signature: hex::encode(sig), key_id })
        }
        None => Ok(Receipt { body, signature: String::new(), key_id: String::new() }),
    }
}

/// When no operator key is on disk, a `DRIFTLOCK_SIGNING_SEED_HEX` deployment seed
/// can still sign. Returns the deployment signer/key-id/class, or `None` to emit
/// unsigned. The deployment seed doubles as the keyring's baseline here (it is the
/// active key either way), so the resolved class is always [`KeyClass::Deployment`]
/// when a valid seed is present.
fn deployment_only_signer() -> Option<(axiom_receipt::Ed25519Signer, String, KeyClass)> {
    let seed_hex = std::env::var(format!("{}_SIGNING_SEED_HEX", key::ENV_PREFIX)).ok()?;
    let key_id_env = std::env::var(format!("{}_SIGNING_KEY_ID", key::ENV_PREFIX)).ok();
    let (seed, key_id) =
        axiom_receipt::resolve_seed(Some(seed_hex), key_id_env, key::DEPLOYMENT_KEY_ID_DEFAULT)?;
    let signer = axiom_receipt::Ed25519Signer::from_seed(seed, key_id.clone());
    Some((signer, key_id, KeyClass::Deployment))
}

/// Typed verdict from [`verify`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Schema is `axiom.receipt.v1` and the Ed25519 signature verifies under the
    /// embedded key.
    Valid,
    /// Schema is correct but the receipt carries no signature (no signing key was
    /// present at emit time). Fail-closed: this is NOT `Valid`.
    Unsigned,
    /// Verification failed; the string explains why.
    Invalid(String),
}

impl Verdict {
    /// True only for [`Verdict::Valid`].
    #[must_use]
    pub fn is_valid(&self) -> bool {
        matches!(self, Verdict::Valid)
    }
}

/// Offline-verify a receipt:
/// 1. schema must be `axiom.receipt.v1`;
/// 2. an unsigned receipt is reported as [`Verdict::Unsigned`] (fail-closed);
/// 3. otherwise the Ed25519 signature must verify over the JCS canonical body
///    under the trusted public key for `key_id`.
///
/// Returns a typed [`Verdict`]; never panics.
pub fn verify(repo_root: &Path, receipt: &Receipt) -> Result<Verdict> {
    if receipt.body.schema != RECEIPT_SCHEMA {
        return Ok(Verdict::Invalid(format!("unsupported schema: {}", receipt.body.schema)));
    }
    if receipt.signature.is_empty() || receipt.key_id.is_empty() {
        return Ok(Verdict::Unsigned);
    }
    let Some(pubkey) = crate::signing::load_trust_pubkey(repo_root, &receipt.key_id)? else {
        return Ok(Verdict::Invalid(format!("unknown key_id: {}", receipt.key_id)));
    };
    let verifier = match axiom_receipt::Ed25519Verifier::from_pubkey(pubkey.to_bytes()) {
        Ok(v) => v,
        Err(e) => return Ok(Verdict::Invalid(format!("bad trust pubkey: {e}"))),
    };
    let Ok(sig_bytes) = hex::decode(&receipt.signature) else {
        return Ok(Verdict::Invalid("signature not hex".to_string()));
    };
    let Ok(sig) = <[u8; 64]>::try_from(sig_bytes) else {
        return Ok(Verdict::Invalid("signature wrong length".to_string()));
    };
    let bytes = receipt.body.canonical_bytes()?;
    match axiom_receipt::verify_bytes(&axiom_receipt::RawBytes(&bytes), &sig, &verifier) {
        Ok(()) => Ok(Verdict::Valid),
        Err(e) => Ok(Verdict::Invalid(e.to_string())),
    }
}

/// BLAKE3 fingerprint of an Ed25519 public key (matches [`crate::signing`]).
fn key_fingerprint(pubkey_bytes: &[u8; 32]) -> String {
    let digest = axiom_hash::blake3_hex(pubkey_bytes);
    format!("fp:{}", &digest[..32])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::{generate_operator_key, trust_operator_key};

    fn inputs() -> Vec<Artifact> {
        vec![Artifact::of_bytes("file", "ADR-0001.md", b"adr text")]
    }

    fn input(
        operation: &str,
        outputs: Vec<Artifact>,
        audit_chain: Option<AuditLink>,
    ) -> ReceiptInput {
        ReceiptInput {
            operation: operation.to_string(),
            outcome: "ok".to_string(),
            exit_code: 0,
            inputs: inputs(),
            outputs,
            audit_chain,
            created_by: "cli".to_string(),
        }
    }

    #[test]
    fn unsigned_when_no_key() {
        let dir = tempfile::tempdir().unwrap();
        crate::init_state_dir(dir.path()).unwrap();
        let r = build_signed(dir.path(), input("build-graph", vec![], None)).unwrap();
        assert_eq!(r.signature, "");
        assert_eq!(verify(dir.path(), &r).unwrap(), Verdict::Unsigned);
        assert_eq!(r.body.schema, RECEIPT_SCHEMA);
        assert_eq!(r.body.tool, TOOL_NAME);
    }

    #[test]
    fn signed_receipt_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        crate::init_state_dir(dir.path()).unwrap();
        let info = generate_operator_key(dir.path(), false).unwrap();
        trust_operator_key(dir.path(), &info.key_id).unwrap();
        let r = build_signed(
            dir.path(),
            input(
                "complete",
                vec![Artifact::of_bytes("file", "out.json", b"{}")],
                Some(AuditLink {
                    trail_path: "audit-trail.jsonl".to_string(),
                    seq: 0,
                    row_hash: "ab".repeat(32),
                }),
            ),
        )
        .unwrap();
        assert_eq!(r.body.operation, "complete");
        assert!(verify(dir.path(), &r).unwrap().is_valid());

        // JSON round-trip preserves verifiability.
        let json = r.to_json().unwrap();
        let parsed = Receipt::from_json(&json).unwrap();
        assert!(verify(dir.path(), &parsed).unwrap().is_valid());
    }

    #[test]
    fn tampered_body_fails_verification() {
        let dir = tempfile::tempdir().unwrap();
        crate::init_state_dir(dir.path()).unwrap();
        let info = generate_operator_key(dir.path(), false).unwrap();
        trust_operator_key(dir.path(), &info.key_id).unwrap();
        let mut r = build_signed(dir.path(), input("claim", vec![], None)).unwrap();
        r.body.exit_code = 1; // tamper after signing.
        assert!(matches!(verify(dir.path(), &r).unwrap(), Verdict::Invalid(_)));
    }

    #[test]
    fn key_class_defaults_dev_and_is_tamper_proof() {
        // With no DRIFTLOCK_SIGNING_SEED_HEX env, the active key is the operator's
        // generated `.driftlock` key, so a fresh signed receipt self-labels `dev`
        // (mechanism, not origin).
        let dir = tempfile::tempdir().unwrap();
        crate::init_state_dir(dir.path()).unwrap();
        let info = generate_operator_key(dir.path(), false).unwrap();
        trust_operator_key(dir.path(), &info.key_id).unwrap();
        let r = build_signed(dir.path(), input("complete", vec![], None)).unwrap();
        assert_eq!(r.body.key_class, KeyClass::Dev);
        assert!(verify(dir.path(), &r).unwrap().is_valid());

        // key_class lives INSIDE the signed body — relabelling a dev receipt as
        // `deployment` after signing breaks the signature (fail-closed).
        let mut forged = r.clone();
        forged.body.key_class = KeyClass::Deployment;
        assert!(matches!(verify(dir.path(), &forged).unwrap(), Verdict::Invalid(_)));
    }
}
