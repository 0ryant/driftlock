//! Deployment-key signing for `axiom.receipt.v1` receipts (`key_class` self-label).
//!
//! driftlock's *baseline* signing key is the operator-generated key under
//! `.driftlock/keys/active.ed25519` ([`crate::signing`]). That key proves a
//! receipt was "bound under THIS key" — a mechanism, not a held-secret origin
//! proof — so it is [`KeyClass::Dev`]. To cross the origin gate a deployment
//! supplies its own private key out-of-band via `DRIFTLOCK_SIGNING_SEED_HEX`;
//! that key is [`KeyClass::Deployment`] (origin-grade once its public half is
//! pinned in the trust store / published as a trust root).
//!
//! Resolution lives once in [`axiom_receipt::Keyring`]. This module only
//! constructs driftlock's keyring (its baseline position is the operator key,
//! not a compiled-in pinned seed) and surfaces the active signer + [`KeyClass`]
//! so the receipt body can stamp which kind of key it carries.
//!
//! Verification is unchanged: the existing trust-store path ([`crate::receipt`])
//! verifies a deployment receipt exactly as it verifies an operator-key one —
//! its `key_id`'s public half must be pinned via `driftlock key trust`. No
//! `active_verifier` is threaded here.

use axiom_receipt::{DeploymentKeyEnv, Ed25519Signer, KeyClass, Keyring};

/// Env-var prefix for driftlock's deployment key:
/// `DRIFTLOCK_SIGNING_SEED_HEX` / `DRIFTLOCK_SIGNING_KEY_ID`.
pub const ENV_PREFIX: &str = "DRIFTLOCK";

/// Default `key_id` for a deployment key when `DRIFTLOCK_SIGNING_KEY_ID` is unset.
pub const DEPLOYMENT_KEY_ID_DEFAULT: &str = "driftlock-deployment-ed25519-v1";

/// Build a keyring whose *baseline* (non-deployment) position is the supplied
/// operator key. When `DRIFTLOCK_SIGNING_SEED_HEX` is set and valid the keyring's
/// active signer is the deployment key instead; otherwise it is the operator key.
///
/// driftlock has no compiled-in pinned dev seed (its baseline key is generated at
/// runtime), so the operator key's seed/fingerprint are passed in as the keyring's
/// "pinned" position.
#[must_use]
pub fn keyring(operator_seed: [u8; 32], operator_key_id: impl Into<String>) -> Keyring {
    Keyring::new(
        DeploymentKeyEnv::from_prefix(ENV_PREFIX),
        operator_seed,
        operator_key_id,
        DEPLOYMENT_KEY_ID_DEFAULT,
    )
}

/// The active signer over an operator-key baseline: the `DRIFTLOCK_SIGNING_SEED_HEX`
/// deployment key if configured and valid, otherwise the operator key.
#[must_use]
pub fn active_signer(operator_seed: [u8; 32], operator_key_id: impl Into<String>) -> Ed25519Signer {
    keyring(operator_seed, operator_key_id).active_signer().0
}

/// The `key_id` the active signer stamps on receipt bodies (deployment key-id when
/// `DRIFTLOCK_SIGNING_SEED_HEX` is configured, else the operator fingerprint).
#[must_use]
pub fn active_key_id(operator_seed: [u8; 32], operator_key_id: impl Into<String>) -> String {
    keyring(operator_seed, operator_key_id).active_key_id().0
}

/// The [`KeyClass`] of the active signer — [`KeyClass::Dev`] for the operator
/// baseline key, [`KeyClass::Deployment`] when a deployment seed is configured.
/// Stamped into the receipt body so a receipt declares whether it is origin-grade.
#[must_use]
pub fn active_key_class(operator_seed: [u8; 32], operator_key_id: impl Into<String>) -> KeyClass {
    keyring(operator_seed, operator_key_id).active_key_id().1
}
