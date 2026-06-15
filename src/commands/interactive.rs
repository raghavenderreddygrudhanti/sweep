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
    println!("\n  \x1b[1mMore Commands:\x1b[0m\n");
    println!("  sweep ai           Clean AI/ML caches (HuggingFace, Ollama)");
    println!("  sweep dev          Clean build artifacts (node_modules, target)");
    println!("  sweep docker       Clean Docker junk");
    println!("  sweep installer    Remove .dmg/.pkg files");
    println!("  sweep scan <path>  Analyze specific directory");
    println!();
}
