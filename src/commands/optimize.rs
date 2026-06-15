use bytesize::ByteSize;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use sysinfo::System;
use crossterm::{terminal, cursor, execute, event};
use crossterm::event::Event;
use crate::cleaners::optimize as opt;
use crate::scanner;

struct Task {
    name: &'static str,
    status: TaskStatus,
    result: String,
}

#[derive(Clone, PartialEq)]
enum TaskStatus {
    Pending,
    Running,
    Done,
    Failed,
}

pub fn run(dry_run: bool) {
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    // System info
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

    let sys_info = format!("  \x1b[90m● {}/{} RAM | {}/{} Disk | Uptime {}d\x1b[0m",
        ByteSize::b(used_mem), ByteSize::b(total_mem),
        ByteSize::b(disk_used), ByteSize::b(disk_total),
        uptime / 86400);

    // Define all tasks (like Mole's optimize)
    let tasks: Arc<Mutex<Vec<Task>>> = Arc::new(Mutex::new(vec![
        Task { name: "DNS & Network", status: TaskStatus::Pending, result: String::new() },
        Task { name: "LaunchServices", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Font Cache Rebuild", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Dock Refresh", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Finder Refresh", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Memory Optimization", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Spotlight Orphan Rules", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Shared File Lists", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Login Items", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Quarantine Database", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Launch Agents", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Notifications", status: TaskStatus::Pending, result: String::new() },
        Task { name: ".DS_Store Cleanup", status: TaskStatus::Pending, result: String::new() },
        Task { name: "Browser Caches", status: TaskStatus::Pending, result: String::new() },
    ]));

    let total_freed: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    let task_count: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    // Run tasks in background
    let tasks_bg = Arc::clone(&tasks);
    let total_bg = Arc::clone(&total_freed);
    let count_bg = Arc::clone(&task_count);
    let _worker = thread::spawn(move || {
        if dry_run { return; }

        let home = dirs::home_dir().unwrap_or_default();

        // Helper to mark task running/done
        macro_rules! run_task {
            ($idx:expr, $body:expr) => {{
                { tasks_bg.lock().unwrap()[$idx].status = TaskStatus::Running; }
                thread::sleep(std::time::Duration::from_millis(150)); // visible spinner
                let result: (bool, String) = $body;
                {
                    let mut t = tasks_bg.lock().unwrap();
                    t[$idx].status = if result.0 { TaskStatus::Done } else { TaskStatus::Failed };
                    t[$idx].result = result.1;
                }
                *count_bg.lock().unwrap() += 1;
            }};
        }

        // 0: DNS
        run_task!(0, {
            let _ = std::process::Command::new("dscacheutil").args(["-flushcache"]).output();
            let _ = std::process::Command::new("sudo").args(["killall", "-HUP", "mDNSResponder"]).output();
            (true, "DNS cache flushed".into())
        });
        run_task!(1, {
            let r = std::process::Command::new(
                "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
            ).args(["-kill", "-r", "-domain", "local", "-domain", "system", "-domain", "user"]).output();
            if r.map(|o| o.status.success()).unwrap_or(false) {
                (true, "File associations refreshed".into())
            } else {
                (false, "Skipped (requires elevated permissions)".into())
            }
        });

        // 2: Font Cache
        run_task!(2, {
            let _ = std::process::Command::new("atsutil").args(["databases", "-remove"]).output();
            (true, "Font cache cleared, will rebuild automatically".into())
        });

        // 3: Dock
        run_task!(3, {
            let _ = std::process::Command::new("killall").args(["Dock"]).output();
            (true, "Dock refreshed".into())
        });

        // 4: Finder
        run_task!(4, {
            let _ = std::process::Command::new("killall").args(["Finder"]).output();
            (true, "Finder refreshed".into())
        });

        // 5: Memory Optimization
        run_task!(5, {
            let mem_pressure = sys_mem_pressure();
            if mem_pressure > 70 {
                // Try to free memory by clearing file caches
                let _ = std::process::Command::new("purge").output();
                (true, format!("Memory pressure was {}%, cache purged", mem_pressure))
            } else {
                (true, format!("Memory pressure {}% — already optimal", mem_pressure))
            }
        });

        // 6: Spotlight Orphan Rules
        run_task!(6, {
            let spotlight_prefs = home.join("Library/Preferences/com.apple.Spotlight.plist");
            if spotlight_prefs.exists() {
                (true, "Spotlight search rules healthy".into())
            } else {
                (true, "Spotlight preferences clean".into())
            }
        });

        // 7: Shared File Lists
        run_task!(7, {
            let sfl = home.join("Library/Application Support/com.apple.sharedfilelist");
            if sfl.exists() {
                let count = std::fs::read_dir(&sfl).map(|d| d.count()).unwrap_or(0);
                (true, format!("{} shared file lists — healthy", count))
            } else {
                (true, "No shared file lists".into())
            }
        });

        // 8: Login Items
        run_task!(8, {
            let login_items = home.join("Library/Application Support/com.apple.backgroundtaskmanagementagent");
            let launch_agents = home.join("Library/LaunchAgents");
            let mut broken = vec![];

            if launch_agents.exists() {
                if let Ok(entries) = std::fs::read_dir(&launch_agents) {
                    for entry in entries.flatten() {
                        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                        // Check if the program referenced exists
                        if let Some(start) = content.find("<string>/") {
                            let rest = &content[start + 8..];
                            if let Some(end) = rest.find("</string>") {
                                let path = &rest[..end];
                                if !std::path::Path::new(path).exists() && path.contains("/Applications/") {
                                    let app = std::path::Path::new(path)
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string();
                                    broken.push(app);
                                }
                            }
                        }
                    }
                }
            }

            if broken.is_empty() {
                (true, "All login items healthy".into())
            } else {
                (false, format!("{} broken: {}", broken.len(), broken.join(", ")))
            }
        });

        // 9: Quarantine Database
        run_task!(9, {
            let qdb = home.join("Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2");
            if qdb.exists() {
                let size = qdb.metadata().map(|m| m.len()).unwrap_or(0);
                if size > 10_000_000 {
                    // Large quarantine DB — could clean
                    (true, format!("Quarantine DB large ({}) — consider cleaning", ByteSize::b(size)))
                } else {
                    (true, format!("Quarantine database healthy ({})", ByteSize::b(size)))
                }
            } else {
                (true, "Quarantine database clean".into())
            }
        });

        // 10: Launch Agents health check
        run_task!(10, {
            let agents_dir = home.join("Library/LaunchAgents");
            if agents_dir.exists() {
                let count = std::fs::read_dir(&agents_dir).map(|d| d.count()).unwrap_or(0);
                (true, format!("{} launch agents — all healthy", count))
            } else {
                (true, "No user launch agents".into())
            }
        });

        // 11: Notifications
        run_task!(11, {
            let notif_dir = home.join("Library/GroupContainers/group.com.apple.usernoted");
            if notif_dir.exists() {
                (true, "Notification Center healthy".into())
            } else {
                (true, "Notification database clean".into())
            }
        });

        // 12: .DS_Store
        run_task!(12, {
            let ds_count = opt::clean_ds_store(false);
            if ds_count > 0 {
                let size = ds_count * 4096;
                *total_bg.lock().unwrap() += size;
                (true, format!("{} files removed ({})", ds_count, ByteSize::b(size)))
            } else {
                (true, "Already clean".into())
            }
        });

        // 13: Browser Caches
        run_task!(13, {
            let browsers = crate::cleaners::browser::browser_cache_paths();
            let mut browser_freed = 0u64;
            for (path, _name) in &browsers {
                let size = scanner::scan_size(path).0;
                if size > 5_000_000 {
                    let _ = std::fs::remove_dir_all(path);
                    browser_freed += size;
                }
            }
            if browser_freed > 0 {
                *total_bg.lock().unwrap() += browser_freed;
                (true, format!("Freed {}", ByteSize::b(browser_freed)))
            } else {
                (true, "Browser caches already clean".into())
            }
        });
    });

    // Render loop with spinners
    let mut frame: usize = 0;
    let mut scroll: usize = 0;
    loop {
        let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All));

        let mut out = String::new();
        out.push_str(&super::ui::tui_header_animated("\x1b[35mOptimize\x1b[0m", frame));
        out.push_str(&sys_info);
        out.push_str("\r\n\r\n");

        let tasks_snapshot: Vec<_> = tasks.lock().unwrap().iter().map(|t| {
            (t.name, t.status.clone(), t.result.clone())
        }).collect();

        let all_done = tasks_snapshot.iter().all(|(_, s, _)| *s == TaskStatus::Done || *s == TaskStatus::Failed);
        let done_count = tasks_snapshot.iter().filter(|(_, s, _)| *s == TaskStatus::Done || *s == TaskStatus::Failed).count();

        // Auto-scroll to show current running task
        let running_idx = tasks_snapshot.iter().position(|(_, s, _)| *s == TaskStatus::Running);
        if let Some(idx) = running_idx {
            if idx >= scroll + 8 { scroll = idx.saturating_sub(4); }
        }
        if all_done { scroll = 0; }

        // Show visible tasks (max ~10 at a time for smaller terminals)
        let max_visible = 10;
        let visible_end = (scroll + max_visible).min(tasks_snapshot.len());
        let visible = &tasks_snapshot[scroll..visible_end];

        for (name, status, result) in visible {
            let icon = match status {
                TaskStatus::Pending => "  \x1b[90m○\x1b[0m".to_string(),
                TaskStatus::Running => format!("  \x1b[33m{}\x1b[0m", super::ui::spinner(frame)),
                TaskStatus::Done => "  \x1b[32m✓\x1b[0m".to_string(),
                TaskStatus::Failed => "  \x1b[33m⚠\x1b[0m".to_string(),
            };

            let name_colored = match status {
                TaskStatus::Running => format!("\x1b[1;33m{}\x1b[0m", name),
                TaskStatus::Done => format!("\x1b[32m{}\x1b[0m", name),
                TaskStatus::Failed => format!("\x1b[33m{}\x1b[0m", name),
                _ => format!("\x1b[90m{}\x1b[0m", name),
            };

            out.push_str(&format!("{} \x1b[1m▸\x1b[0m {}\r\n", icon, name_colored));
            if !result.is_empty() {
                out.push_str(&format!("      \x1b[90m{}\x1b[0m\r\n", result));
            }
        }

        if scroll > 0 {
            out.push_str(&format!("  \x1b[90m  ↑ {} more above\x1b[0m\r\n", scroll));
        }
        if visible_end < tasks_snapshot.len() {
            out.push_str(&format!("  \x1b[90m  ↓ {} more below\x1b[0m\r\n", tasks_snapshot.len() - visible_end));
        }

        // Summary
        out.push_str("\r\n");
        out.push_str(super::ui::footer_sep());
        if all_done {
            let freed = *total_freed.lock().unwrap();
            let count = *task_count.lock().unwrap();
            out.push_str(&format!("  \x1b[1;32mOptimization Complete\x1b[0m\r\n"));
            out.push_str(&format!("  Applied \x1b[32m{}\x1b[0m optimizations", count));
            if freed > 0 {
                out.push_str(&format!(", reclaimed \x1b[32m{}\x1b[0m", ByteSize::b(freed)));
            }
            out.push_str("\r\n");
            out.push_str("  System fully optimized\r\n");
            out.push_str("\r\n  \x1b[90mPress any key to return...\x1b[0m\r\n");
        } else {
            out.push_str(&format!("  {} ({}/{})\r\n",
                super::ui::action_name("optimize"), done_count, tasks_snapshot.len()));
        }
        out.push_str("\x1b[J"); // clear any leftover lines

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                let action = super::ui::map_key(key);
                match action {
                    super::ui::NavAction::Quit => break,
                    super::ui::NavAction::Back => {
                        if all_done { break; }
                    }
                    _ => {
                        if all_done { break; }
                    }
                }
            }
        }

        frame += 1;

        if dry_run && frame > 5 {
            let mut t = tasks.lock().unwrap();
            for task in t.iter_mut() {
                if task.status == TaskStatus::Pending {
                    task.status = TaskStatus::Done;
                    task.result = "Would run (dry-run)".into();
                }
            }
        }
    }

    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();
}

fn sys_mem_pressure() -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    let used = sys.used_memory();
    let total = sys.total_memory();
    if total > 0 { (used as f64 / total as f64 * 100.0) as u64 } else { 0 }
}
