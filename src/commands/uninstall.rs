use std::io::{self, Write};
use bytesize::ByteSize;
use crossterm::{terminal, cursor, execute, event};
use crossterm::event::{Event, KeyCode};
use crate::cleaners::apps;

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    println!("\n  {} {}", "🗑  App Uninstaller".bold_red(), mode);
    print!("  ⏳ Scanning /Applications...");
    let _ = io::stdout().flush();

    let apps_list = apps::find_installed_apps();
    println!("\r  Found {} apps\x1b[K\n", apps_list.len());

    if apps_list.is_empty() {
        println!("  No apps found.\n");
        super::footer::wait_for_key();
        return;
    }

    // Interactive selection
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let mut selected: usize = 0;
    let mut marked: Vec<bool> = vec![false; apps_list.len()];
    let max_display = 18;

    loop {
        let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All));

        let marked_count = marked.iter().filter(|&&m| m).count();
        let marked_size: u64 = apps_list.iter().enumerate()
            .filter(|(i, _)| marked[*i])
            .map(|(_, a)| a.size)
            .sum();

        let mut out = String::new();
        out.push_str(&format!("\r\n  \x1b[1;31m🗑  App Uninstaller\x1b[0m  {} apps", apps_list.len()));
        if marked_count > 0 {
            out.push_str(&format!(" · \x1b[1;32m{} selected ({})\x1b[0m", marked_count, ByteSize::b(marked_size)));
        }
        out.push_str("\r\n  \x1b[90m─────────────────────────────────────────\x1b[0m\r\n\r\n");

        // Scroll window
        let scroll_start = if selected >= max_display { selected - max_display + 1 } else { 0 };
        let display_end = (scroll_start + max_display).min(apps_list.len());

        for i in scroll_start..display_end {
            let app = &apps_list[i];
            let ptr = if i == selected { " \x1b[32m▶\x1b[0m" } else { "  " };
            let chk = if marked[i] { "\x1b[32m●\x1b[0m" } else { "\x1b[90m○\x1b[0m" };

            let size_str = ByteSize::b(app.size).to_string();
            let size_colored = if app.size > 2_000_000_000 { format!("\x1b[31m{}\x1b[0m", size_str) }
                else if app.size > 500_000_000 { format!("\x1b[33m{}\x1b[0m", size_str) }
                else { size_str };

            let remnants = apps::find_app_remnants(app);
            let extra = if !remnants.is_empty() {
                format!(" \x1b[90m+{} remnants\x1b[0m", remnants.len())
            } else { "".into() };

            let name = if i == selected {
                format!("\x1b[1;36m{}\x1b[0m", app.name)
            } else if marked[i] {
                format!("\x1b[32m{}\x1b[0m", app.name)
            } else {
                app.name.clone()
            };

            out.push_str(&format!("{} {} \x1b[1m{:>9}\x1b[0m  {}{}\r\n",
                ptr, chk, size_colored, name, extra));
        }

        // Footer
        out.push_str("\r\n  \x1b[90m─────────────────────────────────────────\x1b[0m\r\n");
        if marked_count > 0 {
            out.push_str(&format!("  💾 {} apps · {}\r\n", marked_count, ByteSize::b(marked_size)));
            out.push_str("  \x1b[33mSpace\x1b[0m toggle · \x1b[33mEnter/d\x1b[0m \x1b[31mDELETE selected\x1b[0m · a all · n none · q quit\r\n");
        } else {
            out.push_str("  \x1b[33mSpace\x1b[0m select apps · \x1b[33mEnter/d\x1b[0m delete · a all · q quit\r\n");
        }

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected > 0 { selected -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected < apps_list.len() - 1 { selected += 1; }
                }
                KeyCode::Char(' ') => {
                    marked[selected] = !marked[selected];
                }
                KeyCode::Char('a') => {
                    for m in marked.iter_mut() { *m = true; }
                }
                KeyCode::Char('n') => {
                    for m in marked.iter_mut() { *m = false; }
                }
                KeyCode::Enter | KeyCode::Char('d') | KeyCode::Char('D') => {
                    if marked_count > 0 {
                        let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
                        let _ = terminal::disable_raw_mode();

                        if dry_run {
                            println!("\n  💾 Would uninstall {} apps ({})",
                                marked_count, ByteSize::b(marked_size));
                            println!("  Run `sweep uninstall` without --dry-run to remove.\n");
                        } else {
                            println!("\n  Uninstalling {} apps...", marked_count);
                            for (i, app) in apps_list.iter().enumerate() {
                                if marked[i] {
                                    // Move app to Trash
                                    let _ = std::process::Command::new("osascript")
                                        .args(["-e", &format!(
                                            "tell application \"Finder\" to delete POSIX file \"{}\"",
                                            app.path.display()
                                        )]).output();
                                    // Remove remnants
                                    let remnants = apps::find_app_remnants(app);
                                    for r in &remnants {
                                        let _ = std::process::Command::new("osascript")
                                            .args(["-e", &format!(
                                                "tell application \"Finder\" to delete POSIX file \"{}\"",
                                                r.display()
                                            )]).output();
                                    }
                                    println!("  ✓ {} (+{} remnants)", app.name, remnants.len());
                                }
                            }
                            println!("\n  🎉 Done! Apps moved to Trash.\n");
                        }
                        return;
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => break,
                _ => {}
            }
        }
    }

    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();
}

// Helper trait for colored bold red (not in colored crate directly)
trait BoldRed {
    fn bold_red(&self) -> String;
}
impl BoldRed for str {
    fn bold_red(&self) -> String {
        format!("\x1b[1;31m{}\x1b[0m", self)
    }
}
