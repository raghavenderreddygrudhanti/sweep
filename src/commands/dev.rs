use crate::cleaners::dev as dev_cleaner;
use crate::cleaners::DeleteMode;
use crate::scanner;
use bytesize::ByteSize;
use colored::*;
use crossterm::event::{Event, KeyCode};
use crossterm::{cursor, event, execute, terminal};
use std::io::{self, Write};
use std::time::{Duration, Instant, SystemTime};

pub fn run(dry_run: bool, older_than_days: u64, mode: DeleteMode) {
    // Clear screen for a fresh view
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();

    let dry_label = if dry_run { "(preview)" } else { "" };
    super::ui::print_header(&format!(
        "\x1b[1;36m\u{26a1} Dev Artifacts\x1b[0m {} \u{2014} older than {}d",
        dry_label, older_than_days
    ));
    print!("  \x1b[33m\u{2022}\x1b[0m Scanning...");
    let _ = io::stdout().flush();

    let start = Instant::now();
    let roots = dev_cleaner::scan_roots();
    let mut found: Vec<(String, u64, String, bool)> = vec![];

    let threshold = SystemTime::now()
        .checked_sub(Duration::from_secs(older_than_days * 86400))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    for (i, name) in dev_cleaner::DEV_ARTIFACTS.iter().enumerate() {
        print!(
            "\r\x1b[K  \x1b[33m{}\x1b[0m Scanning for {}...",
            super::ui::spinner(i),
            name
        );
        let _ = io::stdout().flush();

        for root in &roots {
            let matches = scanner::find_dirs_by_name(root, name, 4);
            for m in matches {
                let p = std::path::Path::new(&m.path);
                let is_old = p
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .map(|t| t < threshold)
                    .unwrap_or(false);
                if is_old && m.size > 10_000_000 {
                    found.push((m.path.clone(), m.size, name.to_string(), true));
                }
            }
        }
    }

    found.sort_by(|a, b| b.1.cmp(&a.1));
    let elapsed = start.elapsed().as_secs_f64();
    print!("\r\x1b[K");
    println!(
        "  \x1b[32m\u{2713}\x1b[0m Found {} items in {:.1}s",
        found.len(),
        elapsed
    );

    if found.is_empty() {
        println!("\n  \x1b[32m\u{2713}\x1b[0m No old build artifacts found.\n");
        println!("  \x1b[90mPress any key to continue...\x1b[0m");
        let _ = crossterm::terminal::enable_raw_mode();
        std::thread::sleep(std::time::Duration::from_millis(400));
        while crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
            let _ = crossterm::event::read();
        }
        let _ = crossterm::event::read();
        let _ = crossterm::terminal::disable_raw_mode();
        return;
    }

    // Switch to alternate screen for interactive selection
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let mut selected: usize = 0;

    // Drain any stray Enter from menu
    std::thread::sleep(std::time::Duration::from_millis(300));
    while event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
        let _ = event::read();
    }

    loop {
        let _ = execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        );

        let sel_total: u64 = found.iter().filter(|f| f.3).map(|f| f.1).sum();
        let sel_count = found.iter().filter(|f| f.3).count();

        let mut out = String::new();
        out.push_str(&super::ui::tui_header("Dev Artifacts"));
        out.push_str(&format!(
            "  \x1b[90m{} selected · \x1b[1;32m{}\x1b[0m\r\n",
            sel_count,
            ByteSize::b(sel_total)
        ));
        out.push_str("\r\n");

        for (i, (path, size, kind, checked)) in found.iter().take(15).enumerate() {
            let short = path.replace(
                &crate::error::home_or_exit().to_string_lossy().to_string(),
                "~",
            );
            let short_display = if short.len() > 45 {
                &short[short.len() - 45..]
            } else {
                &short
            };

            let ptr = if i == selected {
                " \x1b[32m▶\x1b[0m"
            } else {
                "  "
            };
            let chk = if *checked {
                "\x1b[32m●\x1b[0m"
            } else {
                "\x1b[90m○\x1b[0m"
            };
            let size_str = ByteSize::b(*size).to_string();

            out.push_str(&format!(
                "{} {} \x1b[1m{:>8}\x1b[0m  {} \x1b[36m({})\x1b[0m\r\n",
                ptr, chk, size_str, short_display, kind
            ));
        }

        if found.len() > 15 {
            out.push_str(&format!(
                "  \x1b[90m  ... +{} more\x1b[0m\r\n",
                found.len() - 15
            ));
        }

        out.push_str("\r\n  \x1b[90m─────────────────────────────────────────\x1b[0m\r\n");
        out.push_str(&format!(
            "  💾 {} dirs · \x1b[1;32m{}\x1b[0m\r\n\r\n",
            sel_count,
            ByteSize::b(sel_total)
        ));
        out.push_str(
            "  \x1b[90m↑↓ move · Space select · a all · n none · Enter clean · q quit\x1b[0m\r\n",
        );

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected < found.len().min(15) - 1 {
                        selected += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    if selected < found.len() {
                        found[selected].3 = !found[selected].3;
                    }
                }
                KeyCode::Char('a') => {
                    for f in found.iter_mut() {
                        f.3 = true;
                    }
                }
                KeyCode::Char('n') => {
                    for f in found.iter_mut() {
                        f.3 = false;
                    }
                }
                KeyCode::Enter => {
                    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
                    let _ = terminal::disable_raw_mode();

                    if dry_run {
                        let sel_total: u64 = found.iter().filter(|f| f.3).map(|f| f.1).sum();
                        println!(
                            "\n  💾 Would free: {}",
                            ByteSize::b(sel_total).to_string().bold().green()
                        );
                        println!("  Run `sweep dev` (without --dry-run) to delete.\n");
                    } else {
                        // Show delete mode
                        match mode {
                            DeleteMode::Trash => {
                                println!("\n  \x1b[90mMoving to Trash (recoverable)...\x1b[0m")
                            }
                            DeleteMode::Force => {
                                println!("\n  \x1b[31mPermanently deleting (--force)...\x1b[0m")
                            }
                        }
                        let mut freed: u64 = 0;
                        for (path, size, _, checked) in &found {
                            if *checked {
                                let p = std::path::Path::new(path);
                                let ok = match mode {
                                    DeleteMode::Trash => crate::cleaners::trash_delete(p).is_ok(),
                                    DeleteMode::Force => std::fs::remove_dir_all(p).is_ok(),
                                };
                                if ok {
                                    freed += size;
                                    crate::history::log_delete(
                                        path,
                                        *size,
                                        if mode == DeleteMode::Trash {
                                            "trash"
                                        } else {
                                            "delete"
                                        },
                                    );
                                }
                            }
                        }
                        println!(
                            "\n  🎉 Freed: {}\n",
                            ByteSize::b(freed).to_string().bold().green()
                        );
                    }

                    // Pause so user can see results
                    println!("  \x1b[90mPress any key to continue...\x1b[0m");
                    let _ = terminal::enable_raw_mode();
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    while event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                        let _ = event::read();
                    }
                    let _ = event::read();
                    let _ = terminal::disable_raw_mode();
                    return;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    break;
                }
                _ => {}
            }
        }
    }

    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();
}
