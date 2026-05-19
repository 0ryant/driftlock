//! Error type for Driftlock core.

use thiserror::Error;

/// Driftlock core result.
pub type Result<T> = std::result::Result<T, DriftlockError>;

/// Core error variants.
#[derive(Debug, Error)]
pub enum DriftlockError {
    /// A requested task was not found.
    #[error("task not found: {0}")]
    TaskNotFound(String),

    /// A lane could not be found.
    #[error("lane not found: {0}")]
    LaneNotFound(String),

    /// Input failed validation.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Serialization failed.
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    /// TOML parsing failed.
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),

    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
