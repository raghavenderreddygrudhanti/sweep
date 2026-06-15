use bytesize::ByteSize;
use colored::*;
use crate::scanner;
use crate::cleaners::ai;

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    super::ui::print_header(&format!("\x1b[1;35m🤖 AI/ML Cache Clean\x1b[0m {}", mode));

    let caches = ai::ai_cache_paths();
    let mut total: u64 = 0;

    if caches.is_empty() {
        println!("  ✨ No AI/ML caches found.");
        super::ui::wait_any_key();
        return;
    }

    for (path, desc) in &caches {
        let size = scanner::scan_size(path).0;
        if size > 1_000_000 {
            let bar = "█".repeat(((size as f64 / 30_000_000_000.0) * 15.0).min(15.0) as usize);
            let empty = "░".repeat(15usize.saturating_sub(bar.len()));
            println!("  ✓ \x1b[31m{}{}\x1b[0m {:>9}  {}",
                bar, empty, ByteSize::b(size).to_string().bold(), desc.cyan());
            println!("    \x1b[90m{}\x1b[0m", path.display());
            total += size;
        }
    }

    println!("\n  {}", "─".repeat(40).dimmed());
    if total == 0 {
        println!("  ✨ No significant AI/ML caches.\n");
    } else if dry_run {
        println!("  💾 Would free: {}\n", ByteSize::b(total).to_string().bold().green());
    } else {
        for (path, _) in &caches {
            if scanner::scan_size(path).0 > 1_000_000 {
                let _ = std::fs::remove_dir_all(path);
            }
        }
        println!("  🎉 Freed: {}", ByteSize::b(total).to_string().bold().green());
    }

    super::ui::wait_any_key();
}
