use crate::cleaners::docker;
use colored::*;
use crossterm::event::{Event, KeyCode};
use crossterm::{event, terminal};
use std::io::{self, Write};

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    super::ui::print_header(&format!(
        "\x1b[1;34m\u{1f433} Docker Cleanup\x1b[0m {}",
        mode
    ));

    match docker::docker_disk_usage() {
        Some(usage) => {
            for line in usage.lines().take(5) {
                println!("  {}", line.dimmed());
            }
            println!();

            if dry_run {
                println!("  \x1b[90mRun `sweep docker` (without --dry-run) to prune.\x1b[0m\n");
                return;
            }

            // Confirm before pruning — Docker prune is not easily reversible
            println!("  \x1b[33m\u{26a0} This will remove all stopped containers, unused\x1b[0m");
            println!("  \x1b[33m  images, dangling volumes, and build cache.\x1b[0m");
            print!("\n  \x1b[1;33mPrune Docker now? (y/n):\x1b[0m ");
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

            docker::docker_prune(false);
            println!("  \x1b[1;32m\u{2713} Docker cleaned.\x1b[0m\n");
        }
        None => {
            println!("  \x1b[33m\u{26a0}\x1b[0m  Docker not found or not running.\n");
        }
    }
}
