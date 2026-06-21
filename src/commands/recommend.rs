//! Recommend v2 — ranked decision engine.
//! Scores each item and groups into SAFE CLEAN / REVIEW / KEEP.

use std::io::{self, Write};
use std::path::PathBuf;
use bytesize::ByteSize;
use colored::*;
use crate::scanner;
use crate::recommend_engine::{self, ScoredItem, Action};
use crate::output;

pub fn run() {
    if output::is_json() {
        run_json();
        return;
    }

    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
    super::ui::print_header("\x1b[1;32mSmart Recommendations\x1b[0m");

    let home = crate::error::home_or_exit();
    let home_str = home.display().to_string();

    // Scan known locations and score each
    let sources = build_sources(&home);
    let mut items: Vec<ScoredItem> = Vec::new();

    for (path, label) in &sources {
        if !path.exists() { continue; }

        print!("  \x1b[33m{}\x1b[0m {}...\r",
            super::ui::spinner(items.len()), label);
        let _ = io::stdout().flush();

        let size = scanner::scan_size_native(path);
        if size < 10 * 1024 * 1024 { // Skip < 10MB
            print!("\r\x1b[K");
            continue;
        }

        let scored = recommend_engine::score_item(path, label, size);
        print!("\r\x1b[K");

        // Show immediately if actionable
        if scored.action != Action::Keep {
            let short = path.display().to_string().replace(&home_str, "~");
            let short = if short.len() > 35 { format!("{}...", &short[..32]) } else { short };
            println!("  {} {:>9}  {} \x1b[90m(score: {})\x1b[0m",
                scored.action.icon(),
                ByteSize::b(scored.size).to_string().bold(),
                short,
                scored.score);
            // Show top reasons
            for reason in scored.reasons.iter().take(2) {
                println!("    \x1b[90m\u{2713} {}\x1b[0m", reason);
            }
        }

        items.push(scored);
    }

    // Separate by action
    let safe_items: Vec<&ScoredItem> = items.iter()
        .filter(|i| i.action == Action::SafeClean)
        .collect();
    let review_items: Vec<&ScoredItem> = items.iter()
        .filter(|i| i.action == Action::Review)
        .collect();

    let safe_total: u64 = safe_items.iter().map(|i| i.size).sum();
    let review_total: u64 = review_items.iter().map(|i| i.size).sum();

    // Summary
    println!("\n  \x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m\n");

    if safe_items.is_empty() && review_items.is_empty() {
        println!("  \x1b[32m\u{2713}\x1b[0m System is clean — nothing to recommend.\n");
        wait_for_key();
        return;
    }

    if !safe_items.is_empty() {
        println!("  \x1b[1;32mSAFE CLEAN:\x1b[0m  {} ({} items)",
            ByteSize::b(safe_total).to_string().green().bold(),
            safe_items.len());
        println!("  \x1b[90mAll regenerable, high confidence, no risk.\x1b[0m");
    }
    if !review_items.is_empty() {
        println!("  \x1b[1;33mREVIEW:\x1b[0m      {} ({} items)",
            ByteSize::b(review_total).to_string().yellow().bold(),
            review_items.len());
        println!("  \x1b[90mMight be safe, check before deleting.\x1b[0m");
    }
    println!();

    // Impact prediction
    let total = safe_total + review_total;
    if total > 0 {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        if let Some(disk) = disks.list().iter().find(|d| d.mount_point().to_string_lossy() == "/") {
            let free = disk.available_space();
            let after = free + safe_total;
            println!("  \x1b[1mImpact:\x1b[0m");
            println!("    Free now:    {}", ByteSize::b(free));
            println!("    After clean: \x1b[32m{}\x1b[0m (+{})",
                ByteSize::b(after), ByteSize::b(safe_total));
            println!("    Risk:        \x1b[32mNone\x1b[0m (all regenerable caches)");
            println!();
        }
    }

    // Actions
    if !safe_items.is_empty() {
        println!("  \x1b[1mActions:\x1b[0m  \x1b[32ma\x1b[0m clean all safe  \x1b[90mq\x1b[0m quit");
        println!();
        print!("  \x1b[1;33mChoice:\x1b[0m ");
        let _ = io::stdout().flush();

        let _ = crossterm::terminal::enable_raw_mode();
        std::thread::sleep(std::time::Duration::from_millis(400));
        while crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
            let _ = crossterm::event::read();
        }
        let choice = loop {
            if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                match key.code {
                    crossterm::event::KeyCode::Char('a') | crossterm::event::KeyCode::Char('A') => break 'a',
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Char('Q')
                    | crossterm::event::KeyCode::Esc => break 'q',
                    _ => continue,
                }
            }
        };
        let _ = crossterm::terminal::disable_raw_mode();
        println!();

        if choice == 'a' {
            println!("\n  \x1b[33mCleaning safe items...\x1b[0m");
            let mut freed: u64 = 0;
            for item in &safe_items {
                // Use pre-scanned size (not post-delete metadata which returns 0)
                let item_size_before = item.size;
                let mut item_freed: u64 = 0;
                if let Ok(entries) = std::fs::read_dir(&item.path) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        // Measure BEFORE deleting
                        let entry_size = if p.is_dir() {
                            crate::scanner::scan_size_native(&p)
                        } else {
                            p.metadata().map(|m| m.len()).unwrap_or(0)
                        };
                        let ok = if p.is_dir() { std::fs::remove_dir_all(&p).is_ok() }
                            else { std::fs::remove_file(&p).is_ok() };
                        if ok {
                            item_freed += entry_size;
                        }
                    }
                }
                freed += item_freed;
                println!("  \x1b[32m\u{2713}\x1b[0m {} ({})", item.label, ByteSize::b(item_freed));
            }
            println!("\n  \x1b[1;32m\u{2713} Freed: {}\x1b[0m\n", ByteSize::b(freed));
            crate::history::log_delete("recommend", freed, "smart-clean");
        }
    } else {
        wait_for_key();
    }
}

fn run_json() {
    let home = crate::error::home_or_exit();
    let sources = build_sources(&home);
    let mut results = Vec::new();

    for (path, label) in &sources {
        if !path.exists() { continue; }
        let size = scanner::scan_size_native(path);
        if size < 10 * 1024 * 1024 { continue; }

        let scored = recommend_engine::score_item(path, label, size);
        if scored.action != Action::Keep {
            results.push(serde_json::json!({
                "path": path.display().to_string(),
                "label": label,
                "size": size,
                "score": scored.score,
                "action": scored.action.label(),
                "class": scored.class.label(),
                "reasons": scored.reasons,
            }));
        }
    }

    let output = serde_json::json!({
        "recommendations": results,
        "total_safe": results.iter()
            .filter(|r| r["action"] == "SAFE CLEAN")
            .filter_map(|r| r["size"].as_u64())
            .sum::<u64>(),
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
}

fn build_sources(home: &PathBuf) -> Vec<(PathBuf, &'static str)> {
    vec![
        (home.join(".cache/huggingface/hub"), "HuggingFace models"),
        (home.join(".ollama/models"), "Ollama models"),
        (home.join(".cache/torch"), "PyTorch cache"),
        (home.join(".cache/pip"), "pip cache"),
        (home.join(".cache/uv"), "uv cache"),
        (home.join("Library/Caches"), "Application caches"),
        (home.join("Library/Logs"), "System logs"),
        (home.join(".gradle/caches"), "Gradle cache"),
        (home.join(".gradle/wrapper/dists"), "Gradle wrappers"),
        (home.join(".m2/repository"), "Maven repository"),
        (home.join(".npm/_cacache"), "npm cache"),
        (home.join(".cargo/registry/cache"), "Cargo registry"),
        (home.join(".rustup/downloads"), "Rustup downloads"),
        (home.join("Library/Caches/CocoaPods"), "CocoaPods cache"),
        (home.join("Library/Caches/Homebrew"), "Homebrew cache"),
        (home.join("Library/Caches/go-build"), "Go build cache"),
        (home.join("Library/Developer/Xcode/DerivedData"), "Xcode DerivedData"),
        (home.join("Library/Developer/CoreSimulator/Caches"), "Simulator caches"),
        (home.join(".conda/pkgs"), "Conda packages"),
        (home.join("miniconda3/pkgs"), "Miniconda packages"),
        (home.join("Library/Caches/Google/Chrome"), "Chrome cache"),
        (home.join("Library/Caches/com.apple.Safari"), "Safari cache"),
        (home.join("Library/Caches/Firefox"), "Firefox cache"),
        (home.join("Library/Caches/com.microsoft.teams"), "Teams cache"),
        (home.join("Library/Caches/com.tinyspeck.slackmacgap"), "Slack cache"),
        (home.join("Library/Caches/com.hnc.Discord"), "Discord cache"),
        (home.join("Library/Caches/us.zoom.xos"), "Zoom cache"),
        (home.join(".lmstudio/models"), "LM Studio models"),
        (home.join(".codex"), "Codex cache"),
    ]
}

fn wait_for_key() {
    println!("  \x1b[90mPress any key to continue...\x1b[0m");
    let _ = crossterm::terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(400));
    while crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let _ = crossterm::event::read();
    let _ = crossterm::terminal::disable_raw_mode();
}
