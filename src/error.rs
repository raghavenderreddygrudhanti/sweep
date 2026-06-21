//! Centralized error types for Sweep.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SweepError {
    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to move to trash: {path} — {reason}")]
    Trash { path: PathBuf, reason: String },

    #[error("Path not found: {0}")]
    NotFound(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Home directory could not be determined")]
    NoHomeDir,

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}

pub type SweepResult<T> = Result<T, SweepError>;

/// Get home directory or return error.
pub fn home_dir() -> SweepResult<PathBuf> {
    dirs::home_dir().ok_or(SweepError::NoHomeDir)
}
