//! System optimization — DNS flush, Spotlight rebuild, maintenance scripts.

use std::process::Command;

/// Run a system command and report success.
fn run_cmd(label: &str, cmd: &str, args: &[&str], dry_run: bool) -> bool {
    if dry_run {
        println!("  Would run: {} {}", cmd, args.join(" "));
        return true;
    }

    let result = Command::new(cmd).args(args).output();
    match result {
        Ok(o) if o.status.success() => {
            println!("  ✓ {}", label);
            true
        }
        Ok(_) => {
            println!("  ⚠ {} (failed)", label);
            false
        }
        Err(_) => {
            println!("  ⚠ {} (command not found)", label);
            false
        }
    }
}

/// Flush DNS cache.
pub fn flush_dns(dry_run: bool) -> bool {
    run_cmd(
        "Flush DNS cache",
        "dscacheutil",
        &["-flushcache"],
        dry_run,
    )
}

/// Rebuild Spotlight index.
pub fn rebuild_spotlight(dry_run: bool) -> bool {
    run_cmd(
        "Rebuild Spotlight index",
        "mdutil",
        &["-E", "/"],
        dry_run,
    )
}

/// Rebuild Launch Services database.
pub fn rebuild_launch_services(dry_run: bool) -> bool {
    run_cmd(
        "Rebuild Launch Services",
        "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister",
        &["-kill", "-r", "-domain", "local", "-domain", "system", "-domain", "user"],
        dry_run,
    )
}

/// Refresh Dock.
pub fn refresh_dock(dry_run: bool) -> bool {
    run_cmd("Refresh Dock", "killall", &["Dock"], dry_run)
}

/// Refresh Finder.
pub fn refresh_finder(dry_run: bool) -> bool {
    run_cmd("Refresh Finder", "killall", &["Finder"], dry_run)
}

/// Run periodic maintenance scripts.
pub fn run_periodic_maintenance(dry_run: bool) -> bool {
    run_cmd(
        "Run periodic maintenance (daily, weekly, monthly)",
        "sudo",
        &["periodic", "daily", "weekly", "monthly"],
        dry_run,
    )
}

/// Purge inactive memory.
pub fn purge_memory(dry_run: bool) -> bool {
    run_cmd("Purge inactive memory", "sudo", &["purge"], dry_run)
}

/// Remove .DS_Store files recursively from home.
pub fn clean_ds_store(dry_run: bool) -> u64 {
    let home = dirs::home_dir().unwrap_or_default();
    let mut count = 0u64;

    for entry in walkdir::WalkDir::new(&home)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == ".DS_Store" {
            if !dry_run {
                let _ = std::fs::remove_file(entry.path());
            }
            count += 1;
        }
    }

    count
}

/// Find and remove installer files (.dmg, .pkg).
pub fn find_installers() -> Vec<(std::path::PathBuf, u64)> {
    let home = dirs::home_dir().unwrap_or_default();
    let search_dirs = vec![
        home.join("Downloads"),
        home.join("Desktop"),
    ];

    let extensions = ["dmg", "pkg", "iso", "app.zip"];
    let mut found = vec![];

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext) {
                        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                        if size > 10_000_000 {
                            // > 10MB
                            found.push((path, size));
                        }
                    }
                }
            }
        }
    }

    found.sort_by(|a, b| b.1.cmp(&a.1));
    found
}
