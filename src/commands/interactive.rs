use crossterm::{terminal, cursor, execute, event};
use crossterm::event::Event;
use std::io::{self, Write};
use bytesize::ByteSize;

const MENU: &[(&str, &str, &str)] = &[
    ("clean",     "Free up disk space (caches, logs, browsers)",  "\x1b[32m"),
    ("AI/ML",     "Clean HuggingFace, Ollama, PyTorch caches",    "\x1b[35m"),
    ("Dev",       "Remove old node_modules, target, .venv",       "\x1b[36m"),
    ("Uninstall", "Remove apps and leftover files",               "\x1b[33m"),
    ("Analyze",   "Explore what's eating disk space",             "\x1b[34m"),
    ("Optimize",  "Flush DNS, rebuild caches, refresh services",  "\x1b[37m"),
    ("Recommend", "Smart suggestions for space recovery",         "\x1b[32m"),
    ("Timeline",  "See what grew or shrank recently",             "\x1b[90m"),
    ("Status",    "Real-time system monitor (CPU/RAM/Disk)",      "\x1b[90m"),
];

pub fn run() {
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let mut selected: usize = 0;
    let mut frame: usize = 0;

    // Get disk info once for the header
    let disk_info = get_disk_summary();

    loop {
        let _ = execute!(stdout, cursor::MoveTo(0, 0));

        let mut out = String::new();
        out.push_str(&super::ui::tui_header_animated("", frame));

        // Disk usage bar
        out.push_str(&disk_info);
        out.push_str("\r\n");

        for (i, (label, desc, color)) in MENU.iter().enumerate() {
            if i == selected {
                out.push_str(&format!("  \x1b[32m\u{25b6}\x1b[0m {}. {}\x1b[1m{:<12}\x1b[0m {}\r\n", i+1, color, label, desc));
            } else {
                out.push_str(&format!("    {}. {:<12} \x1b[90m{}\x1b[0m\r\n", i+1, label, desc));
            }
        }

        out.push_str("\r\n");
        out.push_str("  \x1b[90m\u{2191}\u{2193} Navigate  |  Enter Select  |  M More  |  Q Quit\x1b[0m\r\n");
        out.push_str("\x1b[J"); // Clear rest of screen

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        // Poll with timeout for animation
        if event::poll(std::time::Duration::from_millis(600)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                let action = super::ui::map_key(key);
                match action {
                    super::ui::NavAction::Up => {
                        if selected > 0 { selected -= 1; }
                    }
                    super::ui::NavAction::Down => {
                        if selected < MENU.len() - 1 { selected += 1; }
                    }
                    super::ui::NavAction::Char('m') | super::ui::NavAction::Char('M') => {
                        let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
                        let _ = terminal::disable_raw_mode();
                        show_more_options();
                        let _ = terminal::enable_raw_mode();
                        let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);
                    }
                    super::ui::NavAction::Select => {
                        let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
                        let _ = terminal::disable_raw_mode();
                        run_selected(selected);
                        let _ = terminal::enable_raw_mode();
                        let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);
                    }
                    super::ui::NavAction::Char('1') => { selected = 0; }
                    super::ui::NavAction::Char('2') => { selected = 1; }
                    super::ui::NavAction::Char('3') => { selected = 2; }
                    super::ui::NavAction::Char('4') => { selected = 3; }
                    super::ui::NavAction::Char('5') => { selected = 4; }
                    super::ui::NavAction::Char('6') => { selected = 5; }
                    super::ui::NavAction::Char('7') => { selected = 6; }
                    super::ui::NavAction::Char('8') => { selected = 7; }
                    super::ui::NavAction::Char('9') => { selected = 8; }
                    super::ui::NavAction::Back | super::ui::NavAction::Quit => {
                        break;
                    }
                    _ => {}
                }
            }
        }
        frame += 1;
    }

    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();
}

fn run_selected(idx: usize) {
    use crate::cleaners::DeleteMode;
    match idx {
        0 => super::clean::run(false, DeleteMode::Trash),
        1 => super::ai::run(false, DeleteMode::Trash),
        2 => super::dev::run(false, 7, DeleteMode::Trash),
        3 => super::uninstall::run(false, DeleteMode::Trash),
        4 => super::scan::run("~"),
        5 => super::optimize::run(false),
        6 => super::recommend::run(),
        7 => super::timeline::run(),
        8 => super::status::run(),
        _ => {}
    }
}

/// Get a one-line disk usage summary with visual bar.
fn get_disk_summary() -> String {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();

    // Find the main disk (mounted at /)
    for disk in disks.list() {
        if disk.mount_point() == std::path::Path::new("/") {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total - available;
            let pct = (used as f64 / total as f64 * 100.0) as u8;

            // Visual bar (30 chars wide)
            let bar_width = 30;
            let filled = (pct as usize * bar_width) / 100;
            let empty = bar_width - filled;

            let bar_color = if pct > 90 { "\x1b[31m" }      // red
                else if pct > 75 { "\x1b[33m" }             // yellow
                else { "\x1b[32m" };                         // green

            return format!(
                "  \x1b[1mDisk:\x1b[0m {}{}{}\x1b[90m{}\x1b[0m {} / {} ({}% used)\r\n",
                bar_color,
                "\u{2588}".repeat(filled),
                "\x1b[0m",
                "\u{2591}".repeat(empty),
                ByteSize::b(used),
                ByteSize::b(total),
                pct
            );
        }
    }

    String::from("  \x1b[90mDisk info unavailable\x1b[0m\r\n")
}

fn show_more_options() {
    super::ui::print_header("\x1b[1mAll Commands\x1b[0m");
    println!("  \x1b[1mCLEANING\x1b[0m");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep clean", "Free up disk space (all caches)");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep ai", "Clean AI/ML model caches");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep dev", "Remove old build artifacts");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep docker", "Clean Docker images and volumes");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep installer", "Remove .dmg/.pkg files");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep uninstall", "Remove apps completely");
    println!();
    println!("  \x1b[1mANALYSIS\x1b[0m");
    println!("  \x1b[34m{:<28}\x1b[0m {}", "sweep scan [path]", "Interactive disk explorer");
    println!("  \x1b[34m{:<28}\x1b[0m {}", "sweep recommend", "Smart cleanup suggestions");
    println!("  \x1b[34m{:<28}\x1b[0m {}", "sweep timeline", "What grew/shrank since last scan");
    println!("  \x1b[34m{:<28}\x1b[0m {}", "sweep status", "Real-time system monitor");
    println!("  \x1b[34m{:<28}\x1b[0m {}", "sweep history", "Past cleanup operations");
    println!();
    println!("  \x1b[1mSYSTEM\x1b[0m");
    println!("  \x1b[33m{:<28}\x1b[0m {}", "sweep optimize", "Flush DNS, rebuild caches");
    println!("  \x1b[33m{:<28}\x1b[0m {}", "sweep completion <shell>", "Shell tab completion setup");
    println!();
    println!("  \x1b[1mFLAGS\x1b[0m");
    println!("  \x1b[90m{:<28}\x1b[0m {}", "--dry-run", "Preview without deleting");
    println!("  \x1b[90m{:<28}\x1b[0m {}", "--json", "Machine-readable JSON output");
    println!("  \x1b[90m{:<28}\x1b[0m {}", "--force", "Permanently delete (skip Trash)");
    println!("  \x1b[90m{:<28}\x1b[0m {}", "--older-than <days>", "Min age for dev artifacts (default: 7)");
    println!();
    println!("  \x1b[90mPress any key to return...\x1b[0m");

    let _ = crossterm::terminal::enable_raw_mode();
    let _ = crossterm::event::read();
    let _ = crossterm::terminal::disable_raw_mode();
}
