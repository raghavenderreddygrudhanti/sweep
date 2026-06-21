//! Developer build artifact cleaner.
//! Finds and removes: node_modules, target/, .venv, __pycache__, .build, dist

use std::path::PathBuf;

/// Developer artifact directories to look for.
pub const DEV_ARTIFACTS: &[&str] = &[
    "node_modules",
    "target",
    ".venv",
    "venv",
    "__pycache__",
    ".build",
    "build",
    "dist",
    ".next",
    ".nuxt",
    ".output",
    ".parcel-cache",
    ".turbo",
];

/// Default root paths to scan for dev artifacts.
pub fn scan_roots() -> Vec<PathBuf> {
    let home = crate::error::home_or_exit();
    vec![
        home.join("lang-chain"),
        home.join("Projects"),
        home.join("Developer"),
        home.join("dev"),
        home.join("code"),
        home.join("workspace"),
        home.join("repos"),
        home.join("src"),
        home.join("git"),
        home.join("work"),
        home.join("agno-projects"),
    ]
    .into_iter()
    .filter(|p| p.exists())
    .collect()
}
