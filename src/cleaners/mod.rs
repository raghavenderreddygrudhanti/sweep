//! Cleaner modules — each knows about specific types of junk.

pub mod system;
pub mod dev;
pub mod ai;
pub mod docker;
pub mod browser;
pub mod apps;
pub mod trash;
pub mod optimize;
pub mod xcode;
pub mod jetbrains;
pub mod apps_cache;
pub mod homebrew;

use std::path::Path;
use std::fs;

use crate::error::{SweepError, SweepResult};
use crate::history;

/// Deletion mode: Trash (recoverable) or Force (permanent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeleteMode {
    /// Move to system Trash (default, recoverable via Finder/file manager)
    Trash,
    /// Permanently delete (irreversible, use with --force)
    Force,
}

/// Remove a directory. Returns bytes freed.
/// Default behavior: moves to Trash. With Force mode: permanent deletion.
pub fn remove_dir(path: &Path, dry_run: bool, mode: DeleteMode) -> u64 {
    let size = crate::scanner::scan_size_native(path);

    if dry_run || size == 0 {
        return size;
    }

    let result = match mode {
        DeleteMode::Trash => trash_delete(path),
        DeleteMode::Force => force_remove_dir(path),
    };

    match result {
        Ok(()) => {
            history::log_delete(
                &path.display().to_string(),
                size,
                if mode == DeleteMode::Trash { "trash" } else { "delete" },
            );
            size
        }
        Err(e) => {
            eprintln!("  \u{26a0} {}", e);
            0
        }
    }
}

/// Remove a file. Returns bytes freed.
/// Default behavior: moves to Trash. With Force mode: permanent deletion.
pub fn remove_file(path: &Path, dry_run: bool, mode: DeleteMode) -> u64 {
    let size = path.metadata().map(|m| m.len()).unwrap_or(0);

    if dry_run || size == 0 {
        return size;
    }

    let result = match mode {
        DeleteMode::Trash => trash_delete(path),
        DeleteMode::Force => force_remove_file(path),
    };

    match result {
        Ok(()) => {
            history::log_delete(
                &path.display().to_string(),
                size,
                if mode == DeleteMode::Trash { "trash" } else { "delete" },
            );
            size
        }
        Err(e) => {
            eprintln!("  \u{26a0} {}", e);
            0
        }
    }
}

/// Convenience: remove_dir with default Trash mode (backward-compatible API).
pub fn remove_dir_safe(path: &Path, dry_run: bool) -> u64 {
    remove_dir(path, dry_run, DeleteMode::Trash)
}

/// Convenience: remove_file with default Trash mode (backward-compatible API).
pub fn remove_file_safe(path: &Path, dry_run: bool) -> u64 {
    remove_file(path, dry_run, DeleteMode::Trash)
}

/// Move a path to the system Trash (recoverable).
/// Falls back to Finder AppleScript on macOS if the trash crate fails (permission issues).
pub(crate) fn trash_delete(path: &Path) -> SweepResult<()> {
    // First try the trash crate (fast, no UI)
    match ::trash::delete(path) {
        Ok(()) => Ok(()),
        Err(_e) => {
            // Fallback: use Finder via osascript (always has permission on macOS)
            #[cfg(target_os = "macos")]
            {
                let abs_path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    std::env::current_dir().unwrap_or_default().join(path)
                };

                let script = format!(
                    "tell application \"Finder\" to delete POSIX file \"{}\"",
                    abs_path.display()
                );

                let output = std::process::Command::new("osascript")
                    .args(["-e", &script])
                    .stderr(std::process::Stdio::piped())
                    .output();

                match output {
                    Ok(o) if o.status.success() => Ok(()),
                    Ok(o) => Err(SweepError::Trash {
                        path: path.to_path_buf(),
                        reason: String::from_utf8_lossy(&o.stderr).to_string(),
                    }),
                    Err(e) => Err(SweepError::Trash {
                        path: path.to_path_buf(),
                        reason: e.to_string(),
                    }),
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                Err(SweepError::Trash {
                    path: path.to_path_buf(),
                    reason: _e.to_string(),
                })
            }
        }
    }
}

/// Permanently remove a directory (irreversible).
fn force_remove_dir(path: &Path) -> SweepResult<()> {
    fs::remove_dir_all(path).map_err(|e| SweepError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Permanently remove a file (irreversible).
fn force_remove_file(path: &Path) -> SweepResult<()> {
    fs::remove_file(path).map_err(|e| SweepError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}
