//! Operation log — detailed audit trail of all sweep operations.
//! Logged to ~/.sweep/operations.log with timestamps.
//! Separate from history.rs (which is a simple human-readable log).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;

fn log_path() -> PathBuf {
    let dir = dirs::home_dir().unwrap_or_default().join(".sweep");
    let _ = fs::create_dir_all(&dir);
    dir.join("operations.log")
}

/// Log an operation with full detail.
pub fn log_operation(operation: &str, path: &str, size: u64, success: bool, detail: &str) {
    let log = log_path();
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log) {
        let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
        let status = if success { "OK" } else { "FAIL" };
        let _ = writeln!(file, "[{}] {} | {} | {} | {} bytes | {}",
            ts, status, operation, path, size, detail);
    }
}

/// Log the start of a sweep session.
pub fn log_session_start(command: &str) {
    let log = log_path();
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log) {
        let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "\n[{}] === SESSION: sweep {} ===", ts, command);
    }
}

/// Log session summary.
pub fn log_session_end(freed: u64, items: u32, categories: u32) {
    let log = log_path();
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log) {
        let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] === DONE: freed {} bytes, {} items, {} categories ===\n",
            ts, freed, items, categories);
    }
}

/// Show recent operations log.
pub fn show_log(lines: usize) {
    let log = log_path();
    if !log.exists() {
        println!("  No operations logged yet.\n");
        return;
    }

    if let Ok(content) = fs::read_to_string(&log) {
        let all_lines: Vec<&str> = content.lines().collect();
        let recent = &all_lines[all_lines.len().saturating_sub(lines)..];
        println!("\n  \x1b[1mRecent operations:\x1b[0m ({})\n", log.display());
        for line in recent {
            if line.contains("=== SESSION") {
                println!("  \x1b[36m{}\x1b[0m", line);
            } else if line.contains("OK") {
                println!("  \x1b[32m{}\x1b[0m", line);
            } else if line.contains("FAIL") {
                println!("  \x1b[33m{}\x1b[0m", line);
            } else {
                println!("  \x1b[90m{}\x1b[0m", line);
            }
        }
        println!();
    }
}
