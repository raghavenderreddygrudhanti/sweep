use bytesize::ByteSize;
use colored::*;
use std::io::{self, Write};
use crate::scanner;
use crate::cleaners::ai;
use crate::cleaners::DeleteMode;

pub fn run(dry_run: bool, mode: DeleteMode) {
    let dry_label = if dry_run { " (preview)" } else { "" };
    super::ui::print_header(&format!("\x1b[1;35m\u{1f916} AI/ML Cache Clean\x1b[0m{}", dry_label));

    print!("  \x1b[90m\u{23f3} Scanning AI/ML caches...\x1b[0m");
    let _ = io::stdout().flush();

    let caches = ai::ai_cache_paths();
    let mut found: Vec<(&std::path::Path, &str, u64)> = Vec::new();

    for (path, desc) in &caches {
        let size = scanner::scan_size_native(path);
        if size > 1_000_000 {
            found.push((path.as_path(), desc, size));
        }
    }

    // Clear spinner
    print!("\r\x1b[K");
    let _ = io::stdout().flush();

    if found.is_empty() {
        println!("  \u{2728} No significant AI/ML caches found.\n");
        super::ui::wait_any_key();
        return;
    }

    let mut total: u64 = 0;
    for (path, desc, size) in &found {
        let bar_len = ((*size as f64 / 30_000_000_000.0) * 15.0).min(15.0) as usize;
        let bar = "\u{2588}".repeat(bar_len);
        let empty = "\u{2591}".repeat(15usize.saturating_sub(bar_len));
        println!("  \u{2713} \x1b[31m{}{}\x1b[0m {:>9}  {}",
            bar, empty, ByteSize::b(*size).to_string().bold(), desc.cyan());
        println!("    \x1b[90m{}\x1b[0m", path.display());
        total += size;
    }

    println!("\n  {}", "\u{2500}".repeat(40).dimmed());

    if dry_run {
        println!("  \u{1f4be} Would free: {}", ByteSize::b(total).to_string().bold().green());
        println!("  \x1b[90mRun without --dry-run to actually clean.\x1b[0m\n");
        super::ui::wait_any_key();
        return;
    }

    // Ask confirmation
    println!("  Total: {}\n", ByteSize::b(total).to_string().bold().green());
    print!("  \x1b[1;33mClean AI/ML caches? (y/n/q):\x1b[0m ");
    let _ = io::stdout().flush();

    let _ = crossterm::terminal::enable_raw_mode();
    // Drain buffered input
    while crossterm::event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let proceed = loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            match key.code {
                crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => break true,
                _ => break false,
            }
        }
    };
    let _ = crossterm::terminal::disable_raw_mode();
    println!();

    if !proceed {
        println!("\n  \x1b[90mCancelled.\x1b[0m");
        super::ui::wait_any_key();
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
    super::ui::wait_any_key();
}
