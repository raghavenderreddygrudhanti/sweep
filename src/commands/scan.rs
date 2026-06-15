use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use bytesize::ByteSize;
use crossterm::{terminal, cursor, execute, event};
use crossterm::event::{Event, KeyCode};
use std::io::{self, Write};
use crate::{scanner, cache};

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
    let spinners = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];
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
        out.push_str("\r\n");

        let scanning = categories.iter().any(|c| c.size < 0);
        let scan_indicator = if scanning {
            spin_frame = (spin_frame + 1) % spinners.len();
            format!(" {}", spinners[spin_frame])
        } else { " ✓".to_string() };

        out.push_str(&format!("  \x1b[1;36mAnalyze Disk\x1b[0m  ({} free){}\r\n",
            ByteSize::b(free_space), scan_indicator));
        out.push_str("  \x1b[90m─────────────────────────────────────────────\x1b[0m\r\n\r\n");

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
        out.push_str("\r\n  \x1b[90m─────────────────────────────────────────────\x1b[0m\r\n");
        if !status_msg.is_empty() {
            out.push_str(&format!("  \x1b[33m{}\x1b[0m\r\n", status_msg));
        }
        if confirm_delete {
            out.push_str("  \x1b[1;31m⚠ Delete this item? (y)es / (n)o\x1b[0m\r\n");
        } else {
            let multi_count = multi_selected.len();
            if multi_count > 0 {
                out.push_str(&format!("  \x1b[32m{} selected\x1b[0m — ", multi_count));
                out.push_str("\x1b[90mD delete selected · Space toggle · n clear\x1b[0m\r\n");
            } else {
                out.push_str("  \x1b[90m↑↓ nav · →Enter open · ←Back · Space select · d del · q quit\x1b[0m\r\n");
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
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if mode == "folder" && selected < folder_results.len() {
                                let target = &folder_results[selected].path.clone();
                                let _ = std::process::Command::new("osascript")
                                    .args(["-e", &format!(
                                        "tell application \"Finder\" to delete POSIX file \"{}\"", target
                                    )]).output();
                                status_msg = format!("Moved to Trash: {}", PathBuf::from(target).file_name().unwrap_or_default().to_string_lossy());
                                folder_results = scanner::scan_children(&current_path);
                                folder_results.sort_by(|a, b| b.size.cmp(&a.size));
                                if selected >= folder_results.len() && selected > 0 { selected -= 1; }
                            }
                            confirm_delete = false;
                        }
                        _ => { confirm_delete = false; status_msg.clear(); }
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected > 0 { selected -= 1; }
                        status_msg.clear();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected < item_count.saturating_sub(1) { selected += 1; }
                        status_msg.clear();
                    }
                    KeyCode::Char(' ') => {
                        // Toggle multi-select
                        if mode == "folder" && selected < folder_results.len() {
                            let path = folder_results[selected].path.clone();
                            if multi_selected.contains_key(&path) {
                                multi_selected.remove(&path);
                            } else {
                                multi_selected.insert(path, true);
                            }
                        }
                    }
                    KeyCode::Char('n') => {
                        multi_selected.clear();
                    }
                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                        if mode == "overview" {
                            // Get the displayed ready list to find the right path
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
                    KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') | KeyCode::Char('b') => {
                        if mode == "folder" {
                            if let Some(parent) = current_path.parent() {
                                let parent_buf = parent.to_path_buf();
                                current_path = parent_buf;
                                folder_results = scanner::scan_children(&current_path);
                                folder_results.sort_by(|a, b| b.size.cmp(&a.size));
                                selected = 0;
                                multi_selected.clear();
                            }
                            let is_root = categories.iter().any(|c| c.path == current_path)
                                || current_path == dirs::home_dir().unwrap_or_default()
                                || current_path == PathBuf::from("/");
                            if is_root {
                                mode = "overview";
                                selected = 0;
                            }
                        }
                        status_msg.clear();
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                        if mode == "folder" {
                            if !multi_selected.is_empty() {
                                // Delete all selected
                                for path in multi_selected.keys() {
                                    let _ = std::process::Command::new("osascript")
                                        .args(["-e", &format!(
                                            "tell application \"Finder\" to delete POSIX file \"{}\"", path
                                        )]).output();
                                }
                                status_msg = format!("Moved {} items to Trash", multi_selected.len());
                                multi_selected.clear();
                                folder_results = scanner::scan_children(&current_path);
                                folder_results.sort_by(|a, b| b.size.cmp(&a.size));
                                if selected >= folder_results.len() && selected > 0 { selected -= 1; }
                            } else if selected < folder_results.len() {
                                confirm_delete = true;
                            }
                        }
                    }
                    KeyCode::Char('q') => break,
                    KeyCode::Esc => {
                        // Esc goes back, only q quits
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
                            // Already in overview, Esc quits
                            break;
                        }
                        status_msg.clear();
                    }
                    _ => {}
                }
            }
        }
    }

    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();
}
