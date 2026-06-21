//! Parallel filesystem scanner.
//! Pure Rust implementation using walkdir + rayon. No subprocess calls.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use rayon::prelude::*;
use walkdir::WalkDir;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanResult {
    pub path: String,
    pub size: u64,
    pub file_count: u64,
    pub is_dir: bool,
}

// --- System directories to skip during scanning ---
const SKIP_DIRS: &[&str] = &[
    ".Spotlight-V100",
    ".fseventsd",
    ".DocumentRevisions-V100",
    ".Trashes",
    ".vol",
    "cores",
];

/// Pure-Rust directory size calculation using parallel walkdir.
/// Returns total bytes for a path (file or directory).
pub fn scan_size_native(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }

    if !path.exists() {
        return 0;
    }

    let total = AtomicU64::new(0);

    WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .filter_map(|e| e.ok())
        .for_each(|entry| {
            if entry.file_type().is_file() {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                total.fetch_add(size, Ordering::Relaxed);
            }
        });

    total.load(Ordering::Relaxed)
}

/// Legacy API — kept for backward compatibility.
pub fn scan_size(path: &Path) -> (u64, u64) {
    (scan_size_native(path), 0)
}

/// Scan all top-level children of a directory in parallel.
/// Returns sorted results (largest first).
pub fn scan_children(path: &Path) -> Vec<ScanResult> {
    let entries: Vec<_> = match std::fs::read_dir(path) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return Vec::new(),
    };

    let mut results: Vec<ScanResult> = entries
        .par_iter()
        .filter_map(|entry| {
            let p = entry.path();
            let name = p.file_name()?.to_string_lossy().to_string();

            // Skip system/hidden junk directories
            if SKIP_DIRS.contains(&name.as_str()) {
                return None;
            }

            let is_dir = p.is_dir();
            let (size, file_count) = if is_dir {
                scan_dir_stats(&p)
            } else {
                let s = p.metadata().map(|m| m.len()).unwrap_or(0);
                (s, 1)
            };

            if size == 0 && is_dir {
                return None;
            }

            Some(ScanResult {
                path: p.display().to_string(),
                size,
                file_count,
                is_dir,
            })
        })
        .collect();

    results.sort_by(|a, b| b.size.cmp(&a.size));
    results
}

/// Scan a directory and return (total_size, file_count).
fn scan_dir_stats(path: &Path) -> (u64, u64) {
    let size = AtomicU64::new(0);
    let count = AtomicU64::new(0);

    WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .filter_map(|e| e.ok())
        .for_each(|entry| {
            if entry.file_type().is_file() {
                let s = entry.metadata().map(|m| m.len()).unwrap_or(0);
                size.fetch_add(s, Ordering::Relaxed);
                count.fetch_add(1, Ordering::Relaxed);
            }
        });

    (size.load(Ordering::Relaxed), count.load(Ordering::Relaxed))
}

/// Find directories matching a name pattern (parallel).
pub fn find_dirs_by_name(root: &Path, name: &str, max_depth: usize) -> Vec<ScanResult> {
    WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .par_bridge()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy() == name)
        .map(|entry| {
            let path = entry.path().to_path_buf();
            let (size, file_count) = scan_dir_stats(&path);
            ScanResult {
                path: path.display().to_string(),
                size,
                file_count,
                is_dir: true,
            }
        })
        .collect()
}

/// Find files larger than a threshold within a directory.
pub fn find_large_files(root: &Path, min_bytes: u64, max_depth: usize) -> Vec<ScanResult> {
    let mut results: Vec<ScanResult> = WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .par_bridge()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|entry| {
            let size = entry.metadata().ok()?.len();
            if size >= min_bytes {
                Some(ScanResult {
                    path: entry.path().display().to_string(),
                    size,
                    file_count: 1,
                    is_dir: false,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| b.size.cmp(&a.size));
    results.truncate(50); // Top 50 largest files
    results
}
