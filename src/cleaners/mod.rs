//! Cleaner modules — each knows about specific types of junk.

pub mod system;
pub mod dev;
pub mod ai;
pub mod docker;
pub mod browser;
pub mod apps;
pub mod trash;
pub mod optimize;

use std::path::Path;
use std::fs;

/// Safely remove a directory. Returns bytes freed.
pub fn remove_dir(path: &Path, dry_run: bool) -> u64 {
    use crate::scanner;

    let (size, _) = scanner::scan_size(path);

    if !dry_run {
        if let Err(e) = fs::remove_dir_all(path) {
            eprintln!("  ⚠ Failed to remove {}: {}", path.display(), e);
            return 0;
        }
    }

    size
}

/// Safely remove a file. Returns bytes freed.
pub fn remove_file(path: &Path, dry_run: bool) -> u64 {
    let size = path.metadata().map(|m| m.len()).unwrap_or(0);

    if !dry_run {
        if let Err(e) = fs::remove_file(path) {
            eprintln!("  ⚠ Failed to remove {}: {}", path.display(), e);
            return 0;
        }
    }

    size
}
