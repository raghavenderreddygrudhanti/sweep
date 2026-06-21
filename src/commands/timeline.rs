//! Timeline command — shows what grew or shrank since last scan.
//! TUI: all items shown at once, each with individual progress spinner.

use crate::cache;
use crate::output::{self, TimelineOutput, TimelineEntry};
use crate::scanner;
use colored::Colorize;
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{self, Write};
use crossterm::{terminal, cursor, execute, event};
use crossterm::event::Event;
use bytesize::ByteSize;

struct TimelineItem {
    path: String,
    short: String,
    prev_size: u64,
    current_size: u64,
    delta: i64,
    status: u8, // 0=pending, 1=scanning, 2=done
}

pub fn run() {
    let cached = cache::load_cached_sizes();

    if cached.is_empty() {
        if output::is_json() {
            output::print_json(&TimelineOutput { changes: vec![], total_growth: 0 });
        } else {
            print!("\x1b[2J\x1b[H");
            let _ = io::stdout().flush();
            super::ui::print_header("\x1b[1;34m\u{1f4c8} Space Timeline\x1b[0m");
            println!("  No previous scan data found.");
            println!("  Run {} first to establish a baseline.\n", "sweep scan ~".bold());
            wait_for_key();
        }
        return;
    }

    // JSON mode
    if output::is_json() {
        run_json(&cached);
        return;
    }

    // Filter to large dirs
    let home_str = crate::error::home_or_exit().display().to_string();
    let paths: Vec<(String, u64)> = cached.iter()
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

    // Build items list
    let items: Arc<Mutex<Vec<TimelineItem>>> = Arc::new(Mutex::new(
        paths.iter().map(|(p, prev)| {
            let short = p.replace(&home_str, "~");
            TimelineItem {
                path: p.clone(),
                short,
                prev_size: *prev,
                current_size: 0,
                delta: 0,
                status: 0,
            }
        }).collect()
    ));

    let done_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let total = paths.len();

    // Background worker: scan each path
    let items_bg = Arc::clone(&items);
    let done_bg = Arc::clone(&done_count);
    let _worker = thread::spawn(move || {
        for i in 0..total {
            {
                items_bg.lock().unwrap()[i].status = 1; // scanning
            }
            let path_str = items_bg.lock().unwrap()[i].path.clone();
            let prev = items_bg.lock().unwrap()[i].prev_size;
            let current = scanner::scan_size_native(std::path::Path::new(&path_str));
            {
                let mut items = items_bg.lock().unwrap();
                items[i].current_size = current;
                items[i].delta = current as i64 - prev as i64;
                items[i].status = 2; // done
            }
            *done_bg.lock().unwrap() += 1;
        }
    });

    // TUI render loop
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let mut frame: usize = 0;

    loop {
        let _ = execute!(stdout, cursor::MoveTo(0, 0));

        let snapshot: Vec<(String, i64, u8)> = items.lock().unwrap().iter()
            .map(|it| (it.short.clone(), it.delta, it.status))
            .collect();

        let all_done = snapshot.iter().all(|(_, _, s)| *s == 2);
        let done = *done_count.lock().unwrap();

        let mut out = String::new();

        if all_done {
            out.push_str(&super::ui::tui_header("\x1b[34m\u{1f4c8} Space Timeline\x1b[0m"));
        } else {
            out.push_str(&super::ui::tui_header_animated("\x1b[34m\u{1f4c8} Space Timeline\x1b[0m", frame));
        }

        // Auto-scroll
        let max_visible: usize = 14;
        let scanning_idx = snapshot.iter().position(|(_, _, s)| *s == 1).unwrap_or(0);
        let scroll = if scanning_idx > max_visible.saturating_sub(3) {
            scanning_idx.saturating_sub(max_visible / 2)
        } else {
            0
        };
        let end = (scroll + max_visible).min(snapshot.len());
        let visible = &snapshot[scroll..end];

        for (short, delta, status) in visible {
            let abs_delta = delta.unsigned_abs();
            match status {
                0 => {
                    out.push_str(&format!("  \x1b[90m\u{25cb} \u{25b8} {}\x1b[0m\r\n", short));
                }
                1 => {
                    out.push_str(&format!("  \x1b[33m{}\x1b[0m \x1b[1;33m\u{25b8} {}\x1b[0m\r\n",
                        super::ui::spinner(frame), short));
                }
                2 => {
                    if abs_delta > 50 * 1024 * 1024 {
                        let size_str = ByteSize::b(abs_delta).to_string();
                        if *delta > 0 {
                            out.push_str(&format!("  \x1b[32m\u{2713}\x1b[0m \x1b[31m\u{25b2}\x1b[0m {:>10}  {}\r\n",
                                format!("+{}", size_str), short));
                        } else {
                            out.push_str(&format!("  \x1b[32m\u{2713}\x1b[0m \x1b[32m\u{25bc}\x1b[0m {:>10}  {}\r\n",
                                format!("-{}", size_str), short));
                        }
                    } else {
                        out.push_str(&format!("  \x1b[32m\u{2713}\x1b[0m \x1b[90m\u{2014} {}\x1b[0m\r\n", short));
                    }
                }
                _ => {}
            }
        }

        if scroll > 0 {
            out.push_str(&format!("\r\n  \x1b[90m  \u{2191} {} more above\x1b[0m\r\n", scroll));
        }
        if end < snapshot.len() {
            out.push_str(&format!("  \x1b[90m  \u{2193} {} more below\x1b[0m\r\n", snapshot.len() - end));
        }

        // Footer
        out.push_str("\r\n");
        out.push_str(super::ui::footer_sep());
        if all_done {
            let total_growth: i64 = snapshot.iter().map(|(_, d, _)| d).sum();
            let changed = snapshot.iter().filter(|(_, d, _)| d.unsigned_abs() > 50 * 1024 * 1024).count();
            let total_str = ByteSize::b(total_growth.unsigned_abs()).to_string();
            if changed == 0 {
                out.push_str("  \x1b[32m\u{2713}\x1b[0m No significant changes\r\n");
            } else if total_growth > 0 {
                out.push_str(&format!("  {} changed \u{2014} Net growth: \x1b[31m+{}\x1b[0m\r\n", changed, total_str));
            } else {
                out.push_str(&format!("  {} changed \u{2014} Net freed: \x1b[32m-{}\x1b[0m\r\n", changed, total_str));
            }
            out.push_str("\r\n  \x1b[90mPress any key to exit\x1b[0m\r\n");
        } else {
            out.push_str(&format!("  \x1b[33m\u{2022}\x1b[0m Scanning ({}/{})...\r\n", done, total));
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
        if prev_size < 100 * 1024 * 1024 { continue; }
        let path = std::path::Path::new(path_str.as_str());
        if !path.exists() { continue; }
        let current_size = scanner::scan_size_native(path);
        let delta = current_size as i64 - prev_size as i64;
        if delta.unsigned_abs() > 50 * 1024 * 1024 {
            changes.push(TimelineEntry {
                path: path_str.clone(), previous_size: prev_size,
                current_size, delta,
                direction: if delta > 0 { "grew" } else { "shrank" }.to_string(),
            });
            total_growth += delta;
        }
    }
    changes.sort_by(|a, b| b.delta.unsigned_abs().cmp(&a.delta.unsigned_abs()));
    output::print_json(&TimelineOutput { changes, total_growth });
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
