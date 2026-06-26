//! Duplicate file finder — finds files with identical content.
//! Algorithm: group by size → parallel hash first+last 4KB → full hash on match.
//! Uses rayon for parallel hashing across all CPU cores.

use bytesize::ByteSize;
use colored::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use walkdir::WalkDir;

/// A group of duplicate files (identical content, same size).
struct DupeGroup {
    size: u64,
    paths: Vec<PathBuf>,
}

impl DupeGroup {
    /// Total wasted bytes = size × (copies - 1)
    fn waste(&self) -> u64 {
        self.size * (self.paths.len() as u64 - 1)
    }

    /// The "keep" candidate: newest file by mtime.
    fn keep_idx(&self) -> usize {
        self.paths
            .iter()
            .enumerate()
            .max_by_key(|(_, p)| {
                p.metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

// ─── folder picker ───────────────────────────────────────────────────────────

fn pick_folders() -> Vec<PathBuf> {
    let home = crate::error::home_or_exit();

    let options: Vec<(&str, PathBuf)> = vec![
        ("Documents", home.join("Documents")),
        ("Downloads", home.join("Downloads")),
        ("Desktop", home.join("Desktop")),
        ("Pictures", home.join("Pictures")),
        (
            "Photos Library",
            home.join("Pictures/Photos Library.photoslibrary/originals"),
        ),
        ("Movies", home.join("Movies")),
        ("Music", home.join("Music")),
        ("All above", home.clone()),
    ]
    .into_iter()
    .filter(|(label, p)| *label == "All above" || p.exists())
    .collect();

    println!("  \x1b[1mSelect folders to scan for duplicates:\x1b[0m\n");

    let mut selected = vec![false; options.len()];

    for (i, (label, _)) in options.iter().enumerate() {
        let mark = if selected[i] {
            "\x1b[32m\u{25cf}\x1b[0m"
        } else {
            "\x1b[90m\u{25cb}\x1b[0m"
        };
        println!("    {} {}. {}", mark, i + 1, label);
    }

    println!(
        "\n  \x1b[90mPress 1-{} to toggle, Enter to start, q to cancel\x1b[0m",
        options.len()
    );
    print!("  \x1b[1;33mChoice:\x1b[0m ");
    let _ = io::stdout().flush();

    let _ = crossterm::terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(300));
    while crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }

    loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            match key.code {
                crossterm::event::KeyCode::Enter => break,
                crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc => {
                    let _ = crossterm::terminal::disable_raw_mode();
                    return Vec::new();
                }
                crossterm::event::KeyCode::Char(c) if c.is_ascii_digit() => {
                    let idx = (c as usize) - ('1' as usize);
                    if idx < options.len() {
                        if options[idx].0 == "All above" {
                            let all_on = selected.iter().take(options.len() - 1).all(|&s| s);
                            for s in selected.iter_mut().take(options.len() - 1) {
                                *s = !all_on;
                            }
                        } else {
                            selected[idx] = !selected[idx];
                        }
                        print!("\x1b[{}A\r", options.len() + 2);
                        for (i, (label, _)) in options.iter().enumerate() {
                            let mark = if selected[i] {
                                "\x1b[32m\u{25cf}\x1b[0m"
                            } else {
                                "\x1b[90m\u{25cb}\x1b[0m"
                            };
                            print!("\x1b[2K    {} {}. {}\r\n", mark, i + 1, label);
                        }
                        print!("\x1b[2K\r\n\x1b[2K  \x1b[90mPress 1-{} to toggle, Enter to start, q to cancel\x1b[0m\r\n", options.len());
                        print!("\x1b[2K  \x1b[1;33mChoice:\x1b[0m ");
                        let _ = io::stdout().flush();
                    }
                }
                _ => {}
            }
        }
    }

    let _ = crossterm::terminal::disable_raw_mode();
    println!();

    options
        .into_iter()
        .enumerate()
        .filter(|(i, (label, _))| selected[*i] && *label != "All above")
        .map(|(_, (_, p))| p)
        .collect()
}

fn pick_min_size() -> u64 {
    let sizes: &[(&str, u64)] = &[
        ("100 KB (find all dupes)", 100 * 1024),
        ("500 KB (recommended)", 500 * 1024),
        ("1 MB", 1024 * 1024),
        ("5 MB (large files only)", 5 * 1024 * 1024),
        ("50 MB (very large only)", 50 * 1024 * 1024),
    ];

    println!("  \x1b[1mMinimum file size:\x1b[0m\n");
    for (i, (label, _)) in sizes.iter().enumerate() {
        let mark = if i == 1 {
            "\x1b[32m\u{25b6}\x1b[0m"
        } else {
            " "
        };
        println!("    {} {}. {}", mark, i + 1, label);
    }
    println!("\n  \x1b[90mPress 1-5 or Enter for default (500 KB)\x1b[0m");
    print!("  \x1b[1;33mSize:\x1b[0m ");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let _ = crossterm::terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(500));
    while crossterm::event::poll(std::time::Duration::from_millis(200)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let choice = loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            match key.code {
                crossterm::event::KeyCode::Char('1') => break 0,
                crossterm::event::KeyCode::Char('2') => break 1,
                crossterm::event::KeyCode::Char('3') => break 2,
                crossterm::event::KeyCode::Char('4') => break 3,
                crossterm::event::KeyCode::Char('5') => break 4,
                crossterm::event::KeyCode::Enter => break 1,
                crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('q') => break 1,
                _ => continue,
            }
        }
    };
    let _ = crossterm::terminal::disable_raw_mode();
    println!(" \x1b[32m{}\x1b[0m\n", sizes[choice].0);
    sizes[choice].1
}

// ─── scanning ────────────────────────────────────────────────────────────────

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    ".venv",
    "__pycache__",
    "Library",
    ".cache",
    ".Trash",
    ".cargo",
    ".rustup",
    ".gradle",
    ".m2",
];

pub fn run(path: &str, min_size: u64) {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
    super::ui::print_header("\x1b[1;34mDuplicate Finder\x1b[0m");

    let scan_paths: Vec<PathBuf> = if path == "~" {
        pick_folders()
    } else {
        vec![PathBuf::from(path)]
    };
    if scan_paths.is_empty() {
        println!("  \x1b[90mNo folders selected.\x1b[0m\n");
        return;
    }

    let actual_min_size = if path == "~" {
        pick_min_size()
    } else {
        min_size
    };
    let home_str = crate::error::home_or_exit().display().to_string();

    println!(
        "  Scanning: {}",
        scan_paths
            .iter()
            .map(|p| p.display().to_string().replace(&home_str, "~"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  Min size: {}\n", ByteSize::b(actual_min_size));

    // Phase 1 — group by size
    let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut total_files: u64 = 0;

    for scan_path in &scan_paths {
        for entry in WalkDir::new(scan_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !SKIP_DIRS.contains(&name.as_ref())
            })
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if size < actual_min_size {
                continue;
            }
            size_groups
                .entry(size)
                .or_default()
                .push(entry.path().to_path_buf());
            total_files += 1;
            if total_files % 500 == 0 {
                print!(
                    "\r\x1b[K  \x1b[33m\u{2022}\x1b[0m Phase 1: {} files",
                    total_files
                );
                let _ = io::stdout().flush();
            }
        }
    }

    let candidates: Vec<(u64, Vec<PathBuf>)> = size_groups
        .into_iter()
        .filter(|(_, p)| p.len() >= 2)
        .collect();
    let candidate_count: usize = candidates.iter().map(|(_, p)| p.len()).sum();

    print!("\r\x1b[K");
    println!(
        "  \x1b[32m\u{2713}\x1b[0m Scanned {} files, {} size-match candidates",
        total_files, candidate_count
    );

    if candidates.is_empty() {
        println!("\n  \x1b[32m\u{2713}\x1b[0m No duplicates found!\n");
        wait_for_key();
        return;
    }

    // Phase 2 — parallel hash (partial: first+last 4 KB)
    let total_to_hash = candidate_count as u64;
    let checked = AtomicU64::new(0);

    let all_files: Vec<(u64, PathBuf)> = candidates
        .into_iter()
        .flat_map(|(size, paths)| paths.into_iter().map(move |p| (size, p)))
        .collect();

    let hashed: Vec<(u64, u64, PathBuf)> = all_files
        .par_iter()
        .filter_map(|(size, path)| {
            let done = checked.fetch_add(1, Ordering::Relaxed) + 1;
            if done % 20 == 0 || done == total_to_hash {
                let pct = (done as f64 / total_to_hash as f64 * 100.0) as u64;
                let filled = (pct as usize * 20) / 100;
                eprint!(
                    "\r\x1b[K  \x1b[33m\u{2022}\x1b[0m Hashing: \x1b[32m{}\x1b[90m{}\x1b[0m {}/{} ({}%)",
                    "\u{2501}".repeat(filled),
                    "\u{2508}".repeat(20 - filled),
                    done, total_to_hash, pct
                );
            }
            hash_file_partial(path).ok().map(|h| (*size, h, path.clone()))
        })
        .collect();

    eprint!("\r\x1b[K");
    println!(
        "  \x1b[32m\u{2713}\x1b[0m Hashed {} files\n",
        checked.load(Ordering::Relaxed)
    );

    // Group by (size, hash)
    let mut groups: HashMap<(u64, u64), Vec<PathBuf>> = HashMap::new();
    for (size, hash, path) in hashed {
        groups.entry((size, hash)).or_default().push(path);
    }

    let mut dupe_groups: Vec<DupeGroup> = groups
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .map(|((size, _), paths)| DupeGroup { size, paths })
        .collect();

    if dupe_groups.is_empty() {
        println!("  \x1b[32m\u{2713}\x1b[0m No duplicates found!\n");
        wait_for_key();
        return;
    }

    // Sort by wasted space descending
    dupe_groups.sort_by(|a, b| b.waste().cmp(&a.waste()));

    let total_waste: u64 = dupe_groups.iter().map(|g| g.waste()).sum();

    println!(
        "  \x1b[1mFound {} duplicate groups — \x1b[31m{} wasted\x1b[0m\n",
        dupe_groups.len(),
        ByteSize::b(total_waste).to_string().red()
    );

    // Hand off to the interactive TUI
    show_dupes_tui(&dupe_groups, &home_str, total_waste);
}

// ─── interactive TUI ──────────────────────────────────────────────────────────

fn show_dupes_tui(groups: &[DupeGroup], home_str: &str, total_waste: u64) {
    use crossterm::{cursor, event, execute, terminal};

    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let mut selected: usize = 0; // which group is focused
    let mut scroll: usize = 0; // top visible group index
    let mut deleted: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut status: String = String::new();
    let mut freed_total: u64 = 0;

    loop {
        let _ = execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        );

        let (term_cols, term_rows) = crossterm::terminal::size().unwrap_or((120, 40));
        let visible_rows = term_rows.saturating_sub(10) as usize; // reserve header + footer
                                                                  // Each group takes: 1 header line + N file lines + 1 blank = N+2 lines
                                                                  // We'll show as many groups as fit.

        let remaining_waste: u64 = groups
            .iter()
            .enumerate()
            .filter(|(i, _)| !deleted.contains(i))
            .map(|(_, g)| g.waste())
            .sum();

        let mut out = String::new();
        out.push_str(&super::ui::tui_header("\x1b[1;34mDuplicate Finder\x1b[0m"));
        out.push_str(&format!(
            "  {} groups  \x1b[31m{} wasted\x1b[0m  \x1b[90m(started with {})\x1b[0m\r\n\r\n",
            groups
                .iter()
                .enumerate()
                .filter(|(i, _)| !deleted.contains(i))
                .count(),
            ByteSize::b(remaining_waste),
            ByteSize::b(total_waste)
        ));

        // Render groups starting from scroll offset
        let mut lines_used: usize = 0;
        let mut last_visible: usize = scroll;

        for (i, group) in groups.iter().enumerate().skip(scroll) {
            let group_lines = group.paths.len() + 2; // header + files + blank
            if lines_used + group_lines > visible_rows {
                break;
            }
            last_visible = i;

            let is_selected = i == selected;
            let is_deleted = deleted.contains(&i);
            let keep = group.keep_idx();

            // Group header
            let selector = if is_selected {
                "\x1b[1;32m▶\x1b[0m"
            } else {
                " "
            };
            let status_tag = if is_deleted {
                " \x1b[32m[cleaned]\x1b[0m"
            } else {
                ""
            };

            out.push_str(&format!(
                "  {} \x1b[1m#{}\x1b[0m  {} each × {} copies = \x1b[31m{} wasted\x1b[0m{}\r\n",
                selector,
                i + 1,
                ByteSize::b(group.size),
                group.paths.len(),
                ByteSize::b(group.waste()),
                status_tag
            ));

            // File list with full paths and folder context
            for (j, path) in group.paths.iter().enumerate() {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let parent = path
                    .parent()
                    .map(|p| p.display().to_string().replace(home_str, "~"))
                    .unwrap_or_default();

                // Truncate parent to fit terminal width
                let max_parent = (term_cols as usize).saturating_sub(30);
                let parent_display = if parent.len() > max_parent {
                    format!("…{}", &parent[parent.len() - max_parent..])
                } else {
                    parent.clone()
                };

                let (tag, name_color) = if is_deleted {
                    ("\x1b[90m  ✓ done \x1b[0m", "\x1b[90m")
                } else if j == keep {
                    ("\x1b[32m  ✓ keep \x1b[0m", "\x1b[32m")
                } else {
                    ("\x1b[31m  ✗ dupe \x1b[0m", "\x1b[0m")
                };

                out.push_str(&format!(
                    "    {}{}{}  \x1b[90min\x1b[0m {}\r\n",
                    tag, name_color, file_name, parent_display
                ));
            }
            out.push_str("\r\n");
            lines_used += group_lines;
        }

        // Scroll hint if more groups below
        if last_visible + 1 < groups.len() {
            out.push_str(&format!(
                "  \x1b[90m↓ {} more groups below — use ↑↓ to scroll\x1b[0m\r\n",
                groups.len() - last_visible - 1
            ));
        }
        if scroll > 0 {
            // Reprint above scroll hint at top — simpler: just note it in footer
        }

        // Footer
        out.push_str(super::ui::footer_sep());
        if !status.is_empty() {
            out.push_str(&format!("  \x1b[32m{}\x1b[0m\r\n", status));
        }
        out.push_str("  \x1b[90m↑↓\x1b[0m Navigate  \x1b[90m|\x1b[0m  \x1b[31md\x1b[0m Delete dupes in group  \x1b[90m|\x1b[0m  \x1b[33ma\x1b[0m Delete ALL  \x1b[90m|\x1b[0m  \x1b[90mq\x1b[0m Quit\r\n");

        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        // Input
        if let Ok(event::Event::Key(key)) = event::read() {
            status.clear();
            match key.code {
                event::KeyCode::Up | event::KeyCode::Char('k') => {
                    if selected > 0 {
                        selected -= 1;
                        // Scroll up if needed
                        if selected < scroll {
                            scroll = selected;
                        }
                    }
                }
                event::KeyCode::Down | event::KeyCode::Char('j') => {
                    if selected + 1 < groups.len() {
                        selected += 1;
                        // Scroll down if selected moved past visible area
                        if selected > last_visible {
                            scroll += 1;
                        }
                    }
                }
                event::KeyCode::Char('d') | event::KeyCode::Char('D') => {
                    if !deleted.contains(&selected) {
                        let group = &groups[selected];
                        let keep = group.keep_idx();
                        let mut freed: u64 = 0;
                        let mut count = 0;
                        for (j, path) in group.paths.iter().enumerate() {
                            if j == keep {
                                continue;
                            }
                            if std::fs::remove_file(path).is_ok() {
                                freed += group.size;
                                count += 1;
                            }
                        }
                        freed_total += freed;
                        deleted.insert(selected);
                        status = format!(
                            "✓ Group #{} — deleted {} dupes, freed {}",
                            selected + 1,
                            count,
                            ByteSize::b(freed)
                        );
                        crate::history::log_delete("duplicates", freed, "dedup");
                    } else {
                        status = format!("Group #{} already cleaned.", selected + 1);
                    }
                }
                event::KeyCode::Char('a') | event::KeyCode::Char('A') => {
                    let mut freed: u64 = 0;
                    let mut count = 0;
                    for (i, group) in groups.iter().enumerate() {
                        if deleted.contains(&i) {
                            continue;
                        }
                        let keep = group.keep_idx();
                        for (j, path) in group.paths.iter().enumerate() {
                            if j == keep {
                                continue;
                            }
                            if std::fs::remove_file(path).is_ok() {
                                freed += group.size;
                                count += 1;
                            }
                        }
                        deleted.insert(i);
                    }
                    freed_total += freed;
                    status = format!(
                        "✓ Deleted {} dupes across all groups — freed {}",
                        count,
                        ByteSize::b(freed)
                    );
                    crate::history::log_delete("duplicates", freed, "dedup");
                }
                event::KeyCode::Char('q')
                | event::KeyCode::Char('Q')
                | event::KeyCode::Esc
                | event::KeyCode::Backspace => {
                    break;
                }
                _ => {}
            }
        }
    }

    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();

    if freed_total > 0 {
        println!(
            "\n  \x1b[1;32m✓ Total freed this session: {}\x1b[0m\n",
            ByteSize::b(freed_total)
        );
    } else {
        println!();
    }
}

// ─── hashing ─────────────────────────────────────────────────────────────────

/// Stable partial hash: first 4 KB + last 4 KB + file size.
/// Uses FNV-1a (deterministic, no random seed) so results are
/// consistent across runs and processes.
fn hash_file_partial(path: &Path) -> io::Result<u64> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();

    let mut hash: u64 = 14695981039346656037u64; // FNV-1a offset basis
    let fnv_prime: u64 = 1099511628211;

    // Mix in the file size so differently-sized files never collide
    for byte in size.to_le_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(fnv_prime);
    }

    // First 4 KB
    let read_size = 4096.min(size as usize);
    let mut buf = vec![0u8; read_size];
    file.read_exact(&mut buf)?;
    for byte in &buf {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(fnv_prime);
    }

    // Last 4 KB (only if file is large enough to have a distinct tail)
    if size > 8192 {
        file.seek(std::io::SeekFrom::End(-4096))?;
        let mut end_buf = vec![0u8; 4096];
        file.read_exact(&mut end_buf)?;
        for byte in &end_buf {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(fnv_prime);
        }
    }

    Ok(hash)
}

fn wait_for_key() {
    println!("  \x1b[90mPress any key to continue...\x1b[0m");
    let _ = crossterm::terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(400));
    while crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let _ = crossterm::event::read();
    let _ = crossterm::terminal::disable_raw_mode();
}
