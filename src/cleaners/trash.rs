//! Trash management — empty trash, show trash size.

use std::path::PathBuf;
use dirs;

/// Get trash directory path.
pub fn trash_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();

    #[cfg(target_os = "macos")]
    {
        home.join(".Trash")
    }

    #[cfg(target_os = "linux")]
    {
        home.join(".local/share/Trash/files")
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        home.join(".Trash")
    }
}

/// Empty trash (move to /dev/null equivalent).
pub fn empty_trash(dry_run: bool) -> u64 {
    let trash = trash_path();
    if !trash.exists() {
        return 0;
    }

    let (size, _) = crate::scanner::scan_size(&trash);

    if !dry_run && size > 0 {
        if let Ok(entries) = std::fs::read_dir(&trash) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let _ = std::fs::remove_dir_all(&path);
                } else {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }

    size
}
