//! Smart recommendations — scans common junk locations and suggests cleanup actions.

use crate::output::{self, RecommendOutput, Recommendation};
use crate::scanner;
use colored::Colorize;
use std::path::PathBuf;

struct RecommendSource {
    path: PathBuf,
    category: &'static str,
    description: &'static str,
    command: &'static str,
    priority: u8,
    min_size_mb: u64,
}

pub fn run() {
    let home = dirs::home_dir().unwrap_or_default();

    let sources = vec![
        // AI/ML caches
        RecommendSource {
            path: home.join(".cache/huggingface/hub"),
            category: "AI/ML",
            description: "HuggingFace model cache",
            command: "sweep ai",
            priority: 1,
            min_size_mb: 500,
        },
        RecommendSource {
            path: home.join(".ollama/models"),
            category: "AI/ML",
            description: "Ollama downloaded models",
            command: "sweep ai",
            priority: 1,
            min_size_mb: 500,
        },
        RecommendSource {
            path: home.join(".cache/torch"),
            category: "AI/ML",
            description: "PyTorch model cache",
            command: "sweep ai",
            priority: 2,
            min_size_mb: 200,
        },
        RecommendSource {
            path: home.join(".cache/pip"),
            category: "AI/ML",
            description: "pip package cache",
            command: "sweep ai",
            priority: 3,
            min_size_mb: 100,
        },
        // Docker
        RecommendSource {
            path: home.join("Library/Containers/com.docker.docker/Data"),
            category: "Docker",
            description: "Docker images, volumes, and build cache",
            command: "sweep docker",
            priority: 1,
            min_size_mb: 1000,
        },
        // System caches
        RecommendSource {
            path: home.join("Library/Caches"),
            category: "System",
            description: "Application caches",
            command: "sweep clean",
            priority: 2,
            min_size_mb: 500,
        },
        // Browser
        RecommendSource {
            path: home.join("Library/Caches/Google/Chrome"),
            category: "Browser",
            description: "Chrome browser cache",
            command: "sweep clean",
            priority: 3,
            min_size_mb: 200,
        },
        // Xcode
        RecommendSource {
            path: home.join("Library/Developer/Xcode/DerivedData"),
            category: "Developer",
            description: "Xcode DerivedData (build artifacts)",
            command: "sweep dev",
            priority: 1,
            min_size_mb: 1000,
        },
        // Logs
        RecommendSource {
            path: home.join("Library/Logs"),
            category: "System",
            description: "System and application logs",
            command: "sweep clean",
            priority: 3,
            min_size_mb: 200,
        },
        // Trash
        RecommendSource {
            path: home.join(".Trash"),
            category: "Trash",
            description: "Files in Trash (already deleted)",
            command: "sweep clean",
            priority: 2,
            min_size_mb: 500,
        },
        // Downloads (old installers)
        RecommendSource {
            path: home.join("Downloads"),
            category: "Installers",
            description: ".dmg/.pkg installer files in Downloads",
            command: "sweep installer",
            priority: 2,
            min_size_mb: 500,
        },
        // Conda
        RecommendSource {
            path: home.join("miniconda3/pkgs"),
            category: "AI/ML",
            description: "Conda package cache",
            command: "sweep ai",
            priority: 2,
            min_size_mb: 500,
        },
        RecommendSource {
            path: home.join("anaconda3/pkgs"),
            category: "AI/ML",
            description: "Anaconda package cache",
            command: "sweep ai",
            priority: 2,
            min_size_mb: 500,
        },
    ];

    // Also check for Linux paths
    #[cfg(target_os = "linux")]
    let sources = {
        let mut s = sources;
        s.push(RecommendSource {
            path: home.join(".cache"),
            category: "System",
            description: "User cache directory",
            command: "sweep clean",
            priority: 2,
            min_size_mb: 500,
        });
        s
    };

    let mut recommendations: Vec<Recommendation> = Vec::new();
    let mut total_reclaimable: u64 = 0;

    for source in &sources {
        if !source.path.exists() {
            continue;
        }

        let size = scanner::scan_size_native(&source.path);
        let size_mb = size / (1024 * 1024);

        if size_mb >= source.min_size_mb {
            total_reclaimable += size;
            recommendations.push(Recommendation {
                priority: source.priority,
                category: source.category.to_string(),
                description: format!("{} ({} files)", source.description, "many"),
                size,
                command: source.command.to_string(),
            });
        }
    }

    // Also scan for node_modules and target directories
    let dev_targets = scan_dev_artifacts(&home);
    for (desc, size, cmd) in dev_targets {
        if size > 100 * 1024 * 1024 {
            total_reclaimable += size;
            recommendations.push(Recommendation {
                priority: 2,
                category: "Developer".to_string(),
                description: desc,
                size,
                command: cmd,
            });
        }
    }

    // Sort by size descending
    recommendations.sort_by(|a, b| b.size.cmp(&a.size));

    if output::is_json() {
        output::print_json(&RecommendOutput {
            recommendations,
            total_reclaimable,
        });
        return;
    }

    // Human-readable output
    println!("\n  {} Sweep Recommendations\n", "\u{1f4a1}");

    if recommendations.is_empty() {
        println!("  Your system looks clean! No major reclaimable space found.\n");
        return;
    }

    for (i, rec) in recommendations.iter().enumerate().take(10) {
        let size_str = bytesize::ByteSize::b(rec.size).to_string();
        let priority_icon = match rec.priority {
            1 => "\u{1f534}",  // red circle
            2 => "\u{1f7e1}",  // yellow circle
            _ => "\u{1f7e2}",  // green circle
        };

        println!(
            "  {} {}. {:>8}  {}",
            priority_icon,
            i + 1,
            size_str.bold(),
            rec.description
        );
        println!(
            "     {} {}",
            "\u{2192}".dimmed(),
            rec.command.cyan()
        );
        println!();
    }

    let total_str = bytesize::ByteSize::b(total_reclaimable).to_string();
    println!(
        "  Total reclaimable: {}\n",
        total_str.bold().green()
    );
}

/// Quick scan for common dev artifacts without deep traversal.
fn scan_dev_artifacts(home: &PathBuf) -> Vec<(String, u64, String)> {
    let mut results = Vec::new();

    // Check common project locations for node_modules
    let project_dirs = [
        home.join("Projects"),
        home.join("projects"),
        home.join("code"),
        home.join("Code"),
        home.join("dev"),
        home.join("Development"),
        home.join("workspace"),
        home.join("src"),
    ];

    let mut node_modules_total: u64 = 0;
    let mut node_count: u32 = 0;
    let mut target_total: u64 = 0;
    let mut target_count: u32 = 0;

    for dir in &project_dirs {
        if !dir.exists() {
            continue;
        }

        // Only go 3 levels deep to find node_modules / target
        let found = scanner::find_dirs_by_name(dir, "node_modules", 3);
        for item in &found {
            node_modules_total += item.size;
            node_count += 1;
        }

        let found = scanner::find_dirs_by_name(dir, "target", 3);
        for item in &found {
            target_total += item.size;
            target_count += 1;
        }
    }

    if node_modules_total > 0 {
        results.push((
            format!("{} node_modules directories", node_count),
            node_modules_total,
            "sweep dev --older-than 7".to_string(),
        ));
    }

    if target_total > 0 {
        results.push((
            format!("{} Cargo target/ directories", target_count),
            target_total,
            "sweep dev --older-than 14".to_string(),
        ));
    }

    results
}
