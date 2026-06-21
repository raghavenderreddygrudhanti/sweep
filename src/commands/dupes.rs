//! Duplicate file finder — finds files with identical content.
//! Algorithm: group by size → parallel hash first+last 64KB → find matches.
//! Uses rayon for parallel hashing across all CPU cores.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::{self, Read, Write};
use std::fs::File;
use std::sync::atomic::{AtomicU64, Ordering};
use bytesize::ByteSize;
use colored::*;
use walkdir::WalkDir;
use rayon::prelude::*;

/// A group of duplicate files.
struct DupeGroup {
    size: u64,
    paths: Vec<PathBuf>,
}

/// Interactive folder picker — user selects which folders to scan.
fn pick_folders() -> Vec<PathBuf> {
    let home = crate::error::home_or_exit();

    let options: Vec<(&str, PathBuf)> = vec![
        ("Documents", home.join("Documents")),
        ("Downloads", home.join("Downloads")),
        ("Desktop",   home.join("Desktop")),
        ("Pictures",  home.join("Pictures")),
        ("Movies",    home.join("Movies")),
        ("Music",     home.join("Music")),
        ("All above", home.clone()),
    ]
    .into_iter()
    .filter(|(label, p)| *label == "All above" || p.exists())
    .collect();

    println!("  \x1b[1mSelect folders to scan for duplicates:\x1b[0m\n");

    let mut selected = vec![false; options.len()];

    for (i, (label, _)) in options.iter().enumerate() {
        let mark = if selected[i] { "\x1b[32m\u{25cf}\x1b[0m" } else { "\x1b[90m\u{25cb}\x1b[0m" };
        println!("    {} {}. {}", mark, i + 1, label);
    }

    println!("\n  \x1b[90mPress 1-{} to toggle, Enter to start, q to cancel\x1b[0m", options.len());
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
                        // "All above" toggles everything
                        if options[idx].0 == "All above" {
                            let all_on = selected.iter().take(options.len() - 1).all(|&s| s);
                            for s in selected.iter_mut().take(options.len() - 1) {
                                *s = !all_on;
                            }
                        } else {
                            selected[idx] = !selected[idx];
                        }
                        // Redraw from top of options
                        print!("\x1b[{}A\r", options.len() + 3);
                        for (i, (label, _)) in options.iter().enumerate() {
                            let mark = if selected[i] { "\x1b[32m\u{25cf}\x1b[0m" } else { "\x1b[90m\u{25cb}\x1b[0m" };
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

    let paths: Vec<PathBuf> = options.into_iter()
        .enumerate()
        .filter(|(i, (label, _))| selected[*i] && *label != "All above")
        .map(|(_, (_, p))| p)
        .collect();

    paths
}

/// Default scan paths (used by CLI with explicit path).
fn default_scan_paths() -> Vec<PathBuf> {
    let home = crate::error::home_or_exit();
    vec![
        home.join("Documents"),
        home.join("Downloads"),
        home.join("Desktop"),
        home.join("Pictures"),
        home.join("Movies"),
        home.join("Music"),
    ]
    .into_iter()
    .filter(|p| p.exists())
    .collect()
}

pub fn run(path: &str, min_size: u64) {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();

    super::ui::print_header("\x1b[1;34mDuplicate Finder\x1b[0m");

    // Determine scan paths
    let scan_paths: Vec<PathBuf> = if path == "~" {
        // Interactive: let user choose which folders to scan
        pick_folders()
    } else {
        vec![PathBuf::from(path)]
    };

    if scan_paths.is_empty() {
        println!("  \x1b[90mNo folders selected.\x1b[0m\n");
        return;
    }

    let home_str = crate::error::home_or_exit().display().to_string();
    println!("  Scanning: {}",
        scan_paths.iter()
            .map(|p| p.display().to_string().replace(&home_str, "~"))
            .collect::<Vec<_>>()
            .join(", "));
    println!("  Min size: {}\n", ByteSize::b(min_size));

    // Phase 1: Group files by size (skip junk dirs, parallel-friendly)
    let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut total_files: u64 = 0;

    let skip_dirs = ["node_modules", ".git", "target", ".venv", "__pycache__",
        "Library", ".cache", ".Trash", ".cargo", ".rustup", ".gradle", ".m2"];

    for scan_path in &scan_paths {
        for entry in WalkDir::new(scan_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !skip_dirs.contains(&name.as_ref())
            })
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() { continue; }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if size < min_size { continue; }

            size_groups.entry(size)
                .or_default()
                .push(entry.path().to_path_buf());
            total_files += 1;

            if total_files % 500 == 0 {
                print!("\r\x1b[K  \x1b[33m\u{2022}\x1b[0m Phase 1: {} files", total_files);
                let _ = io::stdout().flush();
            }
        }
    }

    // Keep only groups with 2+ files
    let candidates: Vec<(u64, Vec<PathBuf>)> = size_groups.into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .collect();

    let candidate_count: usize = candidates.iter().map(|(_, p)| p.len()).sum();
    print!("\r\x1b[K");
    println!("  \x1b[32m\u{2713}\x1b[0m Scanned {} files, {} candidates",
        total_files, candidate_count);

    if candidates.is_empty() {
        println!("\n  \x1b[32m\u{2713}\x1b[0m No duplicates found!\n");
        wait_for_key();
        return;
    }

    // Phase 2: Parallel hashing ALL candidates at once (max parallelism)
    let total_to_hash = candidate_count as u64;
    let checked = AtomicU64::new(0);

    // Flatten all candidates into one list for maximum parallel throughput
    let all_files: Vec<(u64, PathBuf)> = candidates.into_iter()
        .flat_map(|(size, paths)| paths.into_iter().map(move |p| (size, p)))
        .collect();
    // candidates consumed — memory freed

    // Hash all in parallel across all cores
    let hashed: Vec<(u64, u64, PathBuf)> = all_files.par_iter()
        .filter_map(|(size, path)| {
            let done = checked.fetch_add(1, Ordering::Relaxed) + 1;
            if done % 20 == 0 || done == total_to_hash {
                let pct = (done as f64 / total_to_hash as f64 * 100.0) as u64;
                let bar_w = 20usize;
                let filled = (pct as usize * bar_w) / 100;
                let empty = bar_w - filled;
                eprint!("\r\x1b[K  \x1b[33m\u{2022}\x1b[0m Hashing: \x1b[32m{}\x1b[90m{}\x1b[0m {}/{} ({}%)",
                    "\u{2501}".repeat(filled), "\u{2508}".repeat(empty),
                    done, total_to_hash, pct);
            }
            hash_file_partial(path).ok().map(|h| (*size, h, path.clone()))
        })
        .collect();

    eprint!("\r\x1b[K");
    println!("  \x1b[32m\u{2713}\x1b[0m Hashed {} files\n", checked.load(Ordering::Relaxed));

    // Group by (size, hash) to find duplicates
    let mut groups: HashMap<(u64, u64), Vec<PathBuf>> = HashMap::new();
    for (size, hash, path) in hashed {
        groups.entry((size, hash)).or_default().push(path);
    }
    // all_files and hashed are consumed/moved — memory freed

    let mut dupe_groups: Vec<DupeGroup> = groups.into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .map(|((size, _), paths)| DupeGroup { size, paths })
        .collect();
    // groups consumed — memory freed

    if dupe_groups.is_empty() {
        println!("  \x1b[32m\u{2713}\x1b[0m No duplicates found!\n");
        wait_for_key();
        return;
    }

    // Sort by wasted space
    dupe_groups.sort_by(|a, b| {
        let waste_a = a.size * (a.paths.len() as u64 - 1);
        let waste_b = b.size * (b.paths.len() as u64 - 1);
        waste_b.cmp(&waste_a)
    });

    // Display
    let total_waste: u64 = dupe_groups.iter()
        .map(|g| g.size * (g.paths.len() as u64 - 1))
        .sum();

    println!("  \x1b[1mFound {} duplicate groups ({} wasted)\x1b[0m\n",
        dupe_groups.len(), ByteSize::b(total_waste).to_string().red());

    for (i, group) in dupe_groups.iter().take(10).enumerate() {
        let waste = group.size * (group.paths.len() as u64 - 1);
        println!("  \x1b[33m{}.\x1b[0m {} each \u{00d7} {} copies = \x1b[31m{} wasted\x1b[0m",
            i + 1, ByteSize::b(group.size), group.paths.len(), ByteSize::b(waste));

        for (j, path) in group.paths.iter().enumerate() {
            let display = path.display().to_string().replace(&home_str, "~");
            let short = if display.len() > 60 {
                format!("...{}", &display[display.len()-57..])
            } else { display };
            if j == 0 {
                println!("    \x1b[32m\u{2713} keep\x1b[0m  {}", short);
            } else {
                println!("    \x1b[90m\u{2022} dupe\x1b[0m  {}", short);
            }
        }
        println!();
    }

    if dupe_groups.len() > 10 {
        println!("  \x1b[90m... +{} more groups\x1b[0m\n", dupe_groups.len() - 10);
    }

    // Summary + actions
    println!("  \x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m");
    println!("  Total wasted: \x1b[1;31m{}\x1b[0m in {} groups",
        ByteSize::b(total_waste), dupe_groups.len());
    println!();
    println!("  \x1b[1mActions:\x1b[0m  \x1b[32ma\x1b[0m delete all dupes (keep newest)  \x1b[90mq\x1b[0m quit");
    println!();
    print!("  \x1b[1;33mChoice:\x1b[0m ");
    let _ = io::stdout().flush();

    let _ = crossterm::terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(400));
    while crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let choice = loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            match key.code {
                crossterm::event::KeyCode::Char(c) => break c,
                crossterm::event::KeyCode::Esc => break 'q',
                _ => continue,
            }
        }
    };
    let _ = crossterm::terminal::disable_raw_mode();
    println!();

    match choice {
        'a' | 'A' => {
            println!("\n  \x1b[33mRemoving duplicates (keeping newest)...\x1b[0m");
            let mut freed: u64 = 0;
            for group in &dupe_groups {
                let mut with_times: Vec<(u64, &PathBuf)> = group.paths.iter()
                    .map(|p| {
                        let mtime = p.metadata()
                            .and_then(|m| m.modified())
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        (mtime, p)
                    })
                    .collect();
                with_times.sort_by(|a, b| b.0.cmp(&a.0));

                for (_, path) in with_times.iter().skip(1) {
                    if std::fs::remove_file(path).is_ok() {
                        freed += group.size;
                    }
                }
            }
            println!("  \x1b[1;32m\u{2713} Freed: {}\x1b[0m\n", ByteSize::b(freed));
            crate::history::log_delete("duplicates", freed, "dedup");
        }
        _ => {
            println!("\n  \x1b[90mDone.\x1b[0m\n");
        }
    }

    wait_for_key();
}

/// Hash first 4KB + last 4KB of a file. Ultra-fast fingerprint.
fn hash_file_partial(path: &Path) -> io::Result<u64> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    let size = metadata.len();

    let mut hasher = DefaultHasher::new();
    size.hash(&mut hasher);

    // Read first 4KB
    let read_size = 4096.min(size as usize);
    let mut buf = vec![0u8; read_size];
    file.read_exact(&mut buf)?;
    buf.hash(&mut hasher);

    // Read last 4KB if file is large enough
    if size > 8192 {
        use std::io::Seek;
        file.seek(std::io::SeekFrom::End(-4096))?;
        let mut end_buf = vec![0u8; 4096];
        file.read_exact(&mut end_buf)?;
        end_buf.hash(&mut hasher);
    }

    Ok(hasher.finish())
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
