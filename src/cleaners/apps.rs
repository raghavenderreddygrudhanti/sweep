//! App uninstaller — find apps + all their remnants (preferences, caches, support files).

use std::path::PathBuf;
use std::fs;

/// Represents an installed application.
#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub name: String,
    pub path: PathBuf,
    pub bundle_id: Option<String>,
    pub size: u64,
}

/// Find all installed applications in /Applications.
pub fn find_installed_apps() -> Vec<InstalledApp> {
    let apps_dir = PathBuf::from("/Applications");
    let mut apps = vec![];

    // Use batch du for all apps at once (single subprocess)
    let sizes = crate::scanner::scan_children(&apps_dir);
    let size_map: std::collections::HashMap<String, u64> = sizes.into_iter()
        .map(|r| (r.path.clone(), r.size))
        .collect();

    if let Ok(entries) = fs::read_dir(&apps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("app") {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let bundle_id = get_bundle_id(&path);
                let size = size_map.get(&path.display().to_string()).copied().unwrap_or(0);

                apps.push(InstalledApp {
                    name,
                    path,
                    bundle_id,
                    size,
                });
            }
        }
    }

    apps.sort_by(|a, b| b.size.cmp(&a.size));
    apps
}

/// Get bundle identifier from Info.plist.
fn get_bundle_id(app_path: &PathBuf) -> Option<String> {
    let plist_path = app_path.join("Contents/Info.plist");
    if !plist_path.exists() {
        return None;
    }

    // Simple extraction — read plist and find CFBundleIdentifier
    let content = fs::read_to_string(&plist_path).ok()?;
    let key = "CFBundleIdentifier";
    let idx = content.find(key)?;
    let after = &content[idx..];
    let start = after.find("<string>")? + 8;
    let end = after[start..].find("</string>")?;
    Some(after[start..start + end].to_string())
}

/// Find all remnant files for an app (by name and bundle_id).
pub fn find_app_remnants(app: &InstalledApp) -> Vec<PathBuf> {
    let home = crate::error::home_or_exit();
    let mut remnants = vec![];

    let search_dirs = vec![
        home.join("Library/Application Support"),
        home.join("Library/Caches"),
        home.join("Library/Preferences"),
        home.join("Library/Logs"),
        home.join("Library/Containers"),
        home.join("Library/Group Containers"),
        home.join("Library/Saved Application State"),
        home.join("Library/WebKit"),
        home.join("Library/HTTPStorages"),
        home.join("Library/Cookies"),
    ];

    let search_terms: Vec<String> = {
        let mut terms = vec![app.name.to_lowercase()];
        if let Some(ref bid) = app.bundle_id {
            terms.push(bid.to_lowercase());
            // Also search for parts of bundle ID (e.g., "com.spotify" → "spotify")
            if let Some(last) = bid.rsplit('.').next() {
                terms.push(last.to_lowercase());
            }
        }
        terms
    };

    for dir in &search_dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if search_terms.iter().any(|term| name.contains(term)) {
                    remnants.push(entry.path());
                }
            }
        }
    }

    // Also check LaunchAgents/LaunchDaemons
    let launch_dirs = vec![
        home.join("Library/LaunchAgents"),
        PathBuf::from("/Library/LaunchAgents"),
        PathBuf::from("/Library/LaunchDaemons"),
    ];

    for dir in &launch_dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if search_terms.iter().any(|term| name.contains(term)) {
                    remnants.push(entry.path());
                }
            }
        }
    }

    remnants
}
