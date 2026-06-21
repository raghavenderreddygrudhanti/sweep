//! Operation history log — tracks all deletions for undo/audit.

use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;

const MAX_HISTORY_LINES: usize = 1000;

/// Get the log file path.
fn log_path() -> PathBuf {
    let dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
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

    // Rotate if over limit
    rotate_if_needed(&log);
}

/// Show recent history (human-readable).
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
                let size_str = bytesize::ByteSize::b(
                    parts[2].parse().unwrap_or(0)
                ).to_string();
                println!("  {}  {:>10}  {:8}  {}",
                    parts[0], // timestamp
                    size_str,
                    parts[1], // operation
                    parts[3], // path
                );
            }
        }
    }
}

/// Show history as JSON.
pub fn show_history_json() {
    let log = log_path();
    let mut entries: Vec<HistoryEntry> = Vec::new();

    if log.exists() {
        if let Ok(content) = std::fs::read_to_string(&log) {
            for line in content.lines().rev().take(50) {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    entries.push(HistoryEntry {
                        timestamp: parts[0].to_string(),
                        operation: parts[1].to_string(),
                        size: parts[2].parse().unwrap_or(0),
                        path: parts[3].to_string(),
                    });
                }
            }
        }
    }

    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        println!("{}", json);
    }
}

#[derive(serde::Serialize)]
struct HistoryEntry {
    timestamp: String,
    operation: String,
    size: u64,
    path: String,
}

/// Rotate log if it exceeds MAX_HISTORY_LINES.
fn rotate_if_needed(log: &PathBuf) {
    if let Ok(content) = std::fs::read_to_string(log) {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > MAX_HISTORY_LINES {
            // Keep the most recent half
            let keep = &lines[lines.len() - (MAX_HISTORY_LINES / 2)..];
            let trimmed = keep.join("\n") + "\n";
            let _ = std::fs::write(log, trimmed);
        }
    }
}
