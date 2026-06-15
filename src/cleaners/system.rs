//! System cache cleaner — user caches, logs, tmp files.

use std::path::{Path, PathBuf};
use dirs;

/// Known system cache locations (macOS + Linux).
pub fn cache_paths() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut paths = vec![];

    // macOS caches
    paths.push(home.join("Library/Caches"));
    paths.push(home.join("Library/Logs"));
    paths.push(home.join("Library/Application Support/CrashReporter"));

    // Linux caches
    paths.push(home.join(".cache"));
    paths.push(home.join(".local/share/Trash"));

    // Common
    paths.push(home.join(".npm/_cacache"));
    paths.push(home.join(".cargo/registry/cache"));
    paths.push(home.join(".gradle/caches"));

    paths.into_iter().filter(|p| p.exists()).collect()
}

/// Known log locations.
pub fn log_paths() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut paths = vec![];

    paths.push(home.join("Library/Logs"));
    paths.push(home.join(".local/share/logs"));
    paths.push(PathBuf::from("/var/log"));

    paths.into_iter().filter(|p| p.exists()).collect()
}
