use bytesize::ByteSize;
use colored::*;
use crate::cleaners::optimize as opt;
use crate::scanner;

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    println!("\n  {} {}\n", "⚙  System Optimize".bold().blue(), mode.yellow());

    let mut total: u64 = 0;
    let mut actions: u32 = 0;

    // Actions
    let items: Vec<(&str, bool)> = vec![
        ("Flush DNS cache", true),
        ("Rebuild Launch Services", true),
        ("Refresh Dock & Finder", true),
    ];

    for (name, _) in &items {
        println!("  {} {}", "✓".green(), name);
        actions += 1;
    }

    if !dry_run {
        opt::flush_dns(false);
        opt::rebuild_launch_services(false);
        opt::refresh_dock(false);
        opt::refresh_finder(false);
    }

    // .DS_Store
    let ds = opt::clean_ds_store(dry_run);
    if ds > 0 {
        let ds_size = ds * 4096;
        total += ds_size;
        actions += 1;
        println!("  {} Removed {} .DS_Store files ({})",
            "✓".green(), ds, ByteSize::b(ds_size));
    }

    // Browser caches
    let browsers = crate::cleaners::browser::browser_cache_paths();
    for (path, name) in &browsers {
        let size = scanner::scan_size(path).0;
        if size > 5_000_000 {
            println!("  {} {} ({})", "✓".green(), name, ByteSize::b(size).to_string().yellow());
            total += size;
            actions += 1;
            if !dry_run { let _ = std::fs::remove_dir_all(path); }
        }
    }

    // Summary
    println!("\n  {}", "─".repeat(40).dimmed());
    if dry_run {
        println!("  {} actions · Would free: {}\n", actions, ByteSize::b(total).to_string().bold().green());
    } else {
        println!("  {} actions · Freed: {}\n", actions, ByteSize::b(total).to_string().bold().green());
    }

    super::footer::wait_for_key();
}
