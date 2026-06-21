//! Timeline command — shows what grew or shrank since last scan.
//! Compares current directory sizes against the cached sizes.json.

use crate::cache;
use crate::output::{self, TimelineOutput, TimelineEntry};
use crate::scanner;
use colored::Colorize;

pub fn run() {
    let cached = cache::load_cached_sizes();

    if cached.is_empty() {
        if output::is_json() {
            output::print_json(&TimelineOutput {
                changes: vec![],
                total_growth: 0,
            });
        } else {
            super::ui::print_header("\x1b[1;34m\u{1f4c8} Space Timeline\x1b[0m");
            println!("\n  No previous scan data found.");
            println!("  Run {} first to establish a baseline.\n", "sweep scan ~".bold());
            super::ui::wait_any_key();
        }
        return;
    }

    // Re-scan the paths we have cached
    if !output::is_json() {
        super::ui::print_header("\x1b[1;34m\u{1f4c8} Space Timeline\x1b[0m");
        print!("  \x1b[33m[scanning...]\x1b[0m");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }

    let home = dirs::home_dir().unwrap_or_default();
    let current_children = scanner::scan_children(&home);

    if !output::is_json() {
        print!("\r\x1b[K"); // Clear scanning line
        println!();
    }

    let mut changes: Vec<TimelineEntry> = Vec::new();
    let mut total_growth: i64 = 0;

    for child in &current_children {
        if let Some(&prev_size) = cached.get(&child.path) {
            let delta = child.size as i64 - prev_size as i64;
            let abs_delta = delta.unsigned_abs();

            // Only report changes > 50 MB
            if abs_delta > 50 * 1024 * 1024 {
                let direction = if delta > 0 { "grew" } else { "shrank" };
                changes.push(TimelineEntry {
                    path: child.path.clone(),
                    previous_size: prev_size,
                    current_size: child.size,
                    delta,
                    direction: direction.to_string(),
                });
                total_growth += delta;
            }
        } else {
            // New directory (not in cache) — treat as growth if large
            if child.size > 100 * 1024 * 1024 {
                changes.push(TimelineEntry {
                    path: child.path.clone(),
                    previous_size: 0,
                    current_size: child.size,
                    delta: child.size as i64,
                    direction: "new".to_string(),
                });
                total_growth += child.size as i64;
            }
        }
    }

    // Sort by absolute delta descending
    changes.sort_by(|a, b| b.delta.unsigned_abs().cmp(&a.delta.unsigned_abs()));

    if output::is_json() {
        output::print_json(&TimelineOutput {
            changes,
            total_growth,
        });
        return;
    }

    // Human-readable output
    if changes.is_empty() {
        println!("  No significant changes detected (threshold: 50 MB).\n");
        super::ui::wait_any_key();
        return;
    }

    for entry in &changes {
        let size_str = bytesize::ByteSize::b(entry.delta.unsigned_abs()).to_string();
        let arrow = match entry.direction.as_str() {
            "grew" => format!("{}", format!("+{}", size_str).red()),
            "shrank" => format!("{}", format!("-{}", size_str).green()),
            "new" => format!("{}", format!("+{} (new)", size_str).yellow()),
            _ => size_str,
        };

        // Shorten path for display
        let display_path = entry.path.replace(
            &dirs::home_dir().unwrap_or_default().display().to_string(),
            "~",
        );

        println!("  {:>12}  {}", arrow, display_path);
    }

    println!();
    let total_str = bytesize::ByteSize::b(total_growth.unsigned_abs()).to_string();
    if total_growth > 0 {
        println!("  Net growth: {}", format!("+{}", total_str).red().bold());
    } else {
        println!("  Net freed: {}", format!("-{}", total_str).green().bold());
    }
    println!();

    // Update cache with current values
    let new_cache: std::collections::HashMap<String, u64> = current_children
        .iter()
        .map(|c| (c.path.clone(), c.size))
        .collect();
    cache::save_all(&new_cache);

    super::ui::wait_any_key();
}
