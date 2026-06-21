use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use bytesize::ByteSize;
use crossterm::{terminal, cursor, execute, event};
use crossterm::event::Event;
use std::io::{self, Write};
use crate::{scanner, cache};

/// Try to delete a file/folder. Uses direct rm (no GUI popups).
/// Moves to ~/.Trash first (recoverable), falls back to rm -rf.
/// Never prompts for password — silently fails if no permission.
fn try_delete(path: &str) -> bool {
    let p = PathBuf::from(path);
    if !p.exists() { return true; }

    let size = if p.is_dir() {
        crate::scanner::scan_size_native(&p)
    } else {
        p.metadata().map(|m| m.len()).unwrap_or(0)
    };

    // Strategy 1: Move to ~/.Trash (recoverable, no popup)
    let trash_dir = dirs::home_dir().unwrap_or_default().join(".Trash");
    let file_name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
    let mut trash_dest = trash_dir.join(&file_name);

    if trash_dest.exists() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        trash_dest = trash_dir.join(format!("{}_{}", file_name, timestamp));
    }

    let mv_result = std::process::Command::new("mv")
        .arg(path)
        .arg(&trash_dest)
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status();

    if let Ok(s) = mv_result {
        if s.success() && !p.exists() {
            crate::history::log_delete(path, size, "trash");
            return true;
        }
    }

    // Strategy 2: Direct rm -rf (works for owned files)
    let rm_result = if p.is_dir() {
        std::process::Command::new("rm")
            .args(["-rf", path])
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .status()
    } else {
        std::process::Command::new("rm")
            .args(["-f", path])
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .status()
    };

    if let Ok(s) = rm_result {
        if s.success() && !p.exists() {
            crate::history::log_delete(path, size, "delete");
            return true;
        }
    }

    // No admin password prompt — just fail silently
    false
}

struct Category {
    name: &'static str,
    path: PathBuf,
    size: i64, // -1 = pending
    cleanable: bool,
    color: &'static str,
}

fn get_categories() -> Vec<Category> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        Category { name: "Caches", path: home.join("Library/Caches"), size: -1, cleanable: true, color: "32" },
        Category { name: "App Support", path: home.join("Library/Application Support"), size: -1, cleanable: false, color: "33" },
        Category { name: "Applications", path: PathBuf::from("/Applications"), size: -1, cleanable: false, color: "36" },
        Category { name: "Downloads", path: home.join("Downloads"), size: -1, cleanable: true, color: "35" },
        Category { name: "AI/ML Models", path: home.join(".cache/huggingface"), size: -1, cleanable: true, color: "31" },
        Category { name: "Documents", path: home.join("Documents"), size: -1, cleanable: false, color: "37" },
        Category { name: "Desktop", path: home.join("Desktop"), size: -1, cleanable: false, color: "37" },
        Category { name: "Trash", path: home.join(".Trash"), size: -1, cleanable: true, color: "31" },
        Category { name: "Xcode/Dev", path: home.join("Library/Developer"), size: -1, cleanable: true, color: "32" },
        Category { name: "Docker", path: home.join("Library/Containers/com.docker.docker"), size: -1, cleanable: true, color: "34" },
        Category { name: "Pictures", path: home.join("Pictures"), size: -1, cleanable: false, color: "34" },
        Category { name: "Movies", path: home.join("Movies"), size: -1, cleanable: false, color: "34" },
    ].into_iter().filter(|c| c.path.exists()).collect()
}

pub fn run(path: &str) {
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let start = Instant::now();

    // Get free space
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let free_space = disks.list().iter()
        .find(|d| d.mount_point().to_string_lossy() == "/")
        .map(|d| d.available_space())
        .unwrap_or(0);

    // Load cached sizes
    let cached = cache::load_cached_sizes();

    // Build categories
    let mut categories = get_categories();

    // Apply cached sizes immediately
    for cat in categories.iter_mut() {
        if let Some(&size) = cached.get(cat.path.to_str().unwrap_or("")) {
            cat.size = size as i64;
        }
    }

    // Start background scanning for uncached entries
    let pending_paths: Vec<(usize, PathBuf)> = categories.iter().enumerate()
        .filter(|(_, c)| c.size < 0)
        .map(|(i, c)| (i, c.path.clone()))
        .collect();

    let results: Arc<Mutex<HashMap<usize, u64>>> = Arc::new(Mutex::new(HashMap::new()));

    // Spawn background threads for each pending scan
    let handles: Vec<_> = pending_paths.iter().map(|(idx, path)| {
        let path = path.clone();
        let idx = *idx;
        let results = Arc::clone(&results);
        thread::spawn(move || {
            let size = scanner::scan_size(&path).0;
            results.lock().unwrap().insert(idx, size);
        })
    }).collect();

    let mut selected: usize = 0;
    let mut mode = "overview";
    let mut folder_results: Vec<scanner::ScanResult> = vec![];
    let mut current_path = PathBuf::new();
    let mut multi_selected: HashMap<String, bool> = HashMap::new();
    let mut status_msg: String = String::new();
    let mut confirm_delete: bool = false;
    let mut spin_frame: usize = 0;

    loop {
        // Check for completed background scans
        {
            let mut res = results.lock().unwrap();
            for (&idx, &size) in res.iter() {
                if idx < categories.len() {
                    categories[idx].size = size as i64;
                    // Save to cache
                    cache::save_size(
                        categories[idx].path.to_str().unwrap_or(""),
                        size
                    );
                }
            }
            res.clear();
        }

        // Render
        let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All));
        let elapsed = start.elapsed().as_secs_f64();

        let mut out = String::new();
        let scanning = categories.iter().any(|c| c.size < 0);
        if scanning {
            out.push_str(&super::ui::tui_header_animated(super::ui::action_name("scan"), spin_frame));
        } else {
            out.push_str(&super::ui::tui_header("Analyze Disk"));
        }

        let scan_indicator = if scanning {
            spin_frame = (spin_frame + 1) % 10;
            format!(" {} ", super::ui::spinner(spin_frame))
        } else { " \x1b[32m✓\x1b[0m ".to_string() };

        out.push_str(&format!("  \x1b[90m{} free\x1b[0m{}\r\n\r\n",
            ByteSize::b(free_space), scan_indicator));

        if mode == "overview" {
            // Separate ready vs pending entries
            let mut ready: Vec<(usize, &Category)> = categories.iter().enumerate()
                .filter(|(_, c)| c.size > 0)  // Only show items with actual size > 0
                .collect();
            let pending: Vec<(usize, &Category)> = categories.iter().enumerate()
                .filter(|(_, c)| c.size < 0)  // Still scanning
                .collect();

            // Sort ready by size descending
            ready.sort_by(|a, b| b.1.size.cmp(&a.1.size));

            let max_size = ready.first().map(|(_, c)| c.size as u64).unwrap_or(1);

            // Show ready items first
            for (display_idx, (orig_idx, cat)) in ready.iter().enumerate() {
                let size = cat.size as u64;
                let bar_len = if max_size > 0 {
                    let calc = (size as f64 / max_size as f64 * 15.0) as usize;
                    calc.max(1) // Always show at least 1 block of color
                } else { 1 };
                let bar = format!("\x1b[{}m{}\x1b[0m\x1b[90m{}\x1b[0m",
                    cat.color,
                    "█".repeat(bar_len),
                    "░".repeat(15usize.saturating_sub(bar_len)));

                let ptr = if display_idx == selected { " \x1b[32m▶\x1b[0m" } else { "  " };
                let cleanable = if cat.cleanable { "\x1b[33m··\x1b[0m" } else { "  " };
                let name = if display_idx == selected {
                    format!("\x1b[1;36m{}\x1b[0m", cat.name)
                } else {
                    cat.name.to_string()
                };

                out.push_str(&format!("{} {} {} \x1b[1m{:>9}\x1b[0m  📁 {}\r\n",
                    ptr, bar, cleanable, ByteSize::b(size).to_string(), name));
            }

            // Show pending items at the bottom (if any)
            if !pending.is_empty() {
                out.push_str(&format!("\r\n  \x1b[90m  ... scanning {} more\x1b[0m\r\n", pending.len()));
            }
        } else {
            // Folder view
            out.push_str(&format!("  📂 \x1b[36m{}\x1b[0m\r\n\r\n", current_path.display()));
            let max_size = folder_results.first().map(|r| r.size).unwrap_or(1);

            for (i, result) in folder_results.iter().take(15).enumerate() {
                let name = result.path
                    .strip_prefix(current_path.to_str().unwrap_or(""))
                    .unwrap_or(&result.path)
                    .trim_start_matches('/');
                let name = if name.len() > 25 { &name[..25] } else { name };

                let bar_len = (result.size as f64 / max_size as f64 * 12.0).max(1.0) as usize;
                let bar = format!("█{}░{}", "█".repeat(bar_len.saturating_sub(1)), "░".repeat(12usize.saturating_sub(bar_len)));
                let ptr = if i == selected { " \x1b[32m▶\x1b[0m" } else { "  " };
                let icon = if result.is_dir { "📁" } else { "📄" };
                let is_multi = multi_selected.contains_key(&result.path);
                let sel_icon = if is_multi { "\x1b[32m●\x1b[0m" } else { " " };
                let name_fmt = if i == selected {
                    format!("\x1b[1;36m{}\x1b[0m", name)
                } else if is_multi {
                    format!("\x1b[32m{}\x1b[0m", name)
                } else {
                    name.to_string()
                };

                out.push_str(&format!("{}{} \x1b[32m{}\x1b[0m {:>9}  {} {}\r\n",
                    ptr, sel_icon, bar, ByteSize::b(result.size).to_string(), icon, name_fmt));
            }
        }

        // Footer
        out.push_str(super::ui::footer_sep());
        if !status_msg.is_empty() {
            out.push_str(&format!("  \x1b[33m{}\x1b[0m\r\n", status_msg));
        }
        if confirm_delete {
            out.push_str("  \x1b[1;31m⚠ Delete this item? (y)es / (n)o\x1b[0m\r\n");
        } else {
            let multi_count = multi_selected.len();
            if multi_count > 0 {
                out.push_str(&super::ui::footer_selected(multi_count));
            } else {
                out.push_str(super::ui::footer_browse());
            }
        }

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        // Input with timeout (so we can update scanning results)
        if event::poll(std::time::Duration::from_millis(300)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                let item_count = if mode == "overview" {
                    categories.iter().filter(|c| c.size > 0).count()
                } else {
                    folder_results.len().min(15)
                };

                // Handle delete confirmation first
                if confirm_delete {
                    let action = super::ui::map_key(key);
                    match action {
                        super::ui::NavAction::Char('y') | super::ui::NavAction::Char('Y') => {
                            if mode == "folder" && selected < folder_results.len() {
                                let target = &folder_results[selected].path.clone();
                                let deleted = try_delete(target);
                                if deleted {
                                    status_msg = format!("\x1b[32m\u{2713}\x1b[0m Deleted: {}", PathBuf::from(target).file_name().unwrap_or_default().to_string_lossy());
                                    folder_results = scanner::scan_children(&current_path);
                                    folder_results.sort_by(|a, b| b.size.cmp(&a.size));
                                    if selected >= folder_results.len() && selected > 0 { selected -= 1; }
                                } else {
                                    status_msg = format!("\x1b[33m\u{26a0}\x1b[0m Cannot delete (protected by macOS). Skipped.");
                                }
                            }
                            confirm_delete = false;
                        }
                        _ => { confirm_delete = false; status_msg.clear(); }
                    }
                    continue;
                }

                let action = super::ui::map_key(key);
                match action {
                    super::ui::NavAction::Up => {
                        if selected > 0 { selected -= 1; }
                        status_msg.clear();
                    }
                    super::ui::NavAction::Down => {
                        if selected < item_count.saturating_sub(1) { selected += 1; }
                        status_msg.clear();
                    }
                    super::ui::NavAction::Toggle => {
                        if mode == "folder" && selected < folder_results.len() {
                            let path = folder_results[selected].path.clone();
                            if multi_selected.contains_key(&path) {
                                multi_selected.remove(&path);
                            } else {
                                multi_selected.insert(path, true);
                            }
                        }
                    }
                    super::ui::NavAction::ClearAll => {
                        multi_selected.clear();
                    }
                    super::ui::NavAction::Select => {
                        if mode == "overview" {
                            let ready: Vec<(usize, &Category)> = categories.iter().enumerate()
                                .filter(|(_, c)| c.size > 0)
                                .collect();
                            let mut sorted_ready = ready;
                            sorted_ready.sort_by(|a, b| b.1.size.cmp(&a.1.size));

                            if selected < sorted_ready.len() {
                                current_path = sorted_ready[selected].1.path.clone();
                                folder_results = scanner::scan_children(&current_path);
                                folder_results.sort_by(|a, b| b.size.cmp(&a.size));
                                mode = "folder";
                                selected = 0;
                                multi_selected.clear();
                                status_msg.clear();
                            }
                        } else if mode == "folder" && selected < folder_results.len() && folder_results[selected].is_dir {
                            current_path = PathBuf::from(&folder_results[selected].path);
                            folder_results = scanner::scan_children(&current_path);
                            folder_results.sort_by(|a, b| b.size.cmp(&a.size));
                            selected = 0;
                            multi_selected.clear();
                            status_msg.clear();
                        }
                    }
                    super::ui::NavAction::Back => {
                        if mode == "folder" {
                            if let Some(parent) = current_path.parent() {
                                let parent_buf = parent.to_path_buf();
                                current_path = parent_buf;
                                folder_results = scanner::scan_children(&current_path);
                                folder_results.sort_by(|a, b| b.size.cmp(&a.size));
                                selected = 0;
                                multi_selected.clear();
                            }
                            let is_root = categories.iter().filter(|c| c.size > 0).any(|c| c.path == current_path)
                                || current_path == dirs::home_dir().unwrap_or_default()
                                || current_path == PathBuf::from("/");
                            if is_root {
                                mode = "overview";
                                selected = 0;
                            }
                        } else {
                            // In overview — Esc goes back to main menu
                            break;
                        }
                        status_msg.clear();
                    }
                    super::ui::NavAction::Quit => {
                        // q — always quit back to main menu
                        break;
                    }
                    super::ui::NavAction::Delete => {
                        if mode == "folder" {
                            if !multi_selected.is_empty() {
                                let count = multi_selected.len();
                                let mut ok = 0;
                                for path in multi_selected.keys() {
                                    if try_delete(path) { ok += 1; }
                                }
                                status_msg = format!("✓ Deleted {}/{} items", ok, count);
                                multi_selected.clear();
                                folder_results = scanner::scan_children(&current_path);
                                folder_results.sort_by(|a, b| b.size.cmp(&a.size));
                                if selected >= folder_results.len() && selected > 0 { selected -= 1; }
                            } else if selected < folder_results.len() {
                                confirm_delete = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();
}
