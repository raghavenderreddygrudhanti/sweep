//! Recommend v2 — ranked decision engine.
//! Scores each item based on: size + age + regenerability + safety signals.
//! Whitelist is a hard pre-filter — protected paths can NEVER be scored.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Classification of a path.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemClass {
    Regenerable,  // caches, logs, build folders
    Replaceable,  // installers, old zips, duplicates
    UserOwned,    // documents, photos, projects
    Sensitive,    // browser profiles, cookies, keychains
}

/// Recommended action.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    SafeClean,  // 80+ score
    Review,     // 40-79 score
    Keep,       // <40 score
}

/// A scored recommendation item.
#[derive(Debug, Clone)]
pub struct ScoredItem {
    pub path: PathBuf,
    pub label: String,
    pub size: u64,
    pub score: i32,
    pub action: Action,
    pub class: ItemClass,
    pub reasons: Vec<String>,
    pub age_days: u64,
}

/// Hard-blocked paths — NEVER recommend deleting these.
const NEVER_TOUCH: &[&str] = &[
    "Documents", "Desktop", "Pictures", "Movies", "Music",
    "Library/Keychains", "Library/Cookies", "Library/Safari/LocalStorage",
    "Library/Messages", "Library/Mail", "Library/Calendars",
    ".ssh", ".gnupg", ".aws", ".kube",
];

/// Known regenerable patterns.
const REGENERABLE: &[&str] = &[
    "Library/Caches", "Library/Logs", ".cache", ".gradle/caches",
    ".m2/repository", ".npm/_cacache", ".cargo/registry/cache",
    "Library/Developer/Xcode/DerivedData", ".cache/huggingface",
    ".ollama/models", ".cache/torch", ".cache/pip", ".cache/uv",
    "Library/Caches/pip", "Library/Caches/CocoaPods",
    "Library/Caches/Homebrew", "Library/Caches/go-build",
    ".conda/pkgs", "miniconda3/pkgs", "anaconda3/pkgs",
    ".rustup/downloads", ".cargo/registry/src",
    ".gradle/wrapper/dists", "go/pkg/mod/cache",
    "Library/Developer/CoreSimulator/Caches",
];

/// Score an item and produce a recommendation.
/// Whitelist is a hard pre-filter — if protected, always returns Keep.
pub fn score_item(path: &Path, label: &str, size: u64) -> ScoredItem {
    // HARD PRE-FILTER: whitelist check (cannot be overridden by score)
    if crate::whitelist::is_protected(path) {
        return ScoredItem {
            path: path.to_path_buf(),
            label: label.to_string(),
            size, score: -200,
            action: Action::Keep,
            class: ItemClass::Sensitive,
            reasons: vec!["Protected by whitelist".into()],
            age_days: 0,
        };
    }

    let home = crate::error::home_or_exit();
    let home_str = home.display().to_string();
    let rel_path = path.display().to_string().replace(&home_str, "");
    let rel_path = rel_path.trim_start_matches('/');

    let mut score: i32 = 0;
    let mut reasons: Vec<String> = Vec::new();

    // 1. Classify
    let class = classify(rel_path);

    // 2. Hard safety block
    if class == ItemClass::Sensitive {
        return ScoredItem {
            path: path.to_path_buf(),
            label: label.to_string(),
            size, score: -100,
            action: Action::Keep,
            class,
            reasons: vec!["Sensitive data — never delete".into()],
            age_days: 0,
        };
    }
    if class == ItemClass::UserOwned {
        score -= 60;
        reasons.push("User-owned folder".into());
    }

    // 3. Size scoring
    if size > 5 * 1024 * 1024 * 1024 { // >5GB
        score += 35;
        reasons.push("Very large (>5 GB)".into());
    } else if size > 1024 * 1024 * 1024 { // >1GB
        score += 30;
        reasons.push("Large (>1 GB)".into());
    } else if size > 500 * 1024 * 1024 { // >500MB
        score += 20;
        reasons.push("Moderate size (>500 MB)".into());
    } else if size > 100 * 1024 * 1024 { // >100MB
        score += 10;
        reasons.push("Notable size (>100 MB)".into());
    }

    // 4. Age scoring (last accessed)
    let age_days = get_access_age_days(path);
    if age_days > 180 {
        score += 25;
        reasons.push(format!("Not accessed for {} days", age_days));
    } else if age_days > 90 {
        score += 20;
        reasons.push(format!("Not accessed for {} days", age_days));
    } else if age_days > 30 {
        score += 10;
        reasons.push(format!("Last accessed {} days ago", age_days));
    } else if age_days < 7 {
        score -= 40;
        reasons.push("Recently accessed (last 7 days)".into());
    }

    // 5. Regenerable bonus
    if class == ItemClass::Regenerable {
        score += 20;
        reasons.push("Regenerable (cache/build artifact)".into());
    }

    // 6. App-installed check (for installers)
    if is_installer(path) {
        if installer_app_exists(path) {
            score += 15;
            reasons.push("App already installed".into());
        } else {
            score -= 10;
            reasons.push("App not found — might need installer".into());
        }
    }

    // 7. Git-ignored pattern check
    if is_in_gitignore(path) {
        score += 15;
        reasons.push("Known build artifact pattern (regenerable)".into());
    }

    // Determine action
    let action = if score >= 80 {
        Action::SafeClean
    } else if score >= 40 {
        Action::Review
    } else {
        Action::Keep
    };

    ScoredItem {
        path: path.to_path_buf(),
        label: label.to_string(),
        size, score, action, class, reasons, age_days,
    }
}

/// Classify a relative path.
fn classify(rel_path: &str) -> ItemClass {
    // Check hard blocks first
    for blocked in NEVER_TOUCH {
        if rel_path.starts_with(blocked) {
            if rel_path.contains("Keychains") || rel_path.contains("Cookies")
                || rel_path.contains("Sessions") || rel_path.contains(".ssh")
                || rel_path.contains(".gnupg") {
                return ItemClass::Sensitive;
            }
            return ItemClass::UserOwned;
        }
    }

    // Check regenerable patterns
    for pattern in REGENERABLE {
        if rel_path.contains(pattern) {
            return ItemClass::Regenerable;
        }
    }

    // Installers in Downloads
    if rel_path.contains("Downloads") {
        return ItemClass::Replaceable;
    }

    // Default: user-owned (conservative)
    ItemClass::UserOwned
}

/// Get last access time in days.
fn get_access_age_days(path: &Path) -> u64 {
    // Try atime first, fall back to mtime
    path.metadata()
        .ok()
        .and_then(|m| m.accessed().ok().or_else(|| m.modified().ok()))
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_secs() / 86400)
        .unwrap_or(0)
}

/// Check if path is an installer file.
fn is_installer(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext, "dmg" | "pkg" | "iso" | "app")
}

/// Check if the installer's corresponding app is installed.
fn installer_app_exists(path: &Path) -> bool {
    let name = path.file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    // Strip common suffixes like "-installer", "_setup", version numbers
    let clean_name = name
        .replace("-installer", "")
        .replace("_installer", "")
        .replace("-setup", "")
        .replace("_setup", "");

    // Check /Applications
    if let Ok(entries) = std::fs::read_dir("/Applications") {
        for entry in entries.flatten() {
            let app_name = entry.file_name().to_string_lossy().to_lowercase();
            if app_name.contains(&clean_name.to_lowercase()) {
                return true;
            }
        }
    }
    false
}

/// Check if path is a commonly-gitignored build artifact (by name pattern).
/// NOTE: Does NOT read actual .gitignore files. Awards bonus only for
/// well-known regenerable directory names.
fn is_in_gitignore(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(name, "node_modules" | "target" | ".venv" | "__pycache__"
        | "dist" | "build" | ".next" | ".nuxt" | "vendor")
}

/// Action label for display.
impl Action {
    pub fn label(&self) -> &str {
        match self {
            Action::SafeClean => "SAFE CLEAN",
            Action::Review => "REVIEW",
            Action::Keep => "KEEP",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            Action::SafeClean => "\x1b[32m",  // green
            Action::Review => "\x1b[33m",     // yellow
            Action::Keep => "\x1b[90m",       // gray
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Action::SafeClean => "\x1b[32m\u{25cf}\x1b[0m",
            Action::Review => "\x1b[33m\u{25cf}\x1b[0m",
            Action::Keep => "\x1b[90m\u{25cf}\x1b[0m",
        }
    }
}

impl ItemClass {
    pub fn label(&self) -> &str {
        match self {
            ItemClass::Regenerable => "Regenerable",
            ItemClass::Replaceable => "Replaceable",
            ItemClass::UserOwned => "User-owned",
            ItemClass::Sensitive => "Sensitive",
        }
    }
}
