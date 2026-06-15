use crossterm::{terminal, event};
use crossterm::event::{Event, KeyCode};

/// Wait for a single keypress. Returns true if user wants to quit.
pub fn wait_for_key() -> bool {
    print!("  \x1b[90m[Enter] continue · [q] quit\x1b[0m");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let _ = terminal::enable_raw_mode();
    let quit = loop {
        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => break true,
                _ => break false,
            }
        }
    };
    let _ = terminal::disable_raw_mode();
    println!();

    if quit {
        std::process::exit(0);
    }
    false
}
