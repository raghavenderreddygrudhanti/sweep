//! Smart scan — AI-powered file classification.
//! Uses SmolLM2-135M (GGUF) for intelligent "safe to delete" decisions.
//! Falls back to rule-based heuristics when AI feature is not enabled.

use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::time::SystemTime;
use bytesize::ByteSize;
use colored::*;
use crate::scanner;

/// Classification result for a file/directory.
#[derive(Debug, Clone)]
enum Safety {
    Safe(String),    // reason
    Review(String),  // reason
    Keep(String),    // reason
}

/// A scanned item with its classification.
struct SmartItem {
    path: PathBuf,
    short: String,
    size: u64,
    safety: Safety,
}

pub fn run(use_ai: bool) {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();

    super::ui::print_header("\x1b[1;35mSmart Scan\x1b[0m");

    if use_ai {
        #[cfg(feature = "ai")]
        {
            println!("  \x1b[33m\u{2022}\x1b[0m Loading SmolLM2-135M...");
            let _ = io::stdout().flush();
            run_with_ai();
            return;
        }
        #[cfg(not(feature = "ai"))]
        {
            println!("  \x1b[33mAI feature not enabled.\x1b[0m");
            println!("  Rebuild with: \x1b[36mcargo install sweep-cli --features ai\x1b[0m");
            println!("  Using smart rules instead...\n");
        }
    }

    run_rules_based();
}

/// Rule-based smart scan (no model needed, instant).
fn run_rules_based() {
    let home = crate::error::home_or_exit();
    let home_str = home.display().to_string();

    println!("  \x1b[33m\u{2022}\x1b[0m Analyzing files...\n");
    let _ = io::stdout().flush();

    let mut safe_items: Vec<SmartItem> = Vec::new();
    let mut review_items: Vec<SmartItem> = Vec::new();
    let mut total_safe: u64 = 0;
    let mut total_review: u64 = 0;

    // Check known cache/junk directories
    let cache_dirs: Vec<(&str, PathBuf)> = vec![
        ("Gradle build cache", home.join(".gradle/caches")),
        ("Maven repository cache", home.join(".m2/repository")),
        ("npm cache", home.join(".npm/_cacache")),
        ("pip cache", home.join("Library/Caches/pip")),
        ("Cargo registry", home.join(".cargo/registry/cache")),
        ("CocoaPods cache", home.join("Library/Caches/CocoaPods")),
        ("Go build cache", home.join("Library/Caches/go-build")),
        ("Xcode DerivedData", home.join("Library/Developer/Xcode/DerivedData")),
        ("HuggingFace models", home.join(".cache/huggingface/hub")),
        ("Ollama models", home.join(".ollama/models")),
        ("PyTorch cache", home.join(".cache/torch")),
        ("App caches", home.join("Library/Caches")),
        ("System logs", home.join("Library/Logs")),
    ];

    for (name, path) in &cache_dirs {
        if !path.exists() { continue; }
        let size = scanner::scan_size_native(path);
        if size < 50 * 1024 * 1024 { continue; } // Skip < 50MB

        let short = path.display().to_string().replace(&home_str, "~");
        safe_items.push(SmartItem {
            path: path.clone(),
            short,
            size,
            safety: Safety::Safe(format!("{}, recreatable", name)),
        });
        total_safe += size;

        print!("  \x1b[32m\u{2713}\x1b[0m {:>9}  {} \x1b[90m({})\x1b[0m\n",
            ByteSize::b(size).to_string().green(),
            name,
            "cache, safe to delete");
        let _ = io::stdout().flush();
    }

    // Check for stale files in Downloads
    let downloads = home.join("Downloads");
    if downloads.exists() {
        if let Ok(entries) = std::fs::read_dir(&downloads) {
            for entry in entries.flatten() {
                let p = entry.path();
                let size = if p.is_dir() {
                    scanner::scan_size_native(&p)
                } else {
                    p.metadata().map(|m| m.len()).unwrap_or(0)
                };
                if size < 50 * 1024 * 1024 { continue; }

                let age_days = file_age_days(&p);
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                let short_name = if name.len() > 35 { format!("{}...", &name[..32]) } else { name.clone() };

                if ext == "dmg" || ext == "pkg" || ext == "iso" {
                    // Installers are always safe if app is already installed
                    safe_items.push(SmartItem {
                        path: p.clone(), short: short_name.clone(), size,
                        safety: Safety::Safe("installer file, app likely installed".into()),
                    });
                    total_safe += size;
                    print!("  \x1b[32m\u{2713}\x1b[0m {:>9}  {} \x1b[90m(installer, safe)\x1b[0m\n",
                        ByteSize::b(size).to_string().green(), short_name);
                } else if age_days > 180 {
                    review_items.push(SmartItem {
                        path: p.clone(), short: short_name.clone(), size,
                        safety: Safety::Review(format!("{} days old", age_days)),
                    });
                    total_review += size;
                    print!("  \x1b[33m?\x1b[0m {:>9}  {} \x1b[90m({} days old)\x1b[0m\n",
                        ByteSize::b(size).to_string().yellow(), short_name, age_days);
                }
                let _ = io::stdout().flush();
            }
        }
    }

    // Check for old node_modules / target dirs
    let project_dirs = [
        home.join("Projects"), home.join("projects"), home.join("code"),
        home.join("Code"), home.join("dev"), home.join("lang-chain"),
    ];
    for dir in &project_dirs {
        if !dir.exists() { continue; }
        let stale = find_stale_artifacts(dir, 30);
        for (path, size, kind) in stale {
            let short = path.display().to_string().replace(&home_str, "~");
            let short = if short.len() > 40 { format!("...{}", &short[short.len()-37..]) } else { short };
            let age = file_age_days(&path);
            safe_items.push(SmartItem {
                path: path.clone(), short: short.clone(), size,
                safety: Safety::Safe(format!("{}, stale {} days", kind, age)),
            });
            total_safe += size;
            print!("  \x1b[32m\u{2713}\x1b[0m {:>9}  {} \x1b[90m({}, {}d stale)\x1b[0m\n",
                ByteSize::b(size).to_string().green(), short, kind, age);
            let _ = io::stdout().flush();
        }
    }

    // Summary
    println!("\n  \x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m");

    if total_safe == 0 && total_review == 0 {
        println!("  \x1b[32m\u{2713}\x1b[0m System is clean! Nothing to suggest.\n");
        wait_for_key();
        return;
    }

    println!();
    if total_safe > 0 {
        println!("  \x1b[1;32mSafe to delete:\x1b[0m  {} ({} items)",
            ByteSize::b(total_safe).to_string().bold().green(), safe_items.len());
    }
    if total_review > 0 {
        println!("  \x1b[1;33mNeeds review:\x1b[0m    {} ({} items)",
            ByteSize::b(total_review).to_string().bold().yellow(), review_items.len());
    }
    println!("  \x1b[1mTotal:\x1b[0m           {}\n",
        ByteSize::b(total_safe + total_review).to_string().bold());

    // Actions
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
                crossterm::event::KeyCode::Char(c) => break c,
                crossterm::event::KeyCode::Esc => break 'q',
                _ => continue,
            }
        }
    };
    let _ = crossterm::terminal::disable_raw_mode();
    println!();

    match choice {
        'a' | 'A' => {
            println!("\n  \x1b[33mCleaning safe items...\x1b[0m");
            let mut freed: u64 = 0;
            for item in &safe_items {
                let size = item.size;
                // Delete contents (not the dir itself for cache dirs)
                if let Ok(entries) = std::fs::read_dir(&item.path) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        let ok = if p.is_dir() { std::fs::remove_dir_all(&p).is_ok() }
                            else { std::fs::remove_file(&p).is_ok() };
                        if ok { freed += p.metadata().map(|m| m.len()).unwrap_or(0); }
                    }
                }
                // For files (like .dmg), delete the file itself
                if item.path.is_file() {
                    if std::fs::remove_file(&item.path).is_ok() {
                        freed += size;
                    }
                }
            }
            println!("  \x1b[1;32m\u{2713} Freed: {}\x1b[0m\n", ByteSize::b(freed).to_string().bold());
            crate::history::log_delete("smart-scan", freed, "smart-clean");
        }
        _ => {
            println!("\n  \x1b[90mDone.\x1b[0m\n");
        }
    }

    wait_for_key();
}

/// AI-powered scan (requires `ai` feature + SmolLM2 model).
#[cfg(feature = "ai")]
fn run_with_ai() {
    use llama_crab::{Llama, LlamaParams, ChatMessage, Role};

    // Download/load SmolLM2-135M from HuggingFace
    println!("  \x1b[33m\u{2022}\x1b[0m Loading model (first run downloads 138 MB)...");
    let _ = io::stdout().flush();

    let params = LlamaParams::new("HuggingFaceTB/SmolLM2-135M-Instruct-GGUF")
        .with_hf_filename("smollm2-135m-instruct-q8_0.gguf")
        .with_n_ctx(512);

    let mut llama = match Llama::load(params) {
        Ok(m) => m,
        Err(e) => {
            println!("  \x1b[31mFailed to load model: {}\x1b[0m", e);
            println!("  Falling back to rules...\n");
            run_rules_based();
            return;
        }
    };

    println!("  \x1b[32m\u{2713}\x1b[0m Model loaded\n");

    // Find uncertain files (rules can't decide)
    let home = crate::error::home_or_exit();
    let downloads = home.join("Downloads");

    if let Ok(entries) = std::fs::read_dir(&downloads) {
        for entry in entries.flatten() {
            let p = entry.path();
            let size = p.metadata().map(|m| m.len()).unwrap_or(0);
            if size < 100 * 1024 * 1024 { continue; } // Only check >100MB files

            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            let age = file_age_days(&p);
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("unknown");

            let prompt = format!(
                "File: {}\nSize: {}\nAge: {} days\nType: .{}\n\n\
                 Is this file safe to delete from a computer? \
                 Answer in one sentence explaining why or why not.",
                name, ByteSize::b(size), age, ext
            );

            let messages = vec![
                ChatMessage::new(Role::System, "You are a helpful file cleanup assistant. Be concise."),
                ChatMessage::new(Role::User, &prompt),
            ];

            print!("  \x1b[33m\u{2022}\x1b[0m Analyzing {}... ", name);
            let _ = io::stdout().flush();

            match llama.generate_chat(&messages, 100) {
                Ok(response) => {
                    let answer = response.message.content.trim().to_string();
                    let is_safe = answer.to_lowercase().contains("safe")
                        || answer.to_lowercase().contains("can be deleted");
                    let icon = if is_safe { "\x1b[32m\u{2713}\x1b[0m" } else { "\x1b[33m?\x1b[0m" };
                    println!("{}", icon);
                    println!("    \x1b[90mAI: {}\x1b[0m\n", answer);
                }
                Err(e) => {
                    println!("\x1b[31m\u{2717}\x1b[0m");
                    println!("    \x1b[90mError: {}\x1b[0m\n", e);
                }
            }
        }
    }

    wait_for_key();
}

/// Get file age in days.
fn file_age_days(path: &Path) -> u64 {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_secs() / 86400)
        .unwrap_or(0)
}

/// Find stale node_modules/target dirs older than N days.
fn find_stale_artifacts(root: &Path, min_days: u64) -> Vec<(PathBuf, u64, &'static str)> {
    let mut results = Vec::new();
    let artifacts = ["node_modules", "target", ".venv", "__pycache__"];

    for name in &artifacts {
        let found = scanner::find_dirs_by_name(root, name, 3);
        for item in found {
            let p = Path::new(&item.path);
            let age = file_age_days(p);
            if age > min_days && item.size > 50 * 1024 * 1024 {
                results.push((p.to_path_buf(), item.size, *name));
            }
        }
    }

    results
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
