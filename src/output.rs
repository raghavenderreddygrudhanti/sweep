//! Output formatting — supports both human-readable TUI and JSON output.

use std::sync::atomic::{AtomicBool, Ordering};

static JSON_MODE: AtomicBool = AtomicBool::new(false);

/// Set global JSON output mode.
pub fn set_json_mode(enabled: bool) {
    JSON_MODE.store(enabled, Ordering::Relaxed);
}

/// Check if JSON output mode is enabled.
pub fn is_json() -> bool {
    JSON_MODE.load(Ordering::Relaxed)
}

/// Print a JSON value if in JSON mode. Returns true if JSON was printed.
pub fn print_json<T: serde::Serialize>(value: &T) -> bool {
    if is_json() {
        match serde_json::to_string_pretty(value) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("JSON serialization error: {}", e),
        }
        true
    } else {
        false
    }
}

/// Structured output for clean operations.
#[derive(serde::Serialize)]
pub struct CleanOutput {
    pub operation: String,
    pub items: Vec<CleanItem>,
    pub total_freed: u64,
    pub dry_run: bool,
    pub mode: String,
}

#[derive(serde::Serialize)]
pub struct CleanItem {
    pub path: String,
    pub size: u64,
    pub category: String,
}

/// Structured output for scan operations.
#[derive(serde::Serialize)]
pub struct ScanOutput {
    pub path: String,
    pub total_size: u64,
    pub children: Vec<crate::scanner::ScanResult>,
}

/// Structured output for timeline.
#[derive(serde::Serialize)]
pub struct TimelineOutput {
    pub changes: Vec<TimelineEntry>,
    pub total_growth: i64,
}

#[derive(serde::Serialize)]
pub struct TimelineEntry {
    pub path: String,
    pub previous_size: u64,
    pub current_size: u64,
    pub delta: i64,
    pub direction: String, // "grew" or "shrank"
}

/// Structured output for recommendations.
#[derive(serde::Serialize)]
pub struct RecommendOutput {
    pub recommendations: Vec<Recommendation>,
    pub total_reclaimable: u64,
}

#[derive(serde::Serialize)]
pub struct Recommendation {
    pub priority: u8,
    pub category: String,
    pub description: String,
    pub size: u64,
    pub command: String,
}
