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
    match dirs::home_dir() {
        Some(h) if !h.as_os_str().is_empty() => Ok(h),
        _ => Err(SweepError::NoHomeDir),
    }
}

/// Get the home directory, or print an error and exit.
///
/// This replaces the `dirs::home_dir().unwrap_or_default()` footgun: an empty
/// `PathBuf` turns absolute-looking joins like `home.join("Library/Caches")`
/// into *current-directory-relative* paths, which could cause scans/deletes to
/// hit unintended locations. Aborting is always safer than guessing.
pub fn home_or_exit() -> PathBuf {
    match home_dir() {
        Ok(h) => h,
        Err(_) => {
            eprintln!("sweep: could not determine your home directory; aborting for safety.");
            std::process::exit(1);
        }
    }
}
