//! Timeline command — shows what grew or shrank since last scan.
//! Progressive display: each path shows a spinner while scanning, then result.

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
            print!("\x1b[2J\x1b[H");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            super::ui::print_header("\x1b[1;34m\u{1f4c8} Space Timeline\x1b[0m");
            println!("  No previous scan data found.");
            println!("  Run {} first to establish a baseline.\n", "sweep scan ~".bold());
            wait_for_key();
        }
        return;
    }

    // JSON mode — scan all, output at once
    if output::is_json() {
        let mut changes = Vec::new();
        let mut total_growth: i64 = 0;
        for (path_str, &prev_size) in &cached {
            if prev_size < 100 * 1024 * 1024 { continue; }
            let path = std::path::Path::new(path_str.as_str());
            if !path.exists() { continue; }
            let current_size = scanner::scan_size_native(path);
            let delta = current_size as i64 - prev_size as i64;
            if delta.unsigned_abs() > 50 * 1024 * 1024 {
                let direction = if delta > 0 { "grew" } else { "shrank" };
                changes.push(TimelineEntry {
                    path: path_str.clone(), previous_size: prev_size,
                    current_size, delta, direction: direction.to_string(),
                });
                total_growth += delta;
            }
        }
        changes.sort_by(|a, b| b.delta.unsigned_abs().cmp(&a.delta.unsigned_abs()));
        output::print_json(&TimelineOutput { changes, total_growth });
        return;
    }

    // Human-readable: progressive scan
    print!("\x1b[2J\x1b[H");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    super::ui::print_header("\x1b[1;34m\u{1f4c8} Space Timeline\x1b[0m");

    // Filter to large dirs only
    let paths_to_scan: Vec<(String, u64)> = cached.iter()
        .filter(|(path_str, &size)| {
            size > 100 * 1024 * 1024
            && std::path::Path::new(path_str.as_str()).exists()
        })
        .map(|(k, &v)| (k.clone(), v))
        .collect();

    let home_str = crate::error::home_or_exit().display().to_string();
    let mut changes: Vec<TimelineEntry> = Vec::new();
    let mut total_growth: i64 = 0;

    // Progressive: scan each, show result immediately
    for (i, (path_str, prev_size)) in paths_to_scan.iter().enumerate() {
        let short = path_str.replace(&home_str, "~");

        print!("  \x1b[33m{}\x1b[0m ({}/{}) {}...\r",
            super::ui::spinner(i), i + 1, paths_to_scan.len(), short);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let current_size = scanner::scan_size_native(std::path::Path::new(path_str.as_str()));
        let delta = current_size as i64 - *prev_size as i64;
        let abs_delta = delta.unsigned_abs();

        if abs_delta > 50 * 1024 * 1024 {
            print!("\r\x1b[K");
            let size_str = bytesize::ByteSize::b(abs_delta).to_string();
            if delta > 0 {
                println!("  \x1b[32m\u{2713}\x1b[0m \x1b[31m\u{25b2}\x1b[0m {:>10}  {}",
                    format!("+{}", size_str).red(), short);
            } else {
                println!("  \x1b[32m\u{2713}\x1b[0m \x1b[32m\u{25bc}\x1b[0m {:>10}  {}",
                    format!("-{}", size_str).green(), short);
            }

            changes.push(TimelineEntry {
                path: path_str.clone(),
                previous_size: *prev_size,
                current_size,
                delta,
                direction: if delta > 0 { "grew" } else { "shrank" }.to_string(),
            });
            total_growth += delta;
        } else {
            print!("\r\x1b[K");
        }
    }

    // Summary
    println!();
    println!("  \x1b[32m\u{2713}\x1b[0m Scanned {} directories", paths_to_scan.len());

    if changes.is_empty() {
        println!("  No significant changes (threshold: 50 MB).\n");
    } else {
        let total_str = bytesize::ByteSize::b(total_growth.unsigned_abs()).to_string();
        if total_growth > 0 {
            println!("  Net growth: {}", format!("+{}", total_str).red().bold());
        } else {
            println!("  Net freed: {}", format!("-{}", total_str).green().bold());
        }
        println!();
    }

    wait_for_key();
}

fn wait_for_key() {
    println!("  \x1b[90mPress any key to continue...\x1b[0m");
    let _ = crossterm::terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(400));
    while crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let _ = crossterm::event::read();
    let _ = crossterm::terminal::disable_raw_mode();
}
