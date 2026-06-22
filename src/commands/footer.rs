use crossterm::event::{Event, KeyCode};
use crossterm::{event, terminal};

/// Wait for a single keypress. Returns true if user wants to quit.
pub fn wait_for_key() -> bool {
    print!("\n  \x1b[90m[Enter] continue · [q] quit\x1b[0m ");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    // Small delay to flush any buffered keystrokes from menu
    std::thread::sleep(std::time::Duration::from_millis(150));

    let _ = terminal::enable_raw_mode();

    // Drain any leftover key events from previous screens
    while event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
        let _ = event::read();
    }

    // Now wait for actual user input
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
