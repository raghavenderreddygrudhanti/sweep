//! Homebrew cleaner — old versions, download cache.

use std::process::Command;
use std::path::PathBuf;

/// Get Homebrew cache size.
pub fn brew_cache_path() -> Option<PathBuf> {
    let output = Command::new("brew").args(["--cache"]).output().ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let p = PathBuf::from(&path);
        if p.exists() { Some(p) } else { None }
    } else {
        None
    }
}

/// Run brew cleanup (remove old versions + downloads).
pub fn brew_cleanup(dry_run: bool) -> u64 {
    if dry_run {
        // Get size of cache
        if let Some(cache) = brew_cache_path() {
            return crate::scanner::scan_size(&cache).0;
        }
        return 0;
    }

    let output = Command::new("brew")
        .args(["cleanup", "--prune=all", "-s"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            // Return freed space (approximate from cache)
            if let Some(cache) = brew_cache_path() {
                return crate::scanner::scan_size(&cache).0;
            }
            0
        }
        _ => 0,
    }
}
