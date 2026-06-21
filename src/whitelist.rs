//! Whitelist — paths that sweep will never clean or delete.
//! Stored at ~/.sweep/whitelist.txt, one pattern per line.
//! Supports glob patterns and exact paths.

use std::path::{Path, PathBuf};
use std::fs;

const DEFAULT_WHITELIST: &[&str] = &[
    "~/Documents",
    "~/Desktop",
    "~/Pictures",
    "~/Movies",
    "~/Music",
    "~/.ssh",
    "~/.gnupg",
    "~/.aws",
    "~/.kube",
    "~/.gitconfig",
    "~/.zshrc",
    "~/.bashrc",
    "~/.env",
    "~/Library/Keychains",
    "~/Library/Mail",
    "~/Library/Messages",
    "~/Library/Calendars",
    "~/Library/Contacts",
    "~/Library/Safari/Bookmarks.plist",
    "~/Library/Application Support/com.apple.sharedfilelist",
];

fn whitelist_path() -> PathBuf {
    let dir = dirs::home_dir().unwrap_or_default().join(".sweep");
    let _ = fs::create_dir_all(&dir);
    dir.join("whitelist.txt")
}

/// Load whitelist patterns. Creates default if not exists.
pub fn load_whitelist() -> Vec<String> {
    let path = whitelist_path();

    if !path.exists() {
        // Create default whitelist
        let home = dirs::home_dir().unwrap_or_default().display().to_string();
        let content: Vec<String> = DEFAULT_WHITELIST.iter()
            .map(|p| p.replace('~', &home))
            .collect();
        let _ = fs::write(&path, content.join("\n") + "\n");
        return content;
    }

    fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let home = dirs::home_dir().unwrap_or_default().display().to_string();
            l.replace('~', &home)
        })
        .collect()
}

/// Check if a path is protected by the whitelist.
pub fn is_protected(path: &Path) -> bool {
    let whitelist = load_whitelist();
    let path_str = path.display().to_string();

    for pattern in &whitelist {
        if path_str.starts_with(pattern) || path_str == *pattern {
            return true;
        }
    }
    false
}

/// Show whitelist contents.
pub fn show_whitelist() {
    let patterns = load_whitelist();
    let home = dirs::home_dir().unwrap_or_default().display().to_string();

    println!("\n  \x1b[1mProtected paths ({} patterns):\x1b[0m\n", patterns.len());
    for p in &patterns {
        let display = p.replace(&home, "~");
        println!("    \x1b[32m\u{2022}\x1b[0m {}", display);
    }
    println!("\n  \x1b[90mEdit: ~/.sweep/whitelist.txt\x1b[0m\n");
}

/// Add a path to the whitelist.
pub fn add_to_whitelist(path: &str) {
    let wl_path = whitelist_path();
    let mut content = fs::read_to_string(&wl_path).unwrap_or_default();
    if !content.ends_with('\n') { content.push('\n'); }
    content.push_str(path);
    content.push('\n');
    let _ = fs::write(&wl_path, content);
}
