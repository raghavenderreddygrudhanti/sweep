//! Browser cache cleaner — Chrome, Safari, Firefox, Brave, Arc, Edge.

use std::path::PathBuf;
use dirs;

/// All known browser cache locations (macOS + Linux).
pub fn browser_cache_paths() -> Vec<(PathBuf, &'static str)> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut paths = vec![];

    // ─── Chrome ──────────────────────────────────────────
    paths.push((
        home.join("Library/Caches/Google/Chrome"),
        "Chrome cache",
    ));
    paths.push((
        home.join("Library/Application Support/Google/Chrome/Default/Service Worker/CacheStorage"),
        "Chrome Service Workers",
    ));
    paths.push((
        home.join("Library/Application Support/Google/Chrome/Default/Code Cache"),
        "Chrome Code Cache",
    ));
    paths.push((
        home.join(".cache/google-chrome"),
        "Chrome cache (Linux)",
    ));

    // ─── Safari ──────────────────────────────────────────
    paths.push((
        home.join("Library/Caches/com.apple.Safari"),
        "Safari cache",
    ));
    paths.push((
        home.join("Library/Safari/LocalStorage"),
        "Safari LocalStorage",
    ));

    // ─── Firefox ─────────────────────────────────────────
    paths.push((
        home.join("Library/Caches/Firefox"),
        "Firefox cache",
    ));
    paths.push((
        home.join(".cache/mozilla/firefox"),
        "Firefox cache (Linux)",
    ));

    // ─── Brave ───────────────────────────────────────────
    paths.push((
        home.join("Library/Caches/BraveSoftware/Brave-Browser"),
        "Brave cache",
    ));
    paths.push((
        home.join("Library/Application Support/BraveSoftware/Brave-Browser/Default/Service Worker/CacheStorage"),
        "Brave Service Workers",
    ));

    // ─── Arc ─────────────────────────────────────────────
    paths.push((
        home.join("Library/Caches/company.thebrowser.Browser"),
        "Arc cache",
    ));

    // ─── Edge ────────────────────────────────────────────
    paths.push((
        home.join("Library/Caches/Microsoft Edge"),
        "Edge cache",
    ));
    paths.push((
        home.join("Library/Application Support/Microsoft Edge/Default/Service Worker/CacheStorage"),
        "Edge Service Workers",
    ));

    // ─── Vivaldi ─────────────────────────────────────────
    paths.push((
        home.join("Library/Caches/Vivaldi"),
        "Vivaldi cache",
    ));

    paths.into_iter().filter(|(p, _)| p.exists()).collect()
}
