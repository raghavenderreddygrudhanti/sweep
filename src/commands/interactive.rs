use crossterm::{terminal, cursor, execute, event};
use crossterm::event::{Event, KeyCode};
use std::io::{self, Write};

const MENU: &[(&str, &str)] = &[
    ("Clean",     "Free up disk space"),
    ("Uninstall", "Remove apps completely"),
    ("Optimize",  "Refresh caches and services"),
    ("Analyze",   "Explore disk usage"),
    ("Status",    "Monitor system health"),
];

pub fn run() {
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let mut selected: usize = 0;

    loop {
        let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All));

        let mut out = String::new();
        out.push_str("\r\n");
        out.push_str("    \x1b[36m____\x1b[0m\r\n");
        out.push_str("   \x1b[36m/ ___|\x1b[0m_      _____  ___ _ __\r\n");
        out.push_str("   \x1b[36m\\___ \\\x1b[0m\\ \\ /\\ / / _ \\/ _ \\ '_ \\\r\n");
        out.push_str("    \x1b[36m___) |\x1b[0m\\ V  V /  __/  __/ |_) |\r\n");
        out.push_str("   \x1b[36m|____/\x1b[0m  \\_/\\_/ \\___|\\___| .__/\r\n");
        out.push_str("                           |_|\r\n");
        out.push_str("   \x1b[32mhttps://github.com/raghavenderreddygrudhanti/sweep\x1b[0m\r\n");
        out.push_str("   \x1b[90mFast system cleaner · Rust · macOS + Linux\x1b[0m\r\n");
        out.push_str("\r\n");

        for (i, (label, desc)) in MENU.iter().enumerate() {
            if i == selected {
                out.push_str(&format!("  \x1b[32m▶\x1b[0m {}. \x1b[1;36m{:<12}\x1b[0m {}\r\n", i+1, label, desc));
            } else {
                out.push_str(&format!("    {}. {:<12} \x1b[90m{}\x1b[0m\r\n", i+1, label, desc));
            }
        }

        out.push_str("\r\n");
        out.push_str("  \x1b[90m↑↓  |  Enter  |  M More  |  Q Quit\x1b[0m\r\n");

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected > 0 { selected -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected < MENU.len() - 1 { selected += 1; }
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    // Show more options
                    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
                    let _ = terminal::disable_raw_mode();
                    show_more_options();
                    return;
                }
                KeyCode::Enter => {
                    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
                    let _ = terminal::disable_raw_mode();
                    run_selected(selected);
                    return;
                }
                KeyCode::Char('1') => { selected = 0; }
                KeyCode::Char('2') => { selected = 1; }
                KeyCode::Char('3') => { selected = 2; }
                KeyCode::Char('4') => { selected = 3; }
                KeyCode::Char('5') => { selected = 4; }
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

fn run_selected(idx: usize) {
    match idx {
        0 => super::clean::run(false),     // Actually clean (has confirmation inside)
        1 => super::uninstall::run(false), // Actually uninstall (user selects + confirms)
        2 => super::optimize::run(false),  // Actually optimize
        3 => super::scan::run("~"),
        4 => super::status::run(),
        _ => {}
    }
}

fn show_more_options() {
    println!("\n  \x1b[1mCOMMANDS\x1b[0m");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep clean", "Free up disk space");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep uninstall", "Remove apps completely");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep optimize", "Refresh caches and services");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep scan", "Explore disk usage");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep status", "Monitor system health");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep history", "Review cleanup activity");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep ai", "Clean AI/ML caches");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep dev", "Remove old project artifacts");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep docker", "Clean Docker junk");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep installer", "Find and remove installer files");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep completion <shell>", "Setup shell tab completion");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep --help", "Show help");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep --version", "Show version");
    println!();
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep clean --dry-run", "Preview cleanup");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep optimize --dry-run", "Preview optimization");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep uninstall --dry-run", "Preview app uninstall");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep dev --dry-run", "Preview project purge");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep installer --dry-run", "Preview installer cleanup");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep dev --older-than 30", "Only clean artifacts >30 days");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep scan ~/Downloads", "Analyze specific directory");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "sweep history", "Show operation log");
    println!();
    println!("  \x1b[1mOPTIONS\x1b[0m");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "--dry-run", "Preview without deleting");
    println!("  \x1b[32m{:<28}\x1b[0m {}", "--debug", "Show detailed operation logs");
    println!();
}
