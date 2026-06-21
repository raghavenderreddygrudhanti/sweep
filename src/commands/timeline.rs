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

        }
        return;
    }

    // Only rescan paths we previously cached (much faster than full home scan)
    if !output::is_json() {
        super::ui::print_header("\x1b[1;34m\u{1f4c8} Space Timeline\x1b[0m");
        print!("  \x1b[33m\u{2022}\x1b[0m Comparing {} cached paths...\r", cached.len());
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }

    let mut changes: Vec<TimelineEntry> = Vec::new();
    let mut total_growth: i64 = 0;

    for (path_str, &prev_size) in &cached {
        let path = std::path::Path::new(path_str);
        if !path.exists() { continue; }

        let current_size = scanner::scan_size_native(path);
        let delta = current_size as i64 - prev_size as i64;
        let abs_delta = delta.unsigned_abs();

        if abs_delta > 50 * 1024 * 1024 {
            let direction = if delta > 0 { "grew" } else { "shrank" };
            changes.push(TimelineEntry {
                path: path_str.clone(),
                previous_size: prev_size,
                current_size,
                delta,
                direction: direction.to_string(),
            });
            total_growth += delta;
        }
    }

    if !output::is_json() {
        print!("\r\x1b[K");
        println!("  \x1b[32m\u{2713}\x1b[0m Compared {} paths\n", cached.len());
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
        println!("  \x1b[90mPress q to exit\x1b[0m");
        let _ = crossterm::terminal::enable_raw_mode();
        std::thread::sleep(std::time::Duration::from_millis(200));
        while crossterm::event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
            let _ = crossterm::event::read();
        }
        let _ = crossterm::event::read();
        let _ = crossterm::terminal::disable_raw_mode();
        return;
    }

    for entry in &changes {
        let size_str = bytesize::ByteSize::b(entry.delta.unsigned_abs()).to_string();
        let (icon, arrow) = match entry.direction.as_str() {
            "grew" => ("\x1b[31m\u{25b2}\x1b[0m", format!("{}", format!("+{}", size_str).red())),
            "shrank" => ("\x1b[32m\u{25bc}\x1b[0m", format!("{}", format!("-{}", size_str).green())),
            "new" => ("\x1b[33m\u{2605}\x1b[0m", format!("{}", format!("+{}", size_str).yellow())),
            _ => (" ", size_str),
        };

        let display_path = entry.path.replace(
            &crate::error::home_or_exit().display().to_string(),
            "~",
        );

        println!("  {} {:>12}  {}", icon, arrow, display_path);
    }

    println!();
    let total_str = bytesize::ByteSize::b(total_growth.unsigned_abs()).to_string();
    if total_growth > 0 {
        println!("  Net growth: {}", format!("+{}", total_str).red().bold());
    } else {
        println!("  Net freed: {}", format!("-{}", total_str).green().bold());
    }
    println!();

    // Wait for user
    println!("  \x1b[90mPress any key to exit\x1b[0m");
    let _ = crossterm::terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(300));
    while crossterm::event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let _ = crossterm::event::read();
    let _ = crossterm::terminal::disable_raw_mode();
}
