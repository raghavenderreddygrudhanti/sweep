use bytesize::ByteSize;
use colored::*;
use std::io::{self, Write};
use crate::scanner;
use crate::cleaners::ai;
use crate::cleaners::DeleteMode;

pub fn run(dry_run: bool, mode: DeleteMode) {
    let dry_label = if dry_run { " (preview)" } else { "" };
    super::ui::print_header(&format!("\x1b[1;35m\u{1f916} AI/ML Cache Clean\x1b[0m{}", dry_label));

    let caches = ai::ai_cache_paths();
    let mut found: Vec<(&std::path::Path, &str, u64)> = Vec::new();

    // Progressive scan with tick marks
    for (path, desc) in &caches {
        if !path.exists() { continue; }

        print!("  \x1b[33m\u{2022}\x1b[0m Checking {}...\r", desc);
        let _ = io::stdout().flush();

        let size = scanner::scan_size_native(path);
        print!("\r\x1b[K");

        if size > 1_000_000 {
            let bar_len = ((size as f64 / 30_000_000_000.0) * 15.0).min(15.0) as usize;
            let bar = "\u{2588}".repeat(bar_len);
            let empty = "\u{2591}".repeat(15usize.saturating_sub(bar_len));
            println!("  \x1b[32m\u{2713}\x1b[0m \x1b[31m{}{}\x1b[0m {:>9}  {}",
                bar, empty, ByteSize::b(size).to_string().bold(), desc.cyan());
            println!("    \x1b[90m{}\x1b[0m", path.display());
            found.push((path.as_path(), desc, size));
        }
    }

    if found.is_empty() {
        println!("\n  \x1b[32m\u{2713}\x1b[0m No significant AI/ML caches found.\n");
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

    let total: u64 = found.iter().map(|(_, _, s)| s).sum();

    println!("\n  \x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m");
    println!("  Total: {}", ByteSize::b(total).to_string().bold().green());

    if dry_run {
        println!("  \x1b[90mRun without --dry-run to actually clean.\x1b[0m\n");
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

    // Ask confirmation
    println!("  Total: {}\n", ByteSize::b(total).to_string().bold().green());
    print!("  \x1b[1;33mClean AI/ML caches? (y/n/q):\x1b[0m ");
    let _ = io::stdout().flush();

    let _ = crossterm::terminal::enable_raw_mode();
    // Drain buffered input (longer delay to catch menu Enter)
    std::thread::sleep(std::time::Duration::from_millis(400));
    while crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let proceed = loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            match key.code {
                crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => break true,
                crossterm::event::KeyCode::Char('n') | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Char('Q')
                | crossterm::event::KeyCode::Esc => break false,
                _ => continue, // Ignore Enter and other keys
            }
        }
    };
    let _ = crossterm::terminal::disable_raw_mode();
    println!();

    if !proceed {
        println!("\n  \x1b[90mCancelled.\x1b[0m");

        return;
    }

    // Delete
    println!("\n  \x1b[90mCleaning...\x1b[0m");
    let mut freed: u64 = 0;
    for (path, _desc, size) in &found {
        let success = match mode {
            DeleteMode::Trash => {
                if ::trash::delete(path).is_ok() {
                    true
                } else {
                    #[cfg(target_os = "macos")]
                    {
                        let abs = if path.is_absolute() { path.to_path_buf() }
                            else { std::env::current_dir().unwrap_or_default().join(path) };
                        let script = format!(
                            "tell application \"Finder\" to delete POSIX file \"{}\"",
                            abs.display()
                        );
                        std::process::Command::new("osascript")
                            .args(["-e", &script])
                            .stderr(std::process::Stdio::null())
                            .stdout(std::process::Stdio::null())
                            .status()
                            .map(|s| s.success())
                            .unwrap_or(false)
                    }
                    #[cfg(not(target_os = "macos"))]
                    { false }
                }
            }
            DeleteMode::Force => std::fs::remove_dir_all(path).is_ok(),
        };
        if success {
            freed += size;
            crate::history::log_delete(
                &path.display().to_string(),
                *size,
                if mode == DeleteMode::Trash { "trash" } else { "delete" },
            );
        }
    }

    println!("  \x1b[1;32m\u{1f389} Done! Freed: {}\x1b[0m\n", ByteSize::b(freed).to_string().bold());

    // Pause so user can see results
    println!("  \x1b[90mPress any key to continue...\x1b[0m");
    let _ = crossterm::terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(300));
    while crossterm::event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let _ = crossterm::event::read();
    let _ = crossterm::terminal::disable_raw_mode();
}
