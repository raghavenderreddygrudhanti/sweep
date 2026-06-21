//! JetBrains IDE cleaner — IntelliJ, WebStorm, PyCharm, GoLand, etc.

use std::path::PathBuf;
use std::fs;

pub fn jetbrains_paths() -> Vec<(PathBuf, &'static str)> {
    let home = crate::error::home_or_exit();
    let mut paths = vec![];

    // Caches
    let cache_dir = home.join("Library/Caches/JetBrains");
    if cache_dir.exists() {
        if let Ok(entries) = fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    paths.push((p, leak_string(format!("JetBrains {} cache", name))));
                }
            }
        }
    }

    // Logs
    let log_dir = home.join("Library/Logs/JetBrains");
    if log_dir.exists() {
        paths.push((log_dir, "JetBrains logs"));
    }

    // Old Toolbox versions
    let toolbox = home.join("Library/Application Support/JetBrains/Toolbox");
    if toolbox.exists() {
        paths.push((toolbox, "JetBrains Toolbox data"));
    }

    paths
}

// Helper to leak a String into a &'static str for the tuple
fn leak_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
