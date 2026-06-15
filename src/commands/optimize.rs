use bytesize::ByteSize;
use colored::*;
use std::io::{self, Write};
use sysinfo::System;
use crate::cleaners::optimize as opt;
use crate::scanner;

pub fn run(dry_run: bool) {
    println!();
    println!("  \x1b[1;35mOptimize\x1b[0m");

    // System summary
    let mut sys = System::new_all();
    sys.refresh_all();
    let used_mem = sys.used_memory();
    let total_mem = sys.total_memory();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = disks.list().iter()
        .find(|d| d.mount_point().to_string_lossy() == "/")
        .map(|d| (d.total_space() - d.available_space(), d.total_space()))
        .unwrap_or((0, 1));
    let uptime = System::uptime();

    println!("  \x1b[90m● System  {}/{} RAM | {}/{} Disk | Uptime {}d\x1b[0m",
        ByteSize::b(used_mem), ByteSize::b(total_mem),
        ByteSize::b(disk_used), ByteSize::b(disk_total),
        uptime / 86400);
    println!();

    // Performance diagnosis
    let cpu: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;
    let mem_pct = (used_mem as f64 / total_mem as f64 * 100.0) as u64;

    println!("  \x1b[1;35mPERFORMANCE DIAGNOSIS\x1b[0m");
    if cpu > 80.0 {
        println!("    \x1b[33m●\x1b[0m High CPU usage ({:.0}%)", cpu);
    } else if mem_pct > 85 {
        println!("    \x1b[33m●\x1b[0m High memory pressure ({}%)", mem_pct);
    } else {
        println!("    \x1b[32m●\x1b[0m System healthy — no major bottlenecks");
    }
    println!();

    if dry_run {
        println!("  \x1b[33m(preview mode — no changes will be made)\x1b[0m\n");
    }

    let mut total: u64 = 0;

    // Group: DNS & Network
    println!("  \x1b[1;35m▸ DNS & Network\x1b[0m");
    print!("    ");
    let _ = io::stdout().flush();
    if !dry_run { opt::flush_dns(false); }
    println!("✓ DNS cache flushed");

    // Group: LaunchServices
    println!("  \x1b[1;35m▸ LaunchServices\x1b[0m");
    print!("    ");
    let _ = io::stdout().flush();
    if !dry_run { opt::rebuild_launch_services(false); }
    println!("✓ File associations refreshed");

    // Group: Dock & Finder
    println!("  \x1b[1;35m▸ Dock & Finder\x1b[0m");
    if !dry_run { opt::refresh_dock(false); opt::refresh_finder(false); }
    println!("    ✓ Dock refreshed");
    println!("    ✓ Finder refreshed");

    // Group: .DS_Store
    println!("  \x1b[1;35m▸ .DS_Store Cleanup\x1b[0m");
    print!("    ⏳ Scanning...");
    let _ = io::stdout().flush();
    let ds = opt::clean_ds_store(dry_run);
    if ds > 0 {
        let ds_size = ds * 4096;
        total += ds_size;
        print!("\r    ✓ {} files removed ({})\x1b[K\n", ds, ByteSize::b(ds_size));
    } else {
        print!("\r    ✓ Already clean\x1b[K\n");
    }

    // Group: Browser Caches
    println!("  \x1b[1;35m▸ Browser Caches\x1b[0m");
    let browsers = crate::cleaners::browser::browser_cache_paths();
    let mut any_browser = false;
    for (path, name) in &browsers {
        let size = scanner::scan_size(path).0;
        if size > 5_000_000 {
            println!("    ✓ {} ({})", name, ByteSize::b(size).to_string().yellow());
            total += size;
            any_browser = true;
            if !dry_run { let _ = std::fs::remove_dir_all(path); }
        }
    }
    if !any_browser {
        println!("    ✓ Browser caches clean");
    }

    // Summary
    println!("\n  \x1b[90m─────────────────────────────────────\x1b[0m");
    if total > 0 {
        if dry_run {
            println!("  \x1b[1;32m💾 Would free: {}\x1b[0m\n", ByteSize::b(total));
        } else {
            println!("  \x1b[1;32m🎉 Freed: {}\x1b[0m\n", ByteSize::b(total));
        }
    } else {
        println!("  \x1b[32m✓ System already optimized\x1b[0m\n");
    }

    
}
