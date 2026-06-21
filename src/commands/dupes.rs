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

/// Default scan paths (Documents + Downloads — where dupes usually live).
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
        default_scan_paths()
    } else {
        vec![PathBuf::from(path)]
    };

    let home_str = crate::error::home_or_exit().display().to_string();
    println!("  Scanning: {}",
        scan_paths.iter()
            .map(|p| p.display().to_string().replace(&home_str, "~"))
            .collect::<Vec<_>>()
            .join(", "));
    println!("  Min size: {}\n", ByteSize::b(min_size));

    // Phase 1: Group files by size (fast, just stat calls)
    print!("  \x1b[33m\u{2022}\x1b[0m Phase 1: Scanning files...\r");
    let _ = io::stdout().flush();

    let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut total_files: u64 = 0;

    for scan_path in &scan_paths {
        for entry in WalkDir::new(scan_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() { continue; }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if size < min_size { continue; }

            size_groups.entry(size)
                .or_default()
                .push(entry.path().to_path_buf());
            total_files += 1;
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

    // Phase 2: Parallel hashing
    print!("  \x1b[33m\u{2022}\x1b[0m Phase 2: Hashing {} files (parallel)...\r", candidate_count);
    let _ = io::stdout().flush();

    let checked = AtomicU64::new(0);
    let mut dupe_groups: Vec<DupeGroup> = Vec::new();

    for (size, paths) in &candidates {
        // Hash all files in this size group in parallel
        let hashes: Vec<(u64, PathBuf)> = paths.par_iter()
            .filter_map(|path| {
                checked.fetch_add(1, Ordering::Relaxed);
                hash_file_partial(path).ok().map(|h| (h, path.clone()))
            })
            .collect();

        // Group by hash
        let mut hash_map: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        for (hash, path) in hashes {
            hash_map.entry(hash).or_default().push(path);
        }

        // Same hash = duplicates
        for (_, group_paths) in hash_map {
            if group_paths.len() >= 2 {
                dupe_groups.push(DupeGroup {
                    size: *size,
                    paths: group_paths,
                });
            }
        }
    }

    print!("\r\x1b[K");
    println!("  \x1b[32m\u{2713}\x1b[0m Hashed {} files\n", checked.load(Ordering::Relaxed));

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

/// Hash first 64KB + last 64KB of a file. Fast fingerprint.
fn hash_file_partial(path: &Path) -> io::Result<u64> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    let size = metadata.len();

    let mut hasher = DefaultHasher::new();
    size.hash(&mut hasher);

    // Read first 64KB
    let read_size = 65536.min(size as usize);
    let mut buf = vec![0u8; read_size];
    file.read_exact(&mut buf)?;
    buf.hash(&mut hasher);

    // Read last 64KB if file is large enough
    if size > 131072 {
        use std::io::Seek;
        file.seek(std::io::SeekFrom::End(-65536))?;
        let mut end_buf = vec![0u8; 65536];
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
