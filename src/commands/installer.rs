use crate::cleaners::optimize;
use crate::cleaners::DeleteMode;
use bytesize::ByteSize;
use colored::*;
use crossterm::event::{Event, KeyCode};
use crossterm::{event, terminal};
use std::io::{self, Write};

pub fn run(dry_run: bool, mode: DeleteMode) {
    let mode_label = if dry_run { "(preview)" } else { "" };
    super::ui::print_header(&format!(
        "\x1b[1;33m\u{1f4e6} Installer Cleanup\x1b[0m {}",
        mode_label
    ));

    let installers = optimize::find_installers();

    if installers.is_empty() {
        println!("  ✨ No installer files found.\n");
        return;
    }

    let mut total: u64 = 0;
    for (path, size) in &installers {
        println!(
            "  \x1b[90m\u{2022}\x1b[0m {:>9}  {}",
            ByteSize::b(*size).to_string().bold(),
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .dimmed()
        );
        total += size;
    }

    println!("\n  {}", "─".repeat(40).dimmed());
    println!(
        "  {} files · {}",
        installers.len(),
        ByteSize::b(total).to_string().bold().green()
    );

    if dry_run {
        println!("  \x1b[90mRun without --dry-run to delete.\x1b[0m\n");
        return;
    }

    // Show delete mode
    match mode {
        DeleteMode::Trash => println!("\n  \x1b[90mWill move to Trash (recoverable)\x1b[0m"),
        DeleteMode::Force => println!("\n  \x1b[31mWill permanently delete (--force)\x1b[0m"),
    }

    // Confirm before deleting anything
    print!("  \x1b[1;33mDelete these files? (y/n):\x1b[0m ");
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

    let mut freed: u64 = 0;
    let mut failed: u32 = 0;
    for (path, size) in &installers {
        let ok = match mode {
            DeleteMode::Trash => crate::cleaners::trash_delete(path).is_ok(),
            DeleteMode::Force => std::fs::remove_file(path).is_ok(),
        };
        if ok {
            freed += size;
            crate::history::log_delete(
                &path.display().to_string(),
                *size,
                if mode == DeleteMode::Trash {
                    "trash"
                } else {
                    "delete"
                },
            );
        } else {
            failed += 1;
            eprintln!(
                "  \x1b[33m\u{26a0}\x1b[0m Could not delete: {}",
                path.display()
            );
        }
    }

    println!(
        "\n  \x1b[1;32m\u{2713} Freed: {}\x1b[0m{}",
        ByteSize::b(freed).to_string().bold(),
        if failed > 0 {
            format!(" \x1b[90m({} skipped)\x1b[0m", failed)
        } else {
            String::new()
        }
    );
    println!();
}
