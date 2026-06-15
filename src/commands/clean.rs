use bytesize::ByteSize;
use std::io::{self, Write};
use std::path::PathBuf;
use crate::scanner;

pub fn run(dry_run: bool) {
    println!();
    println!("  \x1b[35m\x1b[1mClean Your Mac\x1b[0m");
    println!();
    if dry_run {
        println!("  \x1b[90m● Use --dry-run to preview, --whitelist to manage protected paths\x1b[0m");
    }
    println!("  \x1b[90m▸ System caches need sudo. Enter continue, Space skip:\x1b[0m");
    println!();

    let home = dirs::home_dir().unwrap_or_default();
    let mut total_freed: u64 = 0;

    // ─── System ────────────────────────────────────────────
    println!("  \x1b[1;32m▸ System\x1b[0m");
    total_freed += clean_item("System crash reports", &home.join("Library/Logs/DiagnosticReports"), dry_run);
    total_freed += clean_item("System logs", &PathBuf::from("/var/log"), dry_run);
    total_freed += clean_item("System diagnostic logs", &home.join("Library/Logs"), dry_run);
    total_freed += clean_item("Power logs", &home.join("Library/Logs/powermanagement"), dry_run);
    println!();

    // ─── User essentials ───────────────────────────────────
    println!("  \x1b[1;32m▸ User essentials\x1b[0m");
    total_freed += clean_item("User app cache", &home.join("Library/Caches"), dry_run);
    total_freed += clean_item("User app logs", &home.join("Library/Logs"), dry_run);

    let trash = home.join(".Trash");
    let trash_size = scanner::scan_size(&trash).0;
    if trash_size > 0 {
        println!("    ✓ Trash, \x1b[32m{}\x1b[0m", ByteSize::b(trash_size));
        if !dry_run { crate::cleaners::trash::empty_trash(false); }
        total_freed += trash_size;
    } else {
        println!("    ✓ Trash · already empty");
    }
    println!();

    // ─── App caches ────────────────────────────────────────
    println!("  \x1b[1;32m▸ App caches\x1b[0m");
    let app_caches = crate::cleaners::apps_cache::app_cache_paths();
    for (path, name) in &app_caches {
        total_freed += clean_item(name, path, dry_run);
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
            if !dry_run { let _ = std::fs::remove_dir_all(path); }
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
    total_freed += clean_item("npm cache", &home.join(".npm/_cacache"), dry_run);
    total_freed += clean_item("pip cache", &home.join("Library/Caches/pip"), dry_run);
    total_freed += clean_item("Cargo registry cache", &home.join(".cargo/registry/cache"), dry_run);
    total_freed += clean_item("Go build cache", &home.join("Library/Caches/go-build"), dry_run);
    total_freed += clean_item("Gradle cache", &home.join(".gradle/caches"), dry_run);
    total_freed += clean_item("Maven cache", &home.join(".m2/repository"), dry_run);
    total_freed += clean_item("CocoaPods cache", &home.join("Library/Caches/CocoaPods"), dry_run);
    println!();

    // ─── AI/ML ─────────────────────────────────────────────
    println!("  \x1b[1;32m▸ AI/ML\x1b[0m");
    total_freed += clean_item("HuggingFace cache", &home.join(".cache/huggingface"), dry_run);
    total_freed += clean_item("Ollama models", &home.join(".ollama/models"), dry_run);
    total_freed += clean_item("PyTorch cache", &home.join(".cache/torch"), dry_run);
    println!();

    // ─── Xcode ─────────────────────────────────────────────
    let xcode_paths = crate::cleaners::xcode::xcode_paths();
    if !xcode_paths.is_empty() {
        println!("  \x1b[1;32m▸ Xcode\x1b[0m");
        for (path, name) in &xcode_paths {
            total_freed += clean_item(name, path, dry_run);
        }
        println!();
    }

    // ─── JetBrains ─────────────────────────────────────────
    let jb_paths = crate::cleaners::jetbrains::jetbrains_paths();
    if !jb_paths.is_empty() {
        println!("  \x1b[1;32m▸ JetBrains\x1b[0m");
        for (path, name) in &jb_paths {
            total_freed += clean_item(name, path, dry_run);
        }
        println!();
    }

    // ─── Homebrew ──────────────────────────────────────────
    if crate::cleaners::homebrew::brew_cache_path().is_some() {
        println!("  \x1b[1;32m▸ Homebrew\x1b[0m");
        let brew_size = crate::cleaners::homebrew::brew_cleanup(dry_run);
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
        if dry_run {
            println!("  \x1b[1;32mWould free: {}\x1b[0m", ByteSize::b(total_freed));
            println!("  Run `sweep clean` without --dry-run to apply.");
        } else {
            println!("  \x1b[1;32mSpace freed: {}\x1b[0m", ByteSize::b(total_freed));
        }
    } else {
        println!("  \x1b[32m✓ System already clean\x1b[0m");
    }
    println!("  ═══════════════════════════════════════════════");
    println!();
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
        }
    } else {
        println!("    ✓ {} · nothing to clean", name);
    }

    size
}
