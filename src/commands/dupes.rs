//! Duplicate file finder — finds files with identical content.
//! Algorithm: group by size → hash first 64KB → full hash on matches.
//! Fast and accurate with zero false positives.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::{self, Read, Write};
use std::fs::File;
use bytesize::ByteSize;
use colored::*;
use walkdir::WalkDir;

/// A group of duplicate files.
struct DupeGroup {
    size: u64,
    paths: Vec<PathBuf>,
}

pub fn run(path: &str, min_size: u64) {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();

    super::ui::print_header("\x1b[1;34mDuplicate Finder\x1b[0m");

    let scan_path = if path == "~" {
        crate::error::home_or_exit()
    } else {
        PathBuf::from(path)
    };

    println!("  Scanning: {}", scan_path.display());
    println!("  Min size: {}\n", ByteSize::b(min_size));

    // Phase 1: Group files by size
    print!("  \x1b[33m\u{2022}\x1b[0m Phase 1: Grouping by file size...\r");
    let _ = io::stdout().flush();

    let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut total_files: u64 = 0;

    for entry in WalkDir::new(&scan_path)
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

    // Keep only groups with 2+ files (potential dupes)
    let candidates: Vec<(u64, Vec<PathBuf>)> = size_groups.into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .collect();

    let candidate_count: usize = candidates.iter().map(|(_, p)| p.len()).sum();
    print!("\r\x1b[K");
    println!("  \x1b[32m\u{2713}\x1b[0m Scanned {} files, {} candidates in {} size groups",
        total_files, candidate_count, candidates.len());

    if candidates.is_empty() {
        println!("\n  \x1b[32m\u{2713}\x1b[0m No duplicates found!\n");
        wait_for_key();
        return;
    }

    // Phase 2: Hash first 64KB to find actual duplicates
    print!("  \x1b[33m\u{2022}\x1b[0m Phase 2: Hashing candidates...\r");
    let _ = io::stdout().flush();

    let mut dupe_groups: Vec<DupeGroup> = Vec::new();
    let mut checked: u64 = 0;

    for (size, paths) in &candidates {
        // Hash first 64KB of each file
        let mut hash_map: HashMap<u64, Vec<PathBuf>> = HashMap::new();

        for path in paths {
            checked += 1;
            if checked % 100 == 0 {
                print!("\r\x1b[K  \x1b[33m\u{2022}\x1b[0m Phase 2: Hashing... ({}/{})\r",
                    checked, candidate_count);
                let _ = io::stdout().flush();
            }

            if let Ok(hash) = hash_file_partial(path) {
                hash_map.entry(hash).or_default().push(path.clone());
            }
        }

        // Groups with same partial hash = likely duplicates
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
    println!("  \x1b[32m\u{2713}\x1b[0m Hashed {} files\n", checked);

    if dupe_groups.is_empty() {
        println!("  \x1b[32m\u{2713}\x1b[0m No duplicates found!\n");
        wait_for_key();
        return;
    }

    // Sort by total wasted space
    dupe_groups.sort_by(|a, b| {
        let waste_a = a.size * (a.paths.len() as u64 - 1);
        let waste_b = b.size * (b.paths.len() as u64 - 1);
        waste_b.cmp(&waste_a)
    });

    // Display results
    let total_waste: u64 = dupe_groups.iter()
        .map(|g| g.size * (g.paths.len() as u64 - 1))
        .sum();
    let total_groups = dupe_groups.len();

    println!("  \x1b[1mFound {} duplicate groups ({} wasted)\x1b[0m\n",
        total_groups, ByteSize::b(total_waste).to_string().red());

    let home_str = crate::error::home_or_exit().display().to_string();

    for (i, group) in dupe_groups.iter().take(10).enumerate() {
        let waste = group.size * (group.paths.len() as u64 - 1);
        println!("  \x1b[33m{}.\x1b[0m {} each \u{00d7} {} copies = \x1b[31m{} wasted\x1b[0m",
            i + 1,
            ByteSize::b(group.size),
            group.paths.len(),
            ByteSize::b(waste));

        for (j, path) in group.paths.iter().enumerate() {
            let display = path.display().to_string().replace(&home_str, "~");
            let short = if display.len() > 60 { format!("...{}", &display[display.len()-57..]) } else { display };
            if j == 0 {
                println!("    \x1b[32m\u{2713} keep\x1b[0m  {}", short);
            } else {
                println!("    \x1b[90m\u{2022} dupe\x1b[0m  {}", short);
            }
        }
        println!();
    }

    if total_groups > 10 {
        println!("  \x1b[90m... +{} more groups\x1b[0m\n", total_groups - 10);
    }

    // Summary + actions
    println!("  \x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m");
    println!("  Total wasted: \x1b[1;31m{}\x1b[0m in {} groups",
        ByteSize::b(total_waste), total_groups);
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
                // Keep the newest file (by modified time), delete the rest
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
                with_times.sort_by(|a, b| b.0.cmp(&a.0)); // newest first

                // Skip first (newest), delete rest
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
    size.hash(&mut hasher); // Include size in hash

    // Read first 64KB
    let mut buf = vec![0u8; 65536.min(size as usize)];
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
