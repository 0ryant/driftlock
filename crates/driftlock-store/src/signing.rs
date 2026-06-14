//! Ed25519 signing for audit event lines.

use crate::events::{DriftlockEvent, GENESIS_PREV_HASH};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SIGN_DOMAIN: &str = "driftlock:events:sign:v1:";
const CHAIN_DOMAIN: &str = "driftlock:events:chain:v1:";

/// Signed JSONL envelope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedEventLine {
    /// Event payload.
    pub payload: DriftlockEvent,
    /// Detached signature metadata.
    pub signature: EventSignature,
}

/// Signature block on an event line.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventSignature {
    /// Key fingerprint.
    pub key_id: String,
    /// RFC3339 signing time.
    pub signed_at: String,
    /// Base64 signature bytes.
    pub bytes: String,
}

/// Key metadata for operators.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyInfo {
    /// Path to secret key file.
    pub path: PathBuf,
    /// Fingerprint id.
    pub key_id: String,
    /// Hex-encoded public key.
    pub public_key_hex: String,
}

/// Verification report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerifyReport {
    /// pass or fail.
    pub status: String,
    /// Rows scanned.
    pub rows_scanned: usize,
    /// Failure messages.
    pub failures: Vec<String>,
}

impl VerifyReport {
    /// Returns true when verification passed.
    pub fn is_pass(&self) -> bool {
        self.status == "pass"
    }
}

/// Returns `.driftlock/keys/active.ed25519` under repo.
pub fn active_signing_key_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".driftlock/keys/active.ed25519")
}

/// Returns trust directory for public keys.
pub fn trust_keys_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".driftlock/keys/trust")
}

/// Generates operator signing key under `.driftlock/keys/`.
///
/// Key generation does NOT automatically add the new public key to the trust
/// store: a self-attesting trust store provides no tamper-evidence because any
/// actor who can write the repo could mint a key and "trust" it. Adding a key to
/// the trust set is a separate, explicit operator action via
/// [`trust_operator_key`] (and the `driftlock key trust <fingerprint>` CLI),
/// which pins the fingerprint out-of-band.
pub fn generate_operator_key(repo_root: &Path, force: bool) -> Result<KeyInfo> {
    let key_path = active_signing_key_path(repo_root);
    if key_path.exists() && !force {
        anyhow::bail!("signing key already exists at {}; use force to rotate", key_path.display());
    }
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)?;
        harden_dir_permissions(parent)?;
    }
    let signing_key = SigningKey::generate(&mut OsRng);
    write_secret_file(&key_path, signing_key.to_bytes().as_slice())?;
    Ok(key_info(&key_path, &signing_key))
}

/// Adds the active key's public half to the trust store after the operator
/// confirms its fingerprint out-of-band.
///
/// `expected_fingerprint` must equal the active key's fingerprint; the caller is
/// expected to have obtained it from a trusted channel (e.g. the output of
/// `key generate`, reviewed by a human) rather than from the repo itself. This
/// makes trust an explicit, verifiable action instead of self-attestation.
pub fn trust_operator_key(repo_root: &Path, expected_fingerprint: &str) -> Result<KeyInfo> {
    let signing_key = load_active_signing_key(repo_root)?
        .context("no active signing key to trust; run `key generate` first")?;
    let verifying_key = signing_key.verifying_key();
    let actual = key_fingerprint(&verifying_key);
    if actual != expected_fingerprint {
        anyhow::bail!(
            "fingerprint mismatch: active key is {actual}, operator supplied {expected_fingerprint}"
        );
    }
    publish_trust_pubkey(repo_root, &verifying_key)?;
    Ok(key_info(&active_signing_key_path(repo_root), &signing_key))
}

/// Loads active signing key if present.
pub fn load_active_signing_key(repo_root: &Path) -> Result<Option<SigningKey>> {
    let path = active_signing_key_path(repo_root);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read(&path)?;
    if raw.len() != 32 {
        anyhow::bail!("signing key must be 32 bytes");
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&raw);
    Ok(Some(SigningKey::from_bytes(&bytes)))
}

/// Signs an event for JSONL storage.
pub fn sign_event_line(
    event: &DriftlockEvent,
    signing_key: &SigningKey,
) -> Result<SignedEventLine> {
    let verifying_key = signing_key.verifying_key();
    let key_id = key_fingerprint(&verifying_key);
    let sig = signing_key.sign(&signing_preimage(event)?);
    Ok(SignedEventLine {
        payload: event.clone(),
        signature: EventSignature {
            key_id,
            signed_at: event.at.clone(),
            bytes: STANDARD.encode(sig.to_bytes()),
        },
    })
}

/// Verifies all lines in `events.jsonl`.
pub fn verify_events(repo_root: &Path, require_signed: bool) -> Result<VerifyReport> {
    let path = repo_root.join(".driftlock/events.jsonl");
    let mut report = VerifyReport { status: "pass".into(), rows_scanned: 0, failures: Vec::new() };
    if !path.exists() {
        if require_signed {
            report.status = "fail".into();
            report.failures.push("events.jsonl missing".into());
        }
        return Ok(report);
    }
    let text = fs::read_to_string(&path)?;
    // Tracks the `prev_hash` the next non-empty row must carry. Seeded with the
    // genesis link so deletion of the original first row is caught (the new
    // first row's `prev_hash` would point at a record, not at genesis).
    let mut expected_prev = GENESIS_PREV_HASH.to_string();
    for (idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        report.rows_scanned += 1;
        let line_no = idx + 1;

        // Recover the underlying event for both the chain check and the
        // signed/unsigned handling below.
        let parsed_event = parse_event_line(line).ok();

        // Chain linkage is verified for every row regardless of signing: this
        // is the property that detects truncation, reordering, and deletion —
        // exactly what independent per-row signatures cannot see.
        match &parsed_event {
            Some(event) => {
                if event.prev_hash != expected_prev {
                    report.status = "fail".into();
                    report.failures.push(format!(
                        "line {line_no}: broken hash chain (prev_hash {} != expected {})",
                        event.prev_hash, expected_prev
                    ));
                }
                // Advance the head to this row's record hash even on mismatch so
                // a single edit produces one chain failure, not a cascade.
                expected_prev = record_hash(event)?;
            }
            None => {
                report.status = "fail".into();
                report.failures.push(format!("line {line_no}: invalid event json"));
                continue;
            }
        }

        if let Ok(signed) = serde_json::from_str::<SignedEventLine>(line) {
            if let Err(reason) = verify_signed_line(repo_root, &signed) {
                report.status = "fail".into();
                report.failures.push(format!("line {line_no}: {reason}"));
            }
            continue;
        }
        if require_signed {
            report.status = "fail".into();
            report.failures.push(format!("line {line_no}: unsigned row"));
        }
    }
    Ok(report)
}

fn verify_signed_line(repo_root: &Path, line: &SignedEventLine) -> Result<(), String> {
    let pubkey = load_trust_pubkey(repo_root, &line.signature.key_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "unknown key_id".to_string())?;
    let bytes =
        STANDARD.decode(&line.signature.bytes).map_err(|_| "bad signature encoding".to_string())?;
    let signature = ed25519_dalek::Signature::from_slice(&bytes)
        .map_err(|_| "bad signature bytes".to_string())?;
    pubkey
        .verify(&signing_preimage(&line.payload).map_err(|e| e.to_string())?, &signature)
        .map_err(|_| "signature mismatch".to_string())?;
    Ok(())
}

/// Builds the bytes that are signed/verified for an event.
///
/// NOTE: this is bound to `serde_json`'s current output, not a formal
/// canonical-JSON encoding. Stability relies on `DriftlockEvent`'s field order
/// being fixed and `metadata` being a `BTreeMap` (sorted keys). Any change to
/// the struct's serialized shape changes this preimage and invalidates
/// previously-signed lines. The regression test `preimage_is_byte_stable`
/// pins the exact bytes for a fixed event so such a change cannot land silently.
fn signing_preimage(event: &DriftlockEvent) -> Result<Vec<u8>> {
    let serialized = serde_json::to_string(event)?;
    let hash = Sha256::digest(serialized.as_bytes());
    Ok(format!("{SIGN_DOMAIN}{}", hex::encode(hash)).into_bytes())
}

/// Domain-separated hex SHA-256 over an event's canonical bytes.
///
/// This is the value the *next* row carries in its `prev_hash` field, forming
/// the audit hash chain. It is computed over the same `serde_json` bytes the
/// signing preimage is bound to (so the `prev_hash` link is itself covered by
/// any signature), with a distinct domain tag so a record hash can never be
/// confused with a signing preimage.
pub fn record_hash(event: &DriftlockEvent) -> Result<String> {
    let serialized = serde_json::to_string(event)?;
    let mut hasher = Sha256::new();
    hasher.update(CHAIN_DOMAIN.as_bytes());
    hasher.update(serialized.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

/// Reads the chain head (the `prev_hash` the next appended row must carry) from
/// an existing `events.jsonl`, or the genesis link when the ledger is empty.
///
/// Rows may be signed ([`SignedEventLine`]) or bare [`DriftlockEvent`]; both are
/// understood so chaining is independent of whether signing is enabled.
pub fn chain_head(events_path: &Path) -> Result<String> {
    if !events_path.exists() {
        return Ok(GENESIS_PREV_HASH.to_string());
    }
    let text = fs::read_to_string(events_path)?;
    let mut head = GENESIS_PREV_HASH.to_string();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let event = parse_event_line(line)?;
        head = record_hash(&event)?;
    }
    Ok(head)
}

/// Parses one JSONL row as its underlying [`DriftlockEvent`], whether the row is
/// a signed envelope or a bare event.
fn parse_event_line(line: &str) -> Result<DriftlockEvent> {
    if let Ok(signed) = serde_json::from_str::<SignedEventLine>(line) {
        return Ok(signed.payload);
    }
    serde_json::from_str::<DriftlockEvent>(line).context("invalid event json")
}

fn key_fingerprint(pubkey: &VerifyingKey) -> String {
    let digest = Sha256::digest(pubkey.as_bytes());
    format!("fp:{}", hex::encode(&digest[..16]))
}

fn publish_trust_pubkey(repo_root: &Path, pubkey: &VerifyingKey) -> Result<()> {
    let dir = trust_keys_dir(repo_root);
    fs::create_dir_all(&dir)?;
    let id = key_fingerprint(pubkey);
    let file = dir.join(format!("{id}.pub"));
    fs::write(file, hex::encode(pubkey.as_bytes()))?;
    Ok(())
}

fn load_trust_pubkey(repo_root: &Path, key_id: &str) -> Result<Option<VerifyingKey>> {
    let file = trust_keys_dir(repo_root).join(format!("{key_id}.pub"));
    if !file.exists() {
        return Ok(None);
    }
    let hex_str = fs::read_to_string(&file)?.trim().to_string();
    let bytes = hex::decode(hex_str).context("decode trust pubkey")?;
    if bytes.len() != 32 {
        anyhow::bail!("trust pubkey wrong length");
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(Some(VerifyingKey::from_bytes(&arr)?))
}

/// Writes secret bytes to `path`, creating the file with owner-only (0600)
/// permissions on Unix before any bytes are written.
fn write_secret_file(path: &Path, bytes: &[u8]) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .with_context(|| format!("creating secret file {}", path.display()))?;
        file.write_all(bytes)?;
        // Re-assert mode in case the file pre-existed with looser permissions.
        fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        fs::write(path, bytes)?;
    }
    Ok(())
}

/// Restricts a directory to owner-only (0700) access on Unix.
fn harden_dir_permissions(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .with_context(|| format!("hardening directory {}", dir.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
    Ok(())
}

fn key_info(path: &Path, signing_key: &SigningKey) -> KeyInfo {
    let verifying_key = signing_key.verifying_key();
    KeyInfo {
        path: path.to_path_buf(),
        key_id: key_fingerprint(&verifying_key),
        public_key_hex: hex::encode(verifying_key.as_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::{record_hash, signing_preimage};
    use crate::events::{DriftlockEvent, GENESIS_PREV_HASH};
    use std::collections::BTreeMap;

    fn fixed_event() -> DriftlockEvent {
        let mut metadata = BTreeMap::new();
        metadata.insert("b".to_string(), serde_json::json!("two"));
        metadata.insert("a".to_string(), serde_json::json!(1));
        DriftlockEvent {
            prev_hash: GENESIS_PREV_HASH.to_string(),
            event: "dev.driftlock.task.claimed.v1".to_string(),
            at: "2026-06-08T00:00:00+00:00".to_string(),
            actor: "test".to_string(),
            task: Some("t-1".to_string()),
            metadata,
        }
    }

    #[test]
    fn preimage_serialized_shape_is_pinned() {
        // Pins the exact serde_json serialization the preimage is bound to. If
        // the struct shape / field order / serde behavior changes, this fails
        // loudly rather than silently breaking every previously-signed line.
        // `prev_hash` is first so the chain link is covered by the signature.
        let serialized = serde_json::to_string(&fixed_event()).unwrap();
        assert_eq!(
            serialized,
            r#"{"prev_hash":"0000000000000000000000000000000000000000000000000000000000000000","event":"dev.driftlock.task.claimed.v1","at":"2026-06-08T00:00:00+00:00","actor":"test","task":"t-1","metadata":{"a":1,"b":"two"}}"#,
            "event serialization changed; the signing preimage is bound to this exact shape"
        );
    }

    #[test]
    fn record_hash_changes_when_prev_hash_changes() {
        // The record hash (next row's prev_hash) must depend on this row's own
        // prev_hash, otherwise reordering rows with identical payloads would not
        // break the chain.
        let a = fixed_event();
        let mut b = fixed_event();
        b.prev_hash = "11".repeat(32);
        assert_ne!(record_hash(&a).unwrap(), record_hash(&b).unwrap());
    }

    #[test]
    fn record_hash_is_deterministic() {
        assert_eq!(record_hash(&fixed_event()).unwrap(), record_hash(&fixed_event()).unwrap());
    }

    #[test]
    fn preimage_has_domain_prefix() {
        let preimage = signing_preimage(&fixed_event()).unwrap();
        let text = String::from_utf8(preimage).unwrap();
        assert!(text.starts_with("driftlock:events:sign:v1:"));
    }

    #[test]
    fn preimage_is_deterministic() {
        // Same logical event (metadata inserted in different order) must yield
        // the same preimage thanks to BTreeMap key ordering.
        let mut a = fixed_event();
        let mut b_meta = BTreeMap::new();
        b_meta.insert("a".to_string(), serde_json::json!(1));
        b_meta.insert("b".to_string(), serde_json::json!("two"));
        let mut b = fixed_event();
        b.metadata = b_meta;
        a.metadata = {
            let mut m = BTreeMap::new();
            m.insert("b".to_string(), serde_json::json!("two"));
            m.insert("a".to_string(), serde_json::json!(1));
            m
        };
        assert_eq!(signing_preimage(&a).unwrap(), signing_preimage(&b).unwrap());
    }
}
