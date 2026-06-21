use std::io::{self, Write};
use bytesize::ByteSize;
use crossterm::{terminal, cursor, execute, event};
use crossterm::event::Event;
use crate::cleaners::apps;
use crate::cleaners::DeleteMode;

pub fn run(dry_run: bool, _mode: DeleteMode) {
    // Go straight to TUI — scan in background
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    // Show loading screen
    let mut out = String::new();
    out.push_str(&super::ui::tui_header("App Uninstaller"));
    out.push_str("  ⏳ Scanning /Applications...\r\n");
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();

    let mut apps_list = apps::find_installed_apps();

    if apps_list.is_empty() {
        let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
        let _ = terminal::disable_raw_mode();
        println!("  No apps found.\n");
        return;
    }

    let mut selected: usize = 0;
    let mut marked: Vec<bool> = vec![false; apps_list.len()];
    let max_display = 18;

    loop {
        let _ = execute!(stdout, cursor::MoveTo(0, 0));

        let marked_count = marked.iter().filter(|&&m| m).count();
        let marked_size: u64 = apps_list.iter().enumerate()
            .filter(|(i, _)| marked[*i])
            .map(|(_, a)| a.size)
            .sum();

        let mut out = String::new();
        out.push_str(&super::ui::tui_header("App Uninstaller"));
        out.push_str(&format!("  \x1b[90m{} apps\x1b[0m", apps_list.len()));
        if marked_count > 0 {
            out.push_str(&format!(" · \x1b[1;32m{} selected ({})\x1b[0m", marked_count, ByteSize::b(marked_size)));
        }
        out.push_str("\r\n\r\n");

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
        out.push_str(super::ui::footer_sep());
        if marked_count > 0 {
            out.push_str(&super::ui::footer_selected(marked_count));
        } else {
            out.push_str(super::ui::footer_list());
        }
        out.push_str("\x1b[J"); // Clear rest of screen

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        if let Ok(Event::Key(key)) = event::read() {
            let action = super::ui::map_key(key);
            match action {
                super::ui::NavAction::Up => {
                    if selected > 0 { selected -= 1; }
                }
                super::ui::NavAction::Down => {
                    if selected < apps_list.len() - 1 { selected += 1; }
                }
                super::ui::NavAction::Toggle => {
                    marked[selected] = !marked[selected];
                }
                super::ui::NavAction::SelectAll => {
                    for m in marked.iter_mut() { *m = true; }
                }
                super::ui::NavAction::ClearAll => {
                    for m in marked.iter_mut() { *m = false; }
                }
                super::ui::NavAction::Select | super::ui::NavAction::Delete => {
                    if marked_count > 0 {
                        // Show confirmation in the TUI footer
                        let _ = stdout.write_all(format!(
                            "\r\n  \x1b[1;31m⚠ Delete {} app(s)? (y/n):\x1b[0m ", marked_count
                        ).as_bytes());
                        let _ = stdout.flush();

                        let confirm = loop {
                            if let Ok(Event::Key(k)) = event::read() {
                                let a = super::ui::map_key(k);
                                match a {
                                    super::ui::NavAction::Char('y') | super::ui::NavAction::Char('Y') => break true,
                                    _ => break false,
                                }
                            }
                        };

                        if confirm {
                            // Show progress in the TUI
                            let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All));
                            let mut progress = super::ui::tui_header("App Uninstaller");
                            progress.push_str(&format!("  {}\r\n\r\n", super::ui::action_name("uninstall")));
                            let _ = stdout.write_all(progress.as_bytes());
                            let _ = stdout.flush();

                            for (i, app) in apps_list.iter().enumerate() {
                                if marked[i] {
                                    // Use mv to Trash instead of osascript
                                    let trash = crate::error::home_or_exit().join(".Trash");
                                    let dest = trash.join(app.path.file_name().unwrap_or_default());
                                    let _ = std::process::Command::new("mv")
                                        .arg(&app.path)
                                        .arg(&dest)
                                        .output();
                                    let remnants = apps::find_app_remnants(app);
                                    for r in &remnants {
                                        let rdest = trash.join(r.file_name().unwrap_or_default());
                                        let _ = std::process::Command::new("mv")
                                            .arg(r)
                                            .arg(&rdest)
                                            .output();
                                    }
                                    crate::history::log_delete(&app.path.display().to_string(), app.size, "uninstall");
                                    let msg = format!("  ✓ {} (+{} remnants)\r\n", app.name, remnants.len());
                                    let _ = stdout.write_all(msg.as_bytes());
                                    let _ = stdout.flush();
                                }
                            }

                            let _ = stdout.write_all(b"\r\n  \x1b[1;32m\xf0\x9f\x8e\x89 Done! Apps moved to Trash.\x1b[0m\r\n");
                            let _ = stdout.write_all(b"\r\n  \x1b[90mPress any key to return...\x1b[0m\r\n");
                            let _ = stdout.flush();
                            // Drain + wait
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            while event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                                let _ = event::read();
                            }
                            let _ = event::read(); // wait for keypress
                        }
                        // Refresh list after deletion
                        apps_list = apps::find_installed_apps();
                        marked = vec![false; apps_list.len()];
                        selected = 0;
                    } else if selected < apps_list.len() {
                        // Single app — confirm in TUI
                        let app = &apps_list[selected];
                        let _ = stdout.write_all(format!(
                            "\r\n  \x1b[1;31m⚠ Delete {}? (y/n):\x1b[0m ", app.name
                        ).as_bytes());
                        let _ = stdout.flush();

                        let confirm = loop {
                            if let Ok(Event::Key(k)) = event::read() {
                                let a = super::ui::map_key(k);
                                match a {
                                    super::ui::NavAction::Char('y') | super::ui::NavAction::Char('Y') => break true,
                                    _ => break false,
                                }
                            }
                        };

                        if confirm {
                            // Clear screen and show progress
                            let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All));
                            let mut progress = super::ui::tui_header("App Uninstaller");
                            progress.push_str(&format!("  {}\r\n\r\n", super::ui::action_name("uninstall")));
                            let _ = stdout.write_all(progress.as_bytes());
                            let _ = stdout.flush();

                            let trash = crate::error::home_or_exit().join(".Trash");
                            let dest = trash.join(app.path.file_name().unwrap_or_default());
                            let _ = std::process::Command::new("mv")
                                .arg(&app.path)
                                .arg(&dest)
                                .stderr(std::process::Stdio::null())
                                .output();

                            // Check if it actually got deleted
                            if app.path.exists() {
                                let _ = stdout.write_all(format!(
                                    "  \x1b[31m\u{2717}\x1b[0m {} \x1b[90m(system app, needs admin)\x1b[0m\r\n",
                                    app.name
                                ).as_bytes());
                            } else {
                                let remnants = apps::find_app_remnants(app);
                                for r in &remnants {
                                    let rdest = trash.join(r.file_name().unwrap_or_default());
                                    let _ = std::process::Command::new("mv")
                                        .arg(r)
                                        .arg(&rdest)
                                        .stderr(std::process::Stdio::null())
                                        .output();
                                }
                                crate::history::log_delete(&app.path.display().to_string(), app.size, "uninstall");
                                let _ = stdout.write_all(format!(
                                    "  \x1b[32m\u{2713}\x1b[0m {} (+{} remnants) moved to Trash\r\n",
                                    app.name, remnants.len()
                                ).as_bytes());
                            }
                            let _ = stdout.write_all(b"\r\n  \x1b[90mPress any key to continue...\x1b[0m\r\n");
                            let _ = stdout.flush();
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            while event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                                let _ = event::read();
                            }
                            let _ = event::read();
                        }
                        // Refresh list after deletion
                        apps_list = apps::find_installed_apps();
                        marked = vec![false; apps_list.len()];
                        if selected >= apps_list.len() && selected > 0 { selected -= 1; }
                    }
                }
                super::ui::NavAction::Back | super::ui::NavAction::Quit => break,
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
