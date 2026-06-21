use bytesize::ByteSize;
use std::io::{self, Write};
use std::path::PathBuf;
use crossterm::{terminal, event};
use crossterm::event::{Event, KeyCode};
use crate::scanner;
use crate::cleaners::DeleteMode;

pub fn run(_dry_run: bool, _mode: DeleteMode) {
    super::ui::print_header("\x1b[1;35mClean Your Mac\x1b[0m");

    let home = dirs::home_dir().unwrap_or_default();
    let mut total_freed: u64 = 0;

    // ─── System ────────────────────────────────────────────
    println!("  \x1b[1;32m▸ System\x1b[0m");
    total_freed += clean_item("System crash reports", &home.join("Library/Logs/DiagnosticReports"), true);
    total_freed += clean_item("System logs", &PathBuf::from("/var/log"), true);
    total_freed += clean_item("System diagnostic logs", &home.join("Library/Logs"), true);
    total_freed += clean_item("Power logs", &home.join("Library/Logs/powermanagement"), true);
    println!();

    // ─── User essentials ───────────────────────────────────
    println!("  \x1b[1;32m▸ User essentials\x1b[0m");
    total_freed += clean_item("User app cache", &home.join("Library/Caches"), true);
    total_freed += clean_item("User app logs", &home.join("Library/Logs"), true);

    let trash = home.join(".Trash");
    let trash_size = scanner::scan_size(&trash).0;
    if trash_size > 0 {
        println!("    ✓ Trash, \x1b[32m{}\x1b[0m", ByteSize::b(trash_size));
        // trash cleaned in actual_clean()
        total_freed += trash_size;
    } else {
        println!("    ✓ Trash · already empty");
    }
    println!();

    // ─── App caches ────────────────────────────────────────
    println!("  \x1b[1;32m▸ App caches\x1b[0m");
    let app_caches = crate::cleaners::apps_cache::app_cache_paths();
    for (path, name) in &app_caches {
        total_freed += clean_item(name, path, true);
    }
    if app_caches.is_empty() {
        println!("    ✓ Nothing to clean");
    }
    println!();

    // ─── Browsers ──────────────────────────────────────────
    println!("  \x1b[1;32m▸ Browsers\x1b[0m");
    let browsers = crate::cleaners::browser::browser_cache_paths();
    let mut any_browser = false;
    for (path, name) in &browsers {
        let size = scanner::scan_size(path).0;
        if size > 100_000 {
            println!("    ✓ {}, \x1b[32m{}\x1b[0m", name, ByteSize::b(size));
            // cleaned in actual_clean()
            total_freed += size;
            any_browser = true;
        }
    }
    if !any_browser {
        println!("    ✓ Nothing to clean");
    }
    println!();

    // ─── Developer tools ───────────────────────────────────
    println!("  \x1b[1;32m▸ Developer tools\x1b[0m");
    total_freed += clean_item("npm cache", &home.join(".npm/_cacache"), true);
    total_freed += clean_item("pip cache", &home.join("Library/Caches/pip"), true);
    total_freed += clean_item("Cargo registry cache", &home.join(".cargo/registry/cache"), true);
    total_freed += clean_item("Go build cache", &home.join("Library/Caches/go-build"), true);
    total_freed += clean_item("Gradle cache", &home.join(".gradle/caches"), true);
    total_freed += clean_item("Maven cache", &home.join(".m2/repository"), true);
    total_freed += clean_item("CocoaPods cache", &home.join("Library/Caches/CocoaPods"), true);
    println!();

    // ─── AI/ML ─────────────────────────────────────────────
    println!("  \x1b[1;32m▸ AI/ML\x1b[0m");
    total_freed += clean_item("HuggingFace cache", &home.join(".cache/huggingface"), true);
    total_freed += clean_item("Ollama models", &home.join(".ollama/models"), true);
    total_freed += clean_item("PyTorch cache", &home.join(".cache/torch"), true);
    println!();

    // ─── Xcode ─────────────────────────────────────────────
    let xcode_paths = crate::cleaners::xcode::xcode_paths();
    if !xcode_paths.is_empty() {
        println!("  \x1b[1;32m▸ Xcode\x1b[0m");
        for (path, name) in &xcode_paths {
            total_freed += clean_item(name, path, true);
        }
        println!();
    }

    // ─── JetBrains ─────────────────────────────────────────
    let jb_paths = crate::cleaners::jetbrains::jetbrains_paths();
    if !jb_paths.is_empty() {
        println!("  \x1b[1;32m▸ JetBrains\x1b[0m");
        for (path, name) in &jb_paths {
            total_freed += clean_item(name, path, true);
        }
        println!();
    }

    // ─── Homebrew ──────────────────────────────────────────
    if crate::cleaners::homebrew::brew_cache_path().is_some() {
        println!("  \x1b[1;32m▸ Homebrew\x1b[0m");
        let brew_size = crate::cleaners::homebrew::brew_cleanup(true);
        if brew_size > 0 {
            println!("    ✓ Homebrew cache, \x1b[32m{}\x1b[0m", ByteSize::b(brew_size));
            total_freed += brew_size;
        } else {
            println!("    ✓ Already clean");
        }
        println!();
    }

    // ─── Summary ───────────────────────────────────────────
    println!("  ═══════════════════════════════════════════════");
    if total_freed > 0 {
        println!("  \x1b[1;32mWould free: {}\x1b[0m", ByteSize::b(total_freed));
        println!("  ═══════════════════════════════════════════════");
        println!();

        // Ask for confirmation
        print!("  \x1b[1;33mClean now? (y/n):\x1b[0m ");
        let _ = io::stdout().flush();

        let _ = terminal::enable_raw_mode();
        // Drain any buffered events from scrolling output
        std::thread::sleep(std::time::Duration::from_millis(150));
        while event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
            let _ = event::read();
        }
        let proceed = loop {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => break true,
                    _ => break false,
                }
            }
        };
        let _ = terminal::disable_raw_mode();
        println!();

        if proceed {
            println!("\n  {}", super::ui::action_name("clean"));
            // Actually delete everything
            actual_clean();
            println!("  \x1b[1;32m🎉 Done! Reclaimed: {}\x1b[0m", ByteSize::b(total_freed));
        } else {
            println!("\n  \x1b[90mCancelled.\x1b[0m");
        }
    } else {
        println!("  \x1b[32m✓ System already clean\x1b[0m");
        println!("  ═══════════════════════════════════════════════");
    }

    super::ui::wait_any_key();
}

/// Actually delete all cleanable items.
fn actual_clean() {
    let home = dirs::home_dir().unwrap_or_default();

    let paths_to_clean: Vec<PathBuf> = vec![
        home.join("Library/Logs/DiagnosticReports"),
        home.join("Library/Logs"),
        home.join("Library/Caches"),
    ];

    for path in &paths_to_clean {
        if path.exists() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() { let _ = std::fs::remove_dir_all(&p); }
                    else { let _ = std::fs::remove_file(&p); }
                }
            }
        }
    }

    // Browser caches
    let browsers = crate::cleaners::browser::browser_cache_paths();
    for (path, _) in &browsers {
        let _ = std::fs::remove_dir_all(path);
    }

    // App caches
    let app_caches = crate::cleaners::apps_cache::app_cache_paths();
    for (path, _) in &app_caches {
        let _ = std::fs::remove_dir_all(path);
    }

    // Trash
    crate::cleaners::trash::empty_trash(false);

    // Homebrew
    if crate::cleaners::homebrew::brew_cache_path().is_some() {
        crate::cleaners::homebrew::brew_cleanup(false);
    }
}
/// Clean a single path and show result inline.
fn clean_item(name: &str, path: &PathBuf, dry_run: bool) -> u64 {
    if !path.exists() {
        return 0;
    }

    let size = scanner::scan_size(path).0;
    if size < 1000 {
        // Too small to mention
        return 0;
    }

    let count = std::fs::read_dir(path).map(|d| d.count()).unwrap_or(0);

    if size > 0 {
        println!("    ✓ {} {} items, \x1b[32m{}\x1b[0m", name, count, ByteSize::b(size));
        if !dry_run {
            // Don't delete the parent dir, just contents
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        let _ = std::fs::remove_dir_all(&p);
                    } else {
                        let _ = std::fs::remove_file(&p);
                    }
                }
            }
            crate::history::log_delete(path.to_str().unwrap_or(""), size, "clean");
        }
    } else {
        println!("    ✓ {} · nothing to clean", name);
    }

    size
}
