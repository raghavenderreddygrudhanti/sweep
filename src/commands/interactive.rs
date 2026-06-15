use crossterm::{terminal, cursor, execute, event};
use crossterm::event::Event;
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
    let mut frame: usize = 0;

    loop {
        let _ = execute!(stdout, cursor::MoveTo(0, 0));

        let mut out = String::new();
        out.push_str(&super::ui::tui_header_animated("", frame));

        for (i, (label, desc)) in MENU.iter().enumerate() {
            if i == selected {
                out.push_str(&format!("  \x1b[32m▶\x1b[0m {}. \x1b[1;36m{:<12}\x1b[0m {}\r\n", i+1, label, desc));
            } else {
                out.push_str(&format!("    {}. {:<12} \x1b[90m{}\x1b[0m\r\n", i+1, label, desc));
            }
        }

        out.push_str("\r\n");
        out.push_str("  \x1b[90m↑↓  |  Enter  |  M More  |  Q Quit\x1b[0m\r\n");
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
    super::ui::print_header("\x1b[1mAll Commands\x1b[0m");
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
    println!();
    println!("  \x1b[1mFLAGS\x1b[0m");
    println!("  \x1b[33m{:<28}\x1b[0m {}", "--dry-run", "Preview without deleting");
    println!("  \x1b[33m{:<28}\x1b[0m {}", "--older-than <days>", "Min age for dev artifacts");
    println!();
    println!("  \x1b[90mPress any key to return...\x1b[0m");

    let _ = crossterm::terminal::enable_raw_mode();
    let _ = crossterm::event::read();
    let _ = crossterm::terminal::disable_raw_mode();
}
