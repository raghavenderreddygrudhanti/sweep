//! Browser cache cleaner — Chrome, Safari, Firefox, Brave, Arc, Edge.
//!
//! Safety notes:
//! - Only true *cache* directories are cleaned. User data such as
//!   `Safari/LocalStorage` (site storage, logins, app state) is never touched.
//! - Some cache directories (Service Workers, Code Cache) live *inside* the live
//!   browser profile. Deleting those while the browser is running can corrupt the
//!   profile, so any browser that is currently running is skipped entirely.

use std::path::PathBuf;
use sysinfo::System;

/// A browser's known cache locations, tagged with process-name fragments used to
/// detect whether the browser is currently running.
struct Browser {
    /// Process-name substrings (lowercase) that indicate the browser is running.
    process_match: &'static [&'static str],
    /// (relative-to-home path, human label) cache locations.
    caches: Vec<(PathBuf, &'static str)>,
}

fn browsers(home: &std::path::Path) -> Vec<Browser> {
    vec![
        Browser {
            process_match: &["google chrome", "chrome"],
            caches: vec![
                (home.join("Library/Caches/Google/Chrome"), "Chrome cache"),
                (home.join("Library/Application Support/Google/Chrome/Default/Service Worker/CacheStorage"), "Chrome Service Workers"),
                (home.join("Library/Application Support/Google/Chrome/Default/Code Cache"), "Chrome Code Cache"),
                (home.join(".cache/google-chrome"), "Chrome cache (Linux)"),
            ],
        },
        Browser {
            process_match: &["safari"],
            // NOTE: Safari/LocalStorage is user data, not cache — intentionally excluded.
            caches: vec![
                (home.join("Library/Caches/com.apple.Safari"), "Safari cache"),
            ],
        },
        Browser {
            process_match: &["firefox"],
            caches: vec![
                (home.join("Library/Caches/Firefox"), "Firefox cache"),
                (home.join(".cache/mozilla/firefox"), "Firefox cache (Linux)"),
            ],
        },
        Browser {
            process_match: &["brave"],
            caches: vec![
                (home.join("Library/Caches/BraveSoftware/Brave-Browser"), "Brave cache"),
                (home.join("Library/Application Support/BraveSoftware/Brave-Browser/Default/Service Worker/CacheStorage"), "Brave Service Workers"),
            ],
        },
        Browser {
            process_match: &["arc"],
            caches: vec![
                (home.join("Library/Caches/company.thebrowser.Browser"), "Arc cache"),
            ],
        },
        Browser {
            process_match: &["microsoft edge", "msedge", "edge"],
            caches: vec![
                (home.join("Library/Caches/Microsoft Edge"), "Edge cache"),
                (home.join("Library/Application Support/Microsoft Edge/Default/Service Worker/CacheStorage"), "Edge Service Workers"),
            ],
        },
        Browser {
            process_match: &["vivaldi"],
            caches: vec![
                (home.join("Library/Caches/Vivaldi"), "Vivaldi cache"),
            ],
        },
    ]
}

/// Lowercased list of currently-running process names.
fn running_process_names() -> Vec<String> {
    let mut sys = System::new();
    sys.refresh_processes();
    sys.processes()
        .values()
        .map(|p| p.name().to_lowercase())
        .collect()
}

fn is_running(process_match: &[&str], running: &[String]) -> bool {
    running
        .iter()
        .any(|name| process_match.iter().any(|m| name.contains(m)))
}

/// All cleanable browser cache locations, excluding browsers that are currently
/// running (to avoid profile corruption) and locations that don't exist.
pub fn browser_cache_paths() -> Vec<(PathBuf, &'static str)> {
    let home = crate::error::home_or_exit();
    let running = running_process_names();

    browsers(&home)
        .into_iter()
        .filter(|b| !is_running(b.process_match, &running))
        .flat_map(|b| b.caches.into_iter())
        .filter(|(p, _)| p.exists())
        .collect()
}
