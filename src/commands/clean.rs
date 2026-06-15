use bytesize::ByteSize;
use colored::*;
use crate::scanner;
use crate::cleaners::{system, browser, trash};

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    println!("\n  {} {}\n", "🧹 Clean System".bold().cyan(), mode.yellow());

    let mut total: u64 = 0;

    // System caches
    let caches = system::cache_paths();
    let mut sys_total: u64 = 0;
    for p in &caches { sys_total += scanner::scan_size(p).0; }
    if sys_total > 0 {
        show_item("App caches", sys_total, "33");
        total += sys_total;
    }

    // Browser caches
    let browsers = browser::browser_cache_paths();
    let mut brow_total: u64 = 0;
    for (p, _) in &browsers { brow_total += scanner::scan_size(p).0; }
    if brow_total > 0 {
        show_item("Browser caches", brow_total, "35");
        total += brow_total;
    }

    // Trash
    let tp = trash::trash_path();
    if tp.exists() {
        let ts = scanner::scan_size(&tp).0;
        if ts > 0 {
            show_item("Trash", ts, "31");
            total += ts;
        }
    }

    // Summary
    println!("\n  {}", "─".repeat(40).dimmed());
    if total == 0 {
        println!("  ✨ System is clean!\n");
    } else if dry_run {
        println!("  💾 Would free: {}\n", ByteSize::b(total).to_string().bold().green());
    } else {
        // Actually delete
        for p in &caches { let _ = std::fs::remove_dir_all(p); }
        for (p, _) in &browsers { let _ = std::fs::remove_dir_all(p); }
        trash::empty_trash(false);
        println!("  🎉 Freed: {}\n", ByteSize::b(total).to_string().bold().green());
    }

    
}

fn show_item(name: &str, size: u64, color: &str) {
    let bar = "█".repeat(((size as f64 / 50_000_000_000.0) * 15.0).min(15.0) as usize);
    let empty = "░".repeat(15usize.saturating_sub(bar.len()));
    println!("  ✓ \x1b[{}m{}{}\x1b[0m {:>9}  {}",
        color, bar, empty, ByteSize::b(size).to_string().bold(), name);
}
