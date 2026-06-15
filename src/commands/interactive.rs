use crossterm::{terminal, cursor, execute, event};
use crossterm::event::{Event, KeyCode};
use std::io::{self, Write};

const MENU: &[(&str, &str)] = &[
    ("Clean",     "Free up disk space"),
    ("AI Clean",  "Clean AI/ML caches"),
    ("Dev Clean", "Clean build artifacts"),
    ("Uninstall", "Remove apps completely"),
    ("Docker",    "Clean Docker junk"),
    ("Optimize",  "Rebuild caches & services"),
    ("Analyze",   "Explore disk usage"),
    ("Installer", "Remove .dmg/.pkg files"),
    ("Status",    "System monitor"),
];

pub fn run() {
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let mut selected: usize = 0;

    loop {
        // Clear and redraw from top
        let _ = execute!(stdout, cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All));

        // Draw menu using raw write (avoids println buffering issues)
        let mut out = String::new();
        out.push_str("\r\n");
        out.push_str("  \x1b[1m🧹 Sweep\x1b[0m\r\n");
        out.push_str("\r\n");

        for (i, (label, desc)) in MENU.iter().enumerate() {
            if i == selected {
                out.push_str(&format!(
                    "  \x1b[32m▶\x1b[0m \x1b[1;37m{:<12}\x1b[0m \x1b[36m{}\x1b[0m\r\n",
                    label, desc
                ));
            } else {
                out.push_str(&format!(
                    "    \x1b[37m{:<12}\x1b[0m \x1b[90m{}\x1b[0m\r\n",
                    label, desc
                ));
            }
        }

        out.push_str("\r\n");
        out.push_str("  \x1b[90m↑↓ navigate · Enter select · q quit\x1b[0m\r\n");

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        // Wait for key
        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected > 0 { selected -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected < MENU.len() - 1 { selected += 1; }
                }
                KeyCode::Enter => {
                    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
                    let _ = terminal::disable_raw_mode();
                    run_selected(selected);
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
    println!("  🧹 Sweep — Goodbye! Run `sweep --help` for commands.");
}

fn run_selected(idx: usize) {
    match idx {
        0 => super::clean::run(true),
        1 => super::ai::run(true),
        2 => super::dev::run(true, 7),
        3 => super::uninstall::run(true),
        4 => super::docker::run(true),
        5 => super::optimize::run(true),
        6 => super::scan::run("~"),
        7 => super::installer::run(true),
        8 => super::status::run(),
        _ => {}
    }
}
