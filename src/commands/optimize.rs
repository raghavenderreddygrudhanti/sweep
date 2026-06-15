use bytesize::ByteSize;
use colored::*;
use std::io::{self, Write};
use crate::cleaners::optimize as opt;
use crate::scanner;

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    println!("\n  {} {}\n", "⚙  System Optimize".bold().blue(), mode.yellow());

    let mut total: u64 = 0;
    let mut actions: u32 = 0;

    // Show each action as it completes
    print!("  ✓ Flush DNS cache");
    let _ = io::stdout().flush();
    if !dry_run { opt::flush_dns(false); }
    println!();
    actions += 1;

    print!("  ✓ Rebuild Launch Services");
    let _ = io::stdout().flush();
    if !dry_run { opt::rebuild_launch_services(false); }
    println!();
    actions += 1;

    print!("  ✓ Refresh Dock & Finder");
    let _ = io::stdout().flush();
    if !dry_run { opt::refresh_dock(false); opt::refresh_finder(false); }
    println!();
    actions += 1;

    // .DS_Store
    print!("  ⏳ Scanning .DS_Store files...");
    let _ = io::stdout().flush();
    let ds = opt::clean_ds_store(dry_run);
    if ds > 0 {
        let ds_size = ds * 4096;
        total += ds_size;
        actions += 1;
        print!("\r  ✓ Removed {} .DS_Store files ({})\n", ds, ByteSize::b(ds_size));
    } else {
        print!("\r  ✓ No .DS_Store files found          \n");
    }
    let _ = io::stdout().flush();

    // Browser caches
    print!("  ⏳ Scanning browser caches...");
    let _ = io::stdout().flush();
    let browsers = crate::cleaners::browser::browser_cache_paths();
    let mut browser_total: u64 = 0;
    for (path, name) in &browsers {
        let size = scanner::scan_size(path).0;
        if size > 5_000_000 {
            browser_total += size;
            actions += 1;
            if !dry_run { let _ = std::fs::remove_dir_all(path); }
        }
    }
    if browser_total > 0 {
        total += browser_total;
        print!("\r  ✓ Browser caches ({})\x1b[K\n", ByteSize::b(browser_total).to_string().yellow());
    } else {
        print!("\r  ✓ Browser caches clean\x1b[K\n");
    }
    let _ = io::stdout().flush();

    // Summary
    println!("\n  {}", "─".repeat(40).dimmed());
    if dry_run {
        println!("  {} actions · Would free: {}\n", actions, ByteSize::b(total).to_string().bold().green());
    } else {
        println!("  {} actions · Freed: {}\n", actions, ByteSize::b(total).to_string().bold().green());
    }

    super::footer::wait_for_key();
}
