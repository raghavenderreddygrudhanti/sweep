use crate::cleaners::comprehensive;
use crate::cleaners::DeleteMode;
use crate::scanner;
use bytesize::ByteSize;
use crossterm::event::{Event, KeyCode};
use crossterm::{event, terminal};
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(dry_run: bool, _mode: DeleteMode) {
    // Clear screen
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();

    super::ui::print_header("\x1b[1;35m\u{1f9f9} clean\x1b[0m");

    // Show system info
    let free_before = get_free_space();
    println!(
        "  \x1b[90mFree space: {}\x1b[0m\n",
        ByteSize::b(free_before)
    );

    let categories = comprehensive::all_categories();
    let mut total_freed: u64 = 0;
    let mut total_items: u32 = 0;
    let mut total_categories: u32 = 0;
    let mut targets: Vec<(PathBuf, u64, String)> = Vec::new();

    // Progressive scan — show each category
    for category in &categories {
        let mut category_found = false;
        let mut category_size: u64 = 0;

        for item in &category.items {
            if !item.path.exists() {
                continue;
            }
            if !item.safe {
                continue;
            } // Skip items needing review in auto-clean

            // Show spinner
            print!("  \x1b[33m\u{2022}\x1b[0m {}...\r", item.label);
            let _ = io::stdout().flush();

            let size = scanner::scan_size_native(&item.path);
            print!("\r\x1b[K");

            if size > 1000 {
                if !category_found {
                    println!("  \x1b[1;32m\u{25b8} {}\x1b[0m", category.name);
                    category_found = true;
                }

                let count = std::fs::read_dir(&item.path)
                    .map(|d| d.count())
                    .unwrap_or(0);
                if count > 0 {
                    println!(
                        "    \x1b[32m\u{2713}\x1b[0m {} {} items, \x1b[32m{}\x1b[0m",
                        item.label,
                        count,
                        ByteSize::b(size)
                    );
                } else {
                    println!(
                        "    \x1b[32m\u{2713}\x1b[0m {}, \x1b[32m{}\x1b[0m",
                        item.label,
                        ByteSize::b(size)
                    );
                }

                targets.push((item.path.clone(), size, item.label.to_string()));
                category_size += size;
                total_items += 1;
            }
        }

        // Check for review-only items (show but don't auto-clean)
        for item in &category.items {
            if !item.path.exists() {
                continue;
            }
            if item.safe {
                continue;
            }

            let size = scanner::scan_size_native(&item.path);
            if size > 50 * 1024 * 1024 {
                // Only show review items > 50MB
                if !category_found {
                    println!("  \x1b[1;32m\u{25b8} {}\x1b[0m", category.name);
                    category_found = true;
                }
                println!(
                    "    \x1b[90m\u{25cb} {} \u{2014} manual review ({})\x1b[0m",
                    item.label,
                    ByteSize::b(size)
                );
            }
        }

        if category_found {
            total_freed += category_size;
            if category_size > 0 {
                total_categories += 1;
            }
            println!();
        }
    }

    // Check for running browsers (like Mole does)
    check_running_browsers();

    // Orphan detection (like Mole's "App leftovers")
    print!("  \x1b[33m\u{2022}\x1b[0m Checking for orphaned app data...\r");
    let _ = io::stdout().flush();
    let orphans = crate::cleaners::orphans::find_orphans();
    print!("\r\x1b[K");

    if !orphans.is_empty() {
        println!("  \x1b[1;32m\u{25b8} App leftovers\x1b[0m");
        for orphan in orphans.iter().take(10) {
            let home_str = crate::error::home_or_exit().display().to_string();
            let display = orphan.path.display().to_string().replace(&home_str, "~");
            println!(
                "    \x1b[90m\u{2022}\x1b[0m {} \x1b[90m({}, {} days old)\x1b[0m",
                display,
                ByteSize::b(orphan.size),
                orphan.age_days
            );
            if let crate::cleaners::orphans::OrphanKind::LaunchAgent = orphan.kind {
                println!("      \x1b[90m\u{21b3} {}\x1b[0m", orphan.label);
            }
        }
        if orphans.len() > 10 {
            println!("    \x1b[90m... +{} more\x1b[0m", orphans.len() - 10);
        }
        println!("    \x1b[90m\u{261e} Review manually before removing\x1b[0m");
        println!();
    }

    // Summary
    println!("  \x1b[90m\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\x1b[0m");

    if total_freed == 0 {
        println!("  \x1b[32m\u{2713}\x1b[0m System already clean\n");
        println!("  \x1b[90mPress any key to continue...\x1b[0m");
        let _ = terminal::enable_raw_mode();
        std::thread::sleep(std::time::Duration::from_millis(400));
        while event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
            let _ = event::read();
        }
        let _ = event::read();
        let _ = terminal::disable_raw_mode();
        return;
    }

    if dry_run {
        println!(
            "  \x1b[1;33mWould free: {}\x1b[0m ({} items across {} categories)",
            ByteSize::b(total_freed),
            total_items,
            total_categories
        );
        println!("  \x1b[90mRun without --dry-run to actually clean.\x1b[0m\n");
        println!("  \x1b[90mPress any key to continue...\x1b[0m");
        let _ = terminal::enable_raw_mode();
        std::thread::sleep(std::time::Duration::from_millis(400));
        while event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
            let _ = event::read();
        }
        let _ = event::read();
        let _ = terminal::disable_raw_mode();
        return;
    }

    // Show top items before confirming
    println!(
        "  \x1b[1;32mWould free: {}\x1b[0m ({} items, {} categories)\n",
        ByteSize::b(total_freed),
        total_items,
        total_categories
    );

    let mut sorted = targets.clone();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    println!("  \x1b[1mLargest items:\x1b[0m");
    for (_, size, label) in sorted.iter().take(5) {
        println!(
            "    \x1b[90m\u{2022}\x1b[0m {:>9}  {}",
            ByteSize::b(*size),
            label
        );
    }
    println!();

    // Confirm
    print!("  \x1b[1;33mClean now? (y/n):\x1b[0m ");
    let _ = io::stdout().flush();

    let _ = terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(300));
    while event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
        let _ = event::read();
    }
    let proceed = loop {
        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => break true,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => break false,
                _ => continue,
            }
        }
    };
    let _ = terminal::disable_raw_mode();
    println!();

    if !proceed {
        println!("\n  \x1b[90mCancelled.\x1b[0m\n");
        return;
    }

    // Delete
    println!("\n  \x1b[33mCleaning...\x1b[0m\n");
    let mut actually_freed: u64 = 0;

    for (path, _size, label) in &targets {
        print!("  \x1b[33m\u{2022}\x1b[0m {}...\r", label);
        let _ = io::stdout().flush();

        let mut item_freed: u64 = 0;
        let mut failed: u32 = 0;

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                let sz = if p.is_dir() {
                    scanner::scan_size_native(&p)
                } else {
                    p.metadata().map(|m| m.len()).unwrap_or(0)
                };

                let ok = if p.is_dir() {
                    std::fs::remove_dir_all(&p).is_ok()
                } else {
                    std::fs::remove_file(&p).is_ok()
                };

                if ok {
                    item_freed += sz;
                } else {
                    failed += 1;
                }
            }
        }

        print!("\r\x1b[K");
        if item_freed > 0 && failed == 0 {
            println!(
                "  \x1b[32m\u{2713}\x1b[0m {} \u{2014} freed {}",
                label,
                ByteSize::b(item_freed)
            );
        } else if item_freed > 0 {
            println!(
                "  \x1b[32m\u{2713}\x1b[0m {} \u{2014} freed {} \x1b[90m({} skipped)\x1b[0m",
                label,
                ByteSize::b(item_freed),
                failed
            );
        } else if failed > 0 {
            println!(
                "  \x1b[90m\u{2013}\x1b[0m {} \x1b[90m\u{2014} skipped (protected)\x1b[0m",
                label
            );
        }

        actually_freed += item_freed;
        if item_freed > 0 {
            crate::history::log_delete(path.to_str().unwrap_or(""), item_freed, "clean");
        }
    }

    // Final summary with free space change
    let free_after = get_free_space();
    let actual_change = if free_after > free_before {
        free_after - free_before
    } else {
        actually_freed
    };

    println!("\n  \x1b[90m\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\x1b[0m");
    println!("  \x1b[1;32m\u{2713} Cleanup complete\x1b[0m");
    println!(
        "  Tracked cleanup: \x1b[1m{}\x1b[0m | Items: {} | Categories: {}",
        ByteSize::b(actually_freed),
        total_items,
        total_categories
    );
    println!(
        "  Free space change: \x1b[32m+{}\x1b[0m",
        ByteSize::b(actual_change)
    );
    println!(
        "  Free space now: \x1b[1m{}\x1b[0m\n",
        ByteSize::b(free_after)
    );

    // Pause
    println!("  \x1b[90mPress any key to continue...\x1b[0m");
    let _ = terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(400));
    while event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
        let _ = event::read();
    }
    let _ = event::read();
    let _ = terminal::disable_raw_mode();
}

/// Check running browsers and show skip messages.
fn check_running_browsers() {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_processes();

    let browsers = [
        ("chrome", "Chrome"),
        ("firefox", "Firefox"),
        ("safari", "Safari"),
        ("brave", "Brave"),
        ("arc", "Arc"),
        ("msedge", "Edge"),
    ];

    let mut any_running = false;
    for (proc_name, display_name) in &browsers {
        let running = sys
            .processes()
            .values()
            .any(|p| p.name().to_lowercase().contains(proc_name));
        if running {
            if !any_running {
                println!("  \x1b[1;32m\u{25b8} Browser status\x1b[0m");
                any_running = true;
            }
            println!(
                "    \x1b[90m\u{25cb} {} is running \u{2014} profile cache cleanup skipped\x1b[0m",
                display_name
            );
        }
    }
    if any_running {
        println!();
    }
}

/// Get current free disk space.
fn get_free_space() -> u64 {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .find(|d| d.mount_point().to_string_lossy() == "/")
        .map(|d| d.available_space())
        .unwrap_or(0)
}
