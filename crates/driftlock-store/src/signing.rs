//! Ed25519 signing for audit event lines.

use crate::events::DriftlockEvent;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SIGN_DOMAIN: &str = "driftlock:events:sign:v1:";

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
pub fn generate_operator_key(repo_root: &Path, force: bool) -> Result<KeyInfo> {
    let key_path = active_signing_key_path(repo_root);
    if key_path.exists() && !force {
        anyhow::bail!("signing key already exists at {}; use force to rotate", key_path.display());
    }
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let signing_key = SigningKey::generate(&mut OsRng);
    fs::write(&key_path, signing_key.to_bytes())?;
    publish_trust_pubkey(repo_root, &signing_key.verifying_key())?;
    Ok(key_info(&key_path, &signing_key))
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
    for (idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        report.rows_scanned += 1;
        let line_no = idx + 1;
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
            continue;
        }
        if serde_json::from_str::<DriftlockEvent>(line).is_err() {
            report.status = "fail".into();
            report.failures.push(format!("line {line_no}: invalid event json"));
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

fn signing_preimage(event: &DriftlockEvent) -> Result<Vec<u8>> {
    let canonical = serde_json::to_string(event)?;
    let hash = Sha256::digest(canonical.as_bytes());
    Ok(format!("{SIGN_DOMAIN}{}", hex::encode(hash)).into_bytes())
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

fn key_info(path: &Path, signing_key: &SigningKey) -> KeyInfo {
    let verifying_key = signing_key.verifying_key();
    KeyInfo {
        path: path.to_path_buf(),
        key_id: key_fingerprint(&verifying_key),
        public_key_hex: hex::encode(verifying_key.as_bytes()),
    }
}
