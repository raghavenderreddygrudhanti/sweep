//! Timeline command — shows what grew or shrank since last scan.
//! TUI: all items shown at once, scanned in parallel, checkbox-style progress.

use crate::cache;
use crate::output::{self, TimelineEntry, TimelineOutput};
use crate::scanner;
use bytesize::ByteSize;
use colored::Colorize;
use crossterm::event::Event;
use crossterm::{cursor, event, execute, terminal};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct TimelineItem {
    short: String,
    path: String,
    prev_size: u64,
    current_size: u64,
    delta: i64,
    done: bool,
}

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
            let _ = io::stdout().flush();
            super::ui::print_header("\x1b[1;34m\u{1f4c8} Space Timeline\x1b[0m");
            println!("  No previous scan data found.");
            println!(
                "  Run {} first to establish a baseline.\n",
                "sweep scan ~".bold()
            );
            wait_for_key();
        }
        return;
    }

    if output::is_json() {
        run_json(&cached);
        return;
    }

    // Filter to large dirs
    let home_str = crate::error::home_or_exit().display().to_string();
    let paths: Vec<(String, u64)> = cached
        .iter()
        .filter(|(p, &s)| s > 100 * 1024 * 1024 && std::path::Path::new(p.as_str()).exists())
        .map(|(k, &v)| (k.clone(), v))
        .collect();

    if paths.is_empty() {
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
        super::ui::print_header("\x1b[1;34m\u{1f4c8} Space Timeline\x1b[0m");
        println!("  No large directories cached yet.\n");
        wait_for_key();
        return;
    }

    // Build items
    let items: Arc<Mutex<Vec<TimelineItem>>> = Arc::new(Mutex::new(
        paths
            .iter()
            .map(|(p, prev)| {
                let short = p.replace(&home_str, "~");
                // Truncate safely (char boundary, pad with spaces)
                let short = if short.chars().count() > 28 {
                    format!("{:.28}...", short)
                } else {
                    format!("{:<31}", short)
                };
                TimelineItem {
                    short,
                    path: p.clone(),
                    prev_size: *prev,
                    current_size: 0,
                    delta: 0,
                    done: false,
                }
            })
            .collect(),
    ));

    let total_count = paths.len();

    // Parallel scanning in background — each item updates as it completes
    let items_bg = Arc::clone(&items);
    let _worker = std::thread::spawn(move || {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let next_idx = Arc::new(AtomicUsize::new(0));

        // Spawn worker threads (use 4-8 parallel scanners)
        let num_workers = 6.min(total_count);
        let mut handles = Vec::new();

        for _ in 0..num_workers {
            let items_w = Arc::clone(&items_bg);
            let idx = Arc::clone(&next_idx);
            handles.push(std::thread::spawn(move || {
                loop {
                    let i = idx.fetch_add(1, Ordering::SeqCst);
                    if i >= total_count {
                        break;
                    }

                    let path_str = items_w.lock().unwrap()[i].path.clone();
                    let size = scanner::scan_size_native(std::path::Path::new(&path_str));

                    // Update immediately when this one finishes
                    let mut items = items_w.lock().unwrap();
                    items[i].current_size = size;
                    items[i].delta = size as i64 - items[i].prev_size as i64;
                    items[i].done = true;
                }
            }));
        }

        for h in handles {
            let _ = h.join();
        }
    });

    // TUI render loop
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let mut frame: usize = 0;

    loop {
        let _ = execute!(stdout, cursor::MoveTo(0, 0));

        let snapshot: Vec<TimelineItem> = items.lock().unwrap().clone();
        let done_count = snapshot.iter().filter(|it| it.done).count();
        let all_done = done_count == total_count;

        let mut out = String::new();

        if all_done {
            out.push_str(&super::ui::tui_header(
                "\x1b[34m\u{1f4c8} Space Timeline\x1b[0m",
            ));
        } else {
            out.push_str(&super::ui::tui_header_animated(
                "\x1b[34m\u{1f4c8} Space Timeline\x1b[0m",
                frame,
            ));
        }

        // Show all items with checkbox style
        let max_visible: usize = 14;
        let scroll = if total_count > max_visible {
            // Scroll to show incomplete items
            let first_pending = snapshot.iter().position(|it| !it.done).unwrap_or(0);
            if first_pending > max_visible - 3 {
                first_pending.saturating_sub(3)
            } else {
                0
            }
        } else {
            0
        };

        let end = (scroll + max_visible).min(total_count);

        if scroll > 0 {
            out.push_str(&format!(
                "    \x1b[90m\u{2191} {} more above\x1b[0m\r\n",
                scroll
            ));
        }

        for i in scroll..end {
            let item = &snapshot[i];
            if item.done {
                let abs_delta = item.delta.unsigned_abs();
                if abs_delta > 50 * 1024 * 1024 {
                    let size_str = ByteSize::b(abs_delta).to_string();
                    if item.delta > 0 {
                        out.push_str(&format!(
                            "  \x1b[32m\u{2611}\x1b[0m \x1b[31m\u{25b2} +{:<9}\x1b[0m {}\r\n",
                            size_str, item.short
                        ));
                    } else {
                        out.push_str(&format!(
                            "  \x1b[32m\u{2611}\x1b[0m \x1b[32m\u{25bc} -{:<9}\x1b[0m {}\r\n",
                            size_str, item.short
                        ));
                    }
                } else {
                    out.push_str(&format!(
                        "  \x1b[32m\u{2611}\x1b[0m \x1b[90m\u{2500} no change  {}\x1b[0m\r\n",
                        item.short
                    ));
                }
            } else {
                out.push_str(&format!(
                    "  \x1b[33m\u{2610}\x1b[0m {} {}\r\n",
                    super::ui::spinner(frame + i),
                    item.short
                ));
            }
        }

        if end < total_count {
            out.push_str(&format!(
                "    \x1b[90m\u{2193} {} more below\x1b[0m\r\n",
                total_count - end
            ));
        }

        // Footer
        out.push_str("\r\n");
        out.push_str(super::ui::footer_sep());
        if all_done {
            let total_growth: i64 = snapshot.iter().map(|it| it.delta).sum();
            let changed = snapshot
                .iter()
                .filter(|it| it.delta.unsigned_abs() > 50 * 1024 * 1024)
                .count();
            let total_str = ByteSize::b(total_growth.unsigned_abs()).to_string();
            if changed == 0 {
                out.push_str("  \x1b[32m\u{2713}\x1b[0m No significant changes\r\n");
            } else if total_growth > 0 {
                out.push_str(&format!(
                    "  {} changed \u{2014} Net: \x1b[1;31m+{}\x1b[0m\r\n",
                    changed, total_str
                ));
            } else {
                out.push_str(&format!(
                    "  {} changed \u{2014} Net: \x1b[1;32m-{}\x1b[0m\r\n",
                    changed, total_str
                ));
            }
            out.push_str("  \x1b[90mPress any key to exit\x1b[0m\r\n");
        } else {
            out.push_str(&format!(
                "  \x1b[33m\u{2022}\x1b[0m Scanning... ({}/{}) \x1b[90m[parallel]\x1b[0m\r\n",
                done_count, total_count
            ));
        }
        out.push_str("\x1b[J");

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        // Input
        if event::poll(std::time::Duration::from_millis(80)).unwrap_or(false) {
            if let Ok(Event::Key(_)) = event::read() {
                if all_done {
                    break;
                }
            }
        }

        frame += 1;
    }

    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();
}

fn run_json(cached: &std::collections::HashMap<String, u64>) {
    let mut changes = Vec::new();
    let mut total_growth: i64 = 0;
    for (path_str, &prev_size) in cached {
        if prev_size < 100 * 1024 * 1024 {
            continue;
        }
        let path = std::path::Path::new(path_str.as_str());
        if !path.exists() {
            continue;
        }
        let current_size = scanner::scan_size_native(path);
        let delta = current_size as i64 - prev_size as i64;
        if delta.unsigned_abs() > 50 * 1024 * 1024 {
            changes.push(TimelineEntry {
                path: path_str.clone(),
                previous_size: prev_size,
                current_size,
                delta,
                direction: if delta > 0 { "grew" } else { "shrank" }.to_string(),
            });
            total_growth += delta;
        }
    }
    changes.sort_by(|a, b| b.delta.unsigned_abs().cmp(&a.delta.unsigned_abs()));
    output::print_json(&TimelineOutput {
        changes,
        total_growth,
    });
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
