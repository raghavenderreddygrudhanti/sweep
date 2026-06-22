//! Orphan detection — finds leftover files from apps that are no longer installed.
//! Checks Application Support, LaunchAgents, Preferences, and dotfiles.

use std::fs;
use std::path::PathBuf;

/// An orphaned item (leftover from a deleted app).
pub struct Orphan {
    pub path: PathBuf,
    pub label: String,
    pub size: u64,
    pub kind: OrphanKind,
    pub age_days: u64,
}

pub enum OrphanKind {
    AppSupport,
    LaunchAgent,
    Preference,
    Dotfile,
}

impl OrphanKind {
    pub fn label(&self) -> &str {
        match self {
            Self::AppSupport => "App Support",
            Self::LaunchAgent => "Launch Agent",
            Self::Preference => "Preference",
            Self::Dotfile => "Dotfile",
        }
    }
}

/// Find orphaned Application Support directories.
/// An orphan = directory in ~/Library/Application Support that doesn't match any installed app.
pub fn find_orphans() -> Vec<Orphan> {
    let home = crate::error::home_or_exit();
    let mut orphans: Vec<Orphan> = Vec::new();

    // Get list of installed app bundle IDs
    let installed_apps = get_installed_app_ids();

    // Check Application Support
    let app_support = home.join("Library/Application Support");
    if app_support.exists() {
        if let Ok(entries) = fs::read_dir(&app_support) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_dir() {
                    continue;
                }

                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                // Skip known system dirs
                if is_system_app_support(&name) {
                    continue;
                }

                // Check if matching app exists
                if !has_matching_app(&name, &installed_apps) {
                    let size = crate::scanner::scan_size_native(&p);
                    if size > 1024 * 1024 {
                        // Only report > 1MB orphans
                        let age = file_age_days(&p);
                        orphans.push(Orphan {
                            path: p,
                            label: name,
                            size,
                            kind: OrphanKind::AppSupport,
                            age_days: age,
                        });
                    }
                }
            }
        }
    }

    // Check LaunchAgents for orphans
    let launch_agents = home.join("Library/LaunchAgents");
    if launch_agents.exists() {
        if let Ok(entries) = fs::read_dir(&launch_agents) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if !name.ends_with(".plist") {
                    continue;
                }

                // Check if the program referenced in the plist exists
                if let Ok(content) = fs::read_to_string(&p) {
                    if let Some(program) = extract_program_path(&content) {
                        if !std::path::Path::new(&program).exists() {
                            let size = p.metadata().map(|m| m.len()).unwrap_or(0);
                            orphans.push(Orphan {
                                path: p,
                                label: format!("{} (binary missing: {})", name, program),
                                size,
                                kind: OrphanKind::LaunchAgent,
                                age_days: file_age_days(&entry.path()),
                            });
                        }
                    }
                }
            }
        }
    }

    // Check orphan dotfiles in home
    if let Ok(entries) = fs::read_dir(&home) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('.') {
                continue;
            }
            if !entry.path().is_dir() {
                continue;
            }
            // Skip common/system dotfiles
            if is_common_dotfile(&name) {
                continue;
            }

            // Check if owning binary exists in PATH
            let bin_name = name.trim_start_matches('.');
            if !binary_in_path(bin_name) && !has_matching_app(bin_name, &installed_apps) {
                let size = crate::scanner::scan_size_native(&entry.path());
                if size > 100 * 1024 {
                    // > 100KB
                    let age = file_age_days(&entry.path());
                    orphans.push(Orphan {
                        path: entry.path(),
                        label: name,
                        size,
                        kind: OrphanKind::Dotfile,
                        age_days: age,
                    });
                }
            }
        }
    }

    orphans.sort_by(|a, b| b.size.cmp(&a.size));
    orphans
}

/// Get list of installed app names/bundle IDs.
fn get_installed_app_ids() -> Vec<String> {
    let mut ids = Vec::new();

    for dir in &[
        "/Applications",
        &format!(
            "{}/Applications",
            dirs::home_dir().unwrap_or_default().display()
        ),
    ] {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry
                    .file_name()
                    .to_string_lossy()
                    .trim_end_matches(".app")
                    .to_lowercase();
                ids.push(name);
            }
        }
    }

    ids
}

fn has_matching_app(name: &str, installed: &[String]) -> bool {
    let lower = name.to_lowercase();
    installed
        .iter()
        .any(|app| app.contains(&lower) || lower.contains(app))
}

fn is_system_app_support(name: &str) -> bool {
    let system = [
        "com.apple",
        "Apple",
        "AddressBook",
        "CallHistoryDB",
        "CloudDocs",
        "CoreData",
        "CrashReporter",
        "Dock",
        "FileProvider",
        "Knowledge",
        "MobileSync",
        "SyncServices",
        "Spotlight",
        "icdd",
        "Quick Look",
        "Accessibility",
    ];
    system.iter().any(|s| name.contains(s))
}

fn is_common_dotfile(name: &str) -> bool {
    let common = [
        ".Trash",
        ".cache",
        ".config",
        ".local",
        ".ssh",
        ".gnupg",
        ".aws",
        ".kube",
        ".docker",
        ".git",
        ".gitconfig",
        ".zshrc",
        ".bashrc",
        ".bash_profile",
        ".profile",
        ".zprofile",
        ".npm",
        ".cargo",
        ".rustup",
        ".gradle",
        ".m2",
        ".vscode",
        ".kiro",
        ".CFUserTextEncoding",
        ".DS_Store",
        ".Spotlight-V100",
        ".fseventsd",
    ];
    common.contains(&name)
}

fn binary_in_path(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn extract_program_path(plist_content: &str) -> Option<String> {
    // Simple extraction: find <string>/path/to/binary</string> after ProgramArguments
    if let Some(start) = plist_content.find("<string>/") {
        let rest = &plist_content[start + 8..];
        if let Some(end) = rest.find("</string>") {
            let path = &rest[..end];
            if path.contains('/') && !path.contains(' ') {
                return Some(path.to_string());
            }
        }
    }
    None
}

fn file_age_days(path: &std::path::Path) -> u64 {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| std::time::SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_secs() / 86400)
        .unwrap_or(0)
}
