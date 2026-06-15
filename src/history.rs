//! Operation history log — tracks all deletions for undo/audit.

use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;
use dirs;

/// Get the log file path.
fn log_path() -> PathBuf {
    let dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".sweep");
    let _ = create_dir_all(&dir);
    dir.join("history.log")
}

/// Log a deletion operation.
pub fn log_delete(path: &str, size: u64, operation: &str) {
    let log = log_path();
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "{}|{}|{}|{}", timestamp, operation, size, path);
    }
}

/// Show recent history.
pub fn show_history() {
    let log = log_path();
    if !log.exists() {
        println!("  No history yet. Run a clean operation first.");
        return;
    }

    if let Ok(content) = std::fs::read_to_string(&log) {
        let lines: Vec<&str> = content.lines().collect();
        let recent = &lines[lines.len().saturating_sub(20)..];

        println!("  Recent operations:\n");
        for line in recent {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                println!("  {}  {:>10}  {}  {}",
                    parts[0], // timestamp
                    bytesize::ByteSize::b(parts[2].parse().unwrap_or(0)).to_string(),
                    parts[1], // operation
                    parts[3], // path
                );
            }
        }
    }
}
