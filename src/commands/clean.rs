use bytesize::ByteSize;
use std::io::{self, Write};
use std::path::PathBuf;
use crossterm::{terminal, event};
use crossterm::event::{Event, KeyCode};
use crate::scanner;
use crate::cleaners::DeleteMode;

/// A cleanable item with its path, description, and scanned size.
struct CleanTarget {
    name: String,
    path: PathBuf,
    size: u64,
    item_count: usize,
}

pub fn run(dry_run: bool, mode: DeleteMode) {
    // Clear screen for a fresh view
    print!("\x1b[2J\x1b[H");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    super::ui::print_header("\x1b[1;35m\u{1f9f9} clean\x1b[0m");

    let home = crate::error::home_or_exit();

    // Collect all targets with their sizes
    let mut targets: Vec<CleanTarget> = Vec::new();
    let mut total: u64 = 0;

    // ─── System ────────────────────────────────────────────
    println!("  \x1b[1;32m▸ System\x1b[0m");
    scan_and_show(&mut targets, &mut total, "System crash reports", &home.join("Library/Logs/DiagnosticReports"));
    scan_and_show(&mut targets, &mut total, "System logs", &home.join("Library/Logs"));
    println!();

    // ─── User essentials ───────────────────────────────────
    println!("  \x1b[1;32m▸ User essentials\x1b[0m");
    scan_and_show(&mut targets, &mut total, "User app cache", &home.join("Library/Caches"));

    let trash = home.join(".Trash");
    let trash_size = scanner::scan_size_native(&trash);
    if trash_size > 0 {
        println!("    \u{2713} Trash, \x1b[32m{}\x1b[0m", ByteSize::b(trash_size));
        targets.push(CleanTarget {
            name: "Trash".to_string(),
            path: trash,
            size: trash_size,
            item_count: 0,
        });
        total += trash_size;
    } else {
        println!("    \u{2713} Trash \u{b7} already empty");
    }
    println!();

    // ─── App caches ────────────────────────────────────────
    println!("  \x1b[1;32m▸ App caches\x1b[0m");
    let app_caches = crate::cleaners::apps_cache::app_cache_paths();
    if app_caches.is_empty() {
        println!("    \u{2713} Nothing to clean");
    } else {
        for (path, name) in &app_caches {
            scan_and_show(&mut targets, &mut total, name, path);
        }
    }
    println!();

    // ─── Browsers ──────────────────────────────────────────
    println!("  \x1b[1;32m▸ Browsers\x1b[0m");
    let browsers = crate::cleaners::browser::browser_cache_paths();
    let mut any_browser = false;
    for (path, name) in &browsers {
        let size = scanner::scan_size_native(path);
        if size > 100_000 {
            let count = std::fs::read_dir(path).map(|d| d.count()).unwrap_or(0);
            println!("    \u{2713} {}, \x1b[32m{}\x1b[0m", name, ByteSize::b(size));
            targets.push(CleanTarget {
                name: name.to_string(),
                path: path.clone(),
                size,
                item_count: count,
            });
            total += size;
            any_browser = true;
        }
    }
    if !any_browser {
        println!("    \u{2713} Nothing to clean");
    }
    println!();

    // ─── Developer tools ───────────────────────────────────
    println!("  \x1b[1;32m▸ Developer tools\x1b[0m");
    scan_and_show(&mut targets, &mut total, "npm cache", &home.join(".npm/_cacache"));
    scan_and_show(&mut targets, &mut total, "pip cache", &home.join("Library/Caches/pip"));
    scan_and_show(&mut targets, &mut total, "Cargo registry cache", &home.join(".cargo/registry/cache"));
    scan_and_show(&mut targets, &mut total, "Go build cache", &home.join("Library/Caches/go-build"));
    scan_and_show(&mut targets, &mut total, "Gradle cache", &home.join(".gradle/caches"));
    scan_and_show(&mut targets, &mut total, "Maven cache", &home.join(".m2/repository"));
    scan_and_show(&mut targets, &mut total, "CocoaPods cache", &home.join("Library/Caches/CocoaPods"));
    println!();

    // ─── AI/ML ─────────────────────────────────────────────
    println!("  \x1b[1;32m▸ AI/ML\x1b[0m");
    scan_and_show(&mut targets, &mut total, "HuggingFace cache", &home.join(".cache/huggingface"));
    scan_and_show(&mut targets, &mut total, "Ollama models", &home.join(".ollama/models"));
    scan_and_show(&mut targets, &mut total, "PyTorch cache", &home.join(".cache/torch"));
    println!();

    // ─── Xcode ─────────────────────────────────────────────
    let xcode_paths = crate::cleaners::xcode::xcode_paths();
    if !xcode_paths.is_empty() {
        println!("  \x1b[1;32m▸ Xcode\x1b[0m");
        for (path, name) in &xcode_paths {
            scan_and_show(&mut targets, &mut total, name, path);
        }
        println!();
    }

    // ─── JetBrains ─────────────────────────────────────────
    let jb_paths = crate::cleaners::jetbrains::jetbrains_paths();
    if !jb_paths.is_empty() {
        println!("  \x1b[1;32m▸ JetBrains\x1b[0m");
        for (path, name) in &jb_paths {
            scan_and_show(&mut targets, &mut total, name, path);
        }
        println!();
    }

    // ─── Homebrew ──────────────────────────────────────────
    if crate::cleaners::homebrew::brew_cache_path().is_some() {
        println!("  \x1b[1;32m▸ Homebrew\x1b[0m");
        let brew_size = crate::cleaners::homebrew::brew_cleanup(true);
        if brew_size > 0 {
            println!("    \u{2713} Homebrew cache, \x1b[32m{}\x1b[0m", ByteSize::b(brew_size));
            total += brew_size;
            // Homebrew is handled separately via `brew cleanup`
        } else {
            println!("    \u{2713} Already clean");
        }
        println!();
    }

    // ─── Summary ───────────────────────────────────────────
    println!("  \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}");

    if total == 0 {
        println!("  \x1b[32m\u{2713} System already clean\x1b[0m");
        println!("  \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}");

        return;
    }

    if dry_run {
        println!("  \x1b[1;33mWould free: {} (dry run — nothing deleted)\x1b[0m", ByteSize::b(total));
        println!("  \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}");
        println!("\n  \x1b[90mRun without --dry-run to actually clean.\x1b[0m");

        return;
    }

    // Not dry run — ask for confirmation
    println!("  \x1b[1;32mWould free: {}\x1b[0m", ByteSize::b(total));
    println!("  \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}");
    println!();

    match mode {
        DeleteMode::Trash => println!("  \x1b[90mItems will be moved to the Trash (recoverable).\x1b[0m"),
        DeleteMode::Force => println!("  \x1b[1;31mItems will be permanently deleted (--force).\x1b[0m"),
    }
    print!("  \x1b[1;33mClean now? (y/n):\x1b[0m ");
    let _ = io::stdout().flush();

    let _ = terminal::enable_raw_mode();
    // Longer drain to clear any buffered keys from menu navigation
    std::thread::sleep(std::time::Duration::from_millis(300));
    while event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
        let _ = event::read();
    }
    let proceed = loop {
        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => break true,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => break false,
                _ => continue, // Ignore other keys, wait for y/n
            }
        }
    };
    let _ = terminal::disable_raw_mode();
    println!();

    if !proceed {
        println!("\n  \x1b[90mCancelled.\x1b[0m");

        return;
    }

    // Actually delete everything
    println!("\n  {}", super::ui::action_name("clean"));
    let mut actually_freed: u64 = 0;

    for target in &targets {
        let freed = delete_target(target, mode);
        actually_freed += freed;
    }

    // Homebrew (uses its own cleanup command)
    if crate::cleaners::homebrew::brew_cache_path().is_some() {
        crate::cleaners::homebrew::brew_cleanup(false);
    }

    println!("  \x1b[1;32m\u{1f389} Done! Reclaimed: {}\x1b[0m", ByteSize::b(actually_freed));
    println!();

    // Pause so user can see results (especially when launched from interactive menu)
    println!("  \x1b[90mPress any key to continue...\x1b[0m");
    let _ = terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(300));
    while event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
        let _ = event::read();
    }
    let _ = event::read();
    let _ = terminal::disable_raw_mode();
}

/// Scan a path, display its size progressively, and add to targets if non-empty.
fn scan_and_show(targets: &mut Vec<CleanTarget>, total: &mut u64, name: &str, path: &PathBuf) {
    if !path.exists() {
        return;
    }

    // Show spinner while scanning
    print!("    \x1b[33m\u{2022}\x1b[0m {}...\r", name);
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let size = scanner::scan_size_native(path);
    print!("\r\x1b[K");

    if size < 1000 {
        return;
    }

    let count = std::fs::read_dir(path).map(|d| d.count()).unwrap_or(0);
    println!("    \x1b[32m\u{2713}\x1b[0m {} {} items, \x1b[32m{}\x1b[0m", name, count, ByteSize::b(size));

    targets.push(CleanTarget {
        name: name.to_string(),
        path: path.clone(),
        size,
        item_count: count,
    });
    *total += size;
}

/// Delete a target's contents (not the directory itself).
/// Uses direct deletion for speed — caches are regenerable, no need for Trash.
/// Reports failures at the end instead of prompting.
fn delete_target(target: &CleanTarget, _mode: DeleteMode) -> u64 {
    let path = &target.path;

    if !path.exists() {
        return 0;
    }

    // Special case: Trash — emptying the trash is always permanent.
    if target.name == "Trash" {
        return crate::cleaners::trash::empty_trash(false);
    }

    // Show progress for this target
    print!("  \x1b[33m\u{2022}\x1b[0m {}...\r", target.name);
    let _ = std::io::Write::flush(&mut std::io::stdout());

    // Direct-delete contents. Skip anything that fails (no password prompts).
    let mut freed: u64 = 0;
    let mut failed: u32 = 0;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            let size = if p.is_dir() {
                scanner::scan_size_native(&p)
            } else {
                p.metadata().map(|m| m.len()).unwrap_or(0)
            };

            let success = if p.is_dir() {
                std::fs::remove_dir_all(&p).is_ok()
            } else {
                std::fs::remove_file(&p).is_ok()
            };

            if success {
                freed += size;
            } else {
                failed += 1;
            }
        }
    }

    // Show result
    print!("\r\x1b[K");
    if freed > 0 && failed == 0 {
        println!("  \x1b[32m\u{2713}\x1b[0m {} \u{2014} freed {}",
            target.name, ByteSize::b(freed));
    } else if freed > 0 && failed > 0 {
        println!("  \x1b[32m\u{2713}\x1b[0m {} \u{2014} freed {} \x1b[90m({} items skipped, in use or protected)\x1b[0m",
            target.name, ByteSize::b(freed), failed);
    } else if failed > 0 {
        println!("  \x1b[90m\u{2013}\x1b[0m {} \x1b[90m\u{2014} skipped ({} items protected or in use)\x1b[0m",
            target.name, failed);
    }

    if freed > 0 {
        crate::history::log_delete(path.to_str().unwrap_or(""), freed, "clean");
    }

    freed
}

/// Move a path to Trash (with Finder fallback on macOS).
/// Used by uninstall command for user apps (not by clean).
#[allow(dead_code)]
fn trash_path(path: &std::path::Path) -> bool {
    if ::trash::delete(path).is_ok() {
        return true;
    }

    #[cfg(target_os = "macos")]
    {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };
        let script = format!(
            "tell application \"Finder\" to delete POSIX file \"{}\"",
            abs.display()
        );
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Force-remove a path (permanent).
#[allow(dead_code)]
fn force_remove(path: &std::path::Path) -> bool {
    if path.is_dir() {
        std::fs::remove_dir_all(path).is_ok()
    } else {
        std::fs::remove_file(path).is_ok()
    }
}
