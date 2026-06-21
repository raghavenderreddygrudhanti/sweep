//! Smart recommendations — scans common junk locations and suggests cleanup actions.
//! Shows results progressively as each location is scanned.

use crate::output::{self, RecommendOutput, Recommendation};
use crate::scanner;
use colored::Colorize;
use std::path::PathBuf;

struct RecommendSource {
    path: PathBuf,
    description: &'static str,
    command: &'static str,
    priority: u8,
    min_size_mb: u64,
}

pub fn run() {
    if output::is_json() {
        run_json();
        return;
    }

    super::ui::print_header("\x1b[1;32m\u{1f4a1} Sweep Recommendations\x1b[0m");

    let home = dirs::home_dir().unwrap_or_default();
    let sources = build_sources(&home);

    let mut recommendations: Vec<Recommendation> = Vec::new();
    let mut total_reclaimable: u64 = 0;
    let mut check_count: usize = 0;

    // Progressive scan — show each item with tick as it completes
    for source in &sources {
        if !source.path.exists() {
            continue;
        }

        check_count += 1;
        print!("  \x1b[33m{}\x1b[0m Checking {}...\r",
            super::ui::spinner(check_count), source.description);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        // For Downloads, only count installer files (.dmg, .pkg, .zip)
        let size = if source.command == "sweep installer" {
            scan_installer_size(&source.path)
        } else {
            scanner::scan_size_native(&source.path)
        };
        let size_mb = size / (1024 * 1024);

        if size_mb >= source.min_size_mb {
            total_reclaimable += size;
            let icon = match source.priority {
                1 => "\x1b[31m\u{25cf}\x1b[0m",   // red dot
                2 => "\x1b[33m\u{25cf}\x1b[0m",   // yellow dot
                _ => "\x1b[32m\u{25cf}\x1b[0m",   // green dot
            };
            print!("\r\x1b[K");
            println!("  \x1b[32m\u{2713}\x1b[0m {} {:>9}  {}",
                icon,
                bytesize::ByteSize::b(size).to_string().bold(),
                source.description);
            println!("    \x1b[90m\u{2192} {}\x1b[0m", source.command);

            recommendations.push(Recommendation {
                priority: source.priority,
                category: String::new(),
                description: source.description.to_string(),
                size,
                command: source.command.to_string(),
            });
        } else {
            print!("\r\x1b[K");
        }
    }

    print!("\r\x1b[K");

    if recommendations.is_empty() {
        println!("  \x1b[32m\u{2713}\x1b[0m System looks clean! No major reclaimable space found.\n");

        return;
    }

    // Summary
    println!();
    println!("  Total reclaimable: {}", bytesize::ByteSize::b(total_reclaimable).to_string().bold().green());
    println!();
    println!("  \x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m");
    println!("  \x1b[1mActions:\x1b[0m  \x1b[32ma\x1b[0m clean all  \x1b[32m1-{}\x1b[0m clean item  \x1b[90mq\x1b[0m quit",
        recommendations.len().min(9));
    println!();
    print!("  \x1b[1;33mChoice:\x1b[0m ");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    // Read choice
    let _ = crossterm::terminal::enable_raw_mode();
    // Drain all buffered events (from scrolling, mouse, etc.)
    std::thread::sleep(std::time::Duration::from_millis(200));
    while crossterm::event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let choice = loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            match key.code {
                crossterm::event::KeyCode::Char(c) => break c,
                crossterm::event::KeyCode::Esc => break 'q',
                crossterm::event::KeyCode::Enter => continue, // ignore stray Enter
                _ => continue, // ignore unknown keys, wait for valid input
            }
        }
    };
    let _ = crossterm::terminal::disable_raw_mode();
    println!();

    match choice {
        'q' | 'Q' => {
            println!("\n  \x1b[90mDone.\x1b[0m\n");
        }
        'a' | 'A' => {
            println!("\n  \x1b[33mCleaning all...\x1b[0m");
            let mut freed: u64 = 0;
            for rec in &recommendations {
                print!("  \x1b[33m\u{2022}\x1b[0m {}...\r", rec.description);
                let _ = std::io::Write::flush(&mut std::io::stdout());
                freed += run_clean_command(&rec.command);
                print!("\r\x1b[K");
                println!("  \x1b[32m\u{2713}\x1b[0m {}", rec.description);
            }
            println!("\n  \x1b[1;32m\u{2713} Freed: {}\x1b[0m\n", bytesize::ByteSize::b(freed).to_string().bold());
        }
        c @ '1'..='9' => {
            let idx = (c as usize) - ('1' as usize);
            if idx < recommendations.len() {
                let rec = &recommendations[idx];
                println!("\n  \x1b[33m\u{2022}\x1b[0m Cleaning: {}", rec.description);
                let freed = run_clean_command(&rec.command);
                println!("  \x1b[1;32m\u{2713} Freed: {}\x1b[0m\n", bytesize::ByteSize::b(freed).to_string().bold());
            } else {
                println!("\n  \x1b[90mInvalid selection.\x1b[0m\n");
            }
        }
        _ => {
            println!("\n  \x1b[90mDone.\x1b[0m\n");
        }
    }
}

/// JSON mode — scan everything then output.
fn run_json() {
    let home = dirs::home_dir().unwrap_or_default();
    let sources = build_sources(&home);

    let mut recommendations: Vec<Recommendation> = Vec::new();
    let mut total_reclaimable: u64 = 0;

    for source in &sources {
        if !source.path.exists() { continue; }
        let size = scanner::scan_size_native(&source.path);
        if size / (1024 * 1024) >= source.min_size_mb {
            total_reclaimable += size;
            recommendations.push(Recommendation {
                priority: source.priority,
                category: String::new(),
                description: source.description.to_string(),
                size,
                command: source.command.to_string(),
            });
        }
    }

    recommendations.sort_by(|a, b| b.size.cmp(&a.size));
    output::print_json(&RecommendOutput { recommendations, total_reclaimable });
}

/// Build sources list.
fn build_sources(home: &PathBuf) -> Vec<RecommendSource> {
    vec![
        RecommendSource { path: home.join(".cache/huggingface/hub"), description: "HuggingFace model cache", command: "sweep ai", priority: 1, min_size_mb: 500 },
        RecommendSource { path: home.join(".ollama/models"), description: "Ollama downloaded models", command: "sweep ai", priority: 1, min_size_mb: 500 },
        RecommendSource { path: home.join(".cache/torch"), description: "PyTorch model cache", command: "sweep ai", priority: 2, min_size_mb: 200 },
        RecommendSource { path: home.join(".cache/pip"), description: "pip package cache", command: "sweep ai", priority: 3, min_size_mb: 100 },
        RecommendSource { path: home.join("Library/Containers/com.docker.docker/Data"), description: "Docker data", command: "sweep docker", priority: 1, min_size_mb: 1000 },
        RecommendSource { path: home.join("Library/Caches"), description: "Application caches", command: "sweep clean", priority: 2, min_size_mb: 500 },
        RecommendSource { path: home.join("Library/Caches/Google/Chrome"), description: "Chrome browser cache", command: "sweep clean", priority: 3, min_size_mb: 200 },
        RecommendSource { path: home.join("Library/Developer/Xcode/DerivedData"), description: "Xcode DerivedData", command: "sweep dev", priority: 1, min_size_mb: 1000 },
        RecommendSource { path: home.join("Library/Logs"), description: "System and app logs", command: "sweep clean", priority: 3, min_size_mb: 200 },
        RecommendSource { path: home.join(".Trash"), description: "Trash", command: "sweep clean", priority: 2, min_size_mb: 500 },
        RecommendSource { path: home.join("Downloads"), description: "Downloads (.dmg/.pkg)", command: "sweep installer", priority: 2, min_size_mb: 500 },
        RecommendSource { path: home.join("miniconda3/pkgs"), description: "Conda package cache", command: "sweep ai", priority: 2, min_size_mb: 500 },
        RecommendSource { path: home.join("anaconda3/pkgs"), description: "Anaconda package cache", command: "sweep ai", priority: 2, min_size_mb: 500 },
        RecommendSource { path: home.join(".gradle/caches"), description: "Gradle cache", command: "sweep dev", priority: 2, min_size_mb: 500 },
        RecommendSource { path: home.join(".m2/repository"), description: "Maven cache", command: "sweep dev", priority: 2, min_size_mb: 500 },
    ]
}

/// Execute a clean and return bytes freed.
fn run_clean_command(cmd: &str) -> u64 {
    let home = dirs::home_dir().unwrap_or_default();
    let mut freed: u64 = 0;

    let paths: Vec<PathBuf> = match cmd {
        "sweep ai" => vec![
            home.join(".cache/huggingface/hub"),
            home.join(".ollama/models"),
            home.join(".cache/torch"),
            home.join(".cache/pip"),
            home.join("miniconda3/pkgs"),
            home.join("anaconda3/pkgs"),
        ],
        "sweep clean" => vec![
            home.join("Library/Caches"),
            home.join("Library/Logs"),
        ],
        "sweep docker" => {
            let _ = std::process::Command::new("docker")
                .args(["system", "prune", "-f"])
                .stderr(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .status();
            return 0;
        }
        "sweep installer" => vec![home.join("Downloads")],
        "sweep dev" => vec![
            home.join("Library/Developer/Xcode/DerivedData"),
            home.join(".gradle/caches"),
            home.join(".m2/repository"),
        ],
        _ => vec![],
    };

    for path in &paths {
        if !path.exists() { continue; }

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();

                // For Downloads, only delete installers
                if cmd == "sweep installer" {
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext != "dmg" && ext != "pkg" && ext != "zip" { continue; }
                }

                let size = if p.is_dir() {
                    scanner::scan_size_native(&p)
                } else {
                    p.metadata().map(|m| m.len()).unwrap_or(0)
                };

                let ok = if p.is_dir() {
                    std::fs::remove_dir_all(&p).is_ok()
                } else {
                    std::fs::remove_file(&p).is_ok()
                };

                if ok { freed += size; }
            }
        }
    }

    if freed > 0 {
        crate::history::log_delete("recommend", freed, "clean");
    }
    freed
}

/// Only count .dmg, .pkg, .zip files in a directory (not everything).
fn scan_installer_size(path: &std::path::Path) -> u64 {
    let mut total: u64 = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "dmg" || ext == "pkg" || ext == "zip" || ext == "iso" || ext == "app" {
                    total += p.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
    }
    total
}
