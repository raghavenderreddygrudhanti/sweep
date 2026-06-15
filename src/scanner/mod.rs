//! Parallel filesystem scanner.
//! Uses a single `du` call for batch sizing + Rust walkdir for drill-down.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use rayon::prelude::*;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub path: String,
    pub size: u64,
    pub file_count: u64,
    pub is_dir: bool,
}

/// Get size of a single path using `du -sk`.
pub fn scan_size(path: &Path) -> (u64, u64) {
    (du_single(path), 0)
}

/// Scan all top-level children using ONE `du` call (batch — fast).
/// This is the key optimization: one subprocess instead of N.
pub fn scan_children(path: &Path) -> Vec<ScanResult> {
    // Single `du -skc` on the parent gets all children in one call
    let sizes = du_batch(path);

    let mut results: Vec<ScanResult> = std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let p = entry.path();
            let name = p.file_name()?.to_string_lossy().to_string();

            // Skip system junk
            if name == ".Spotlight-V100" || name == ".fseventsd"
                || name == ".DocumentRevisions-V100" || name == ".Trashes"
                || name == ".vol" || name == "cores" {
                return None;
            }

            let is_dir = p.is_dir();
            let size = sizes.get(&name).copied().unwrap_or(0);

            if size == 0 && is_dir {
                return None;
            }

            Some(ScanResult {
                path: p.display().to_string(),
                size,
                file_count: 0,
                is_dir,
            })
        })
        .collect();

    results.sort_by(|a, b| b.size.cmp(&a.size));
    results
}

/// Batch `du` — runs `du -sk *` in the directory to get all child sizes
/// in a single subprocess call. Returns HashMap<filename, bytes>.
fn du_batch(dir: &Path) -> HashMap<String, u64> {
    let mut map = HashMap::new();

    // du -sk on each entry inside dir, with depth 0
    // Using `du -skc dir/*` would hit arg limit, so use du -sk with maxdepth
    let output = std::process::Command::new("du")
        .args(["-sk", "-d", "1"])
        .arg(dir)
        .stderr(std::process::Stdio::null())
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            let stdout = String::from_utf8_lossy(&o.stdout);
            for line in stdout.lines() {
                let mut parts = line.split_whitespace();
                if let (Some(size_str), Some(path_str)) = (parts.next(), parts.next()) {
                    if let Ok(kb) = size_str.parse::<u64>() {
                        // Extract just the filename from the full path
                        let name = Path::new(path_str)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        if !name.is_empty() && path_str != dir.to_string_lossy().as_ref() {
                            map.insert(name, kb * 1024);
                        }
                    }
                }
            }
        }
    }

    map
}

/// Single path du (for individual items in overview).
fn du_single(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }

    let output = std::process::Command::new("du")
        .args(["-sk"])
        .arg(path)
        .stderr(std::process::Stdio::null())
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .map(|kb| kb * 1024)
                .unwrap_or(0)
        }
        _ => 0,
    }
}

/// Find directories matching a name pattern.
pub fn find_dirs_by_name(root: &Path, name: &str, max_depth: usize) -> Vec<ScanResult> {
    WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .par_bridge()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy() == name)
        .map(|entry| {
            let path = entry.path().to_path_buf();
            let size = du_single(&path);
            ScanResult {
                path: path.display().to_string(),
                size,
                file_count: 0,
                is_dir: true,
            }
        })
        .collect()
}
