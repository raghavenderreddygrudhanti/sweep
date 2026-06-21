//! Comprehensive cleaner — all known cache/junk locations for macOS.
//! Covers everything Mole does + extras Mole misses.

use std::path::PathBuf;

/// A cleanable target with metadata.
pub struct CleanCategory {
    pub name: &'static str,
    pub items: Vec<CleanEntry>,
}

pub struct CleanEntry {
    pub path: PathBuf,
    pub label: &'static str,
    pub safe: bool, // true = auto-clean, false = needs review
}

/// Get all cleanable categories for macOS.
pub fn all_categories() -> Vec<CleanCategory> {
    let home = crate::error::home_or_exit();

    vec![
        // ─── System ───────────────────────────────────────
        CleanCategory {
            name: "System",
            items: vec![
                entry(&home, "Library/Logs/DiagnosticReports", "System crash reports", true),
                entry(&home, "Library/Logs", "System logs", true),
                entry_abs("/var/log", "System daemon logs", true),
                entry_abs("/Library/Logs", "Global app logs", true),
                entry(&home, "Library/Logs/powermanagement", "Power logs", true),
                entry_abs("/System/Library/Caches", "System framework caches", false),
                entry_abs("/Library/Caches", "Global app caches", true),
                // GPU caches (Mole has this)
                entry(&home, "Library/Caches/com.apple.metal", "Metal shader cache", true),
                entry_abs("/private/var/folders", "Temporary system folders", false),
            ],
        },

        // ─── User Essentials ──────────────────────────────
        CleanCategory {
            name: "User essentials",
            items: vec![
                entry(&home, "Library/Caches", "User app caches", true),
                entry(&home, ".Trash", "Trash", true),
                entry(&home, "Library/Saved Application State", "App saved states", true),
                entry(&home, "Library/Cookies", "Cookies", true),
            ],
        },

        // ─── Browsers ─────────────────────────────────────
        CleanCategory {
            name: "Browsers",
            items: vec![
                // Only Library/Caches paths (safe, not profile dirs)
                entry(&home, "Library/Caches/Google/Chrome", "Chrome cache", true),
                entry(&home, "Library/Caches/com.apple.Safari", "Safari cache", true),
                entry(&home, "Library/Caches/Firefox", "Firefox cache", true),
                entry(&home, "Library/Caches/BraveSoftware/Brave-Browser", "Brave cache", true),
                entry(&home, "Library/Caches/company.thebrowser.Browser", "Arc cache", true),
                entry(&home, "Library/Caches/Microsoft Edge", "Edge cache", true),
                // Google updater
                entry(&home, "Library/Google/GoogleSoftwareUpdate", "Google Updater cache", true),
                // Chrome old versions (Mole has this)
                entry(&home, "Library/Caches/com.google.SoftwareUpdate", "Chrome update cache", true),
            ],
        },

        // ─── Cloud & Office ───────────────────────────────
        CleanCategory {
            name: "Cloud & Office",
            items: vec![
                entry(&home, "Library/Caches/com.microsoft.teams", "Teams cache", true),
                entry(&home, "Library/Caches/com.microsoft.outlook", "Outlook cache", true),
                entry(&home, "Library/Caches/com.microsoft.Word", "Word cache", true),
                entry(&home, "Library/Caches/com.microsoft.Excel", "Excel cache", true),
                entry(&home, "Library/Caches/com.microsoft.Powerpoint", "PowerPoint cache", true),
                entry(&home, "Library/Caches/com.microsoft.onenote.mac", "OneNote cache", true),
                entry(&home, "Library/Caches/com.apple.icloud", "iCloud cache", true),
                entry(&home, "Library/Caches/CloudKit", "CloudKit cache", true),
                entry(&home, "Library/Group Containers/UBF8T346G9.OneDriveSyncClientSuite", "OneDrive cache", false),
                entry(&home, "Library/Caches/com.tinyspeck.slackmacgap", "Slack cache", true),
                entry(&home, "Library/Caches/us.zoom.xos", "Zoom cache", true),
                entry(&home, "Library/Application Support/Zoom/data", "Zoom recordings cache", false),
            ],
        },

        // ─── Developer Tools ──────────────────────────────
        CleanCategory {
            name: "Developer tools",
            items: vec![
                // Node/npm
                entry(&home, ".npm/_cacache", "npm cache", true),
                entry(&home, ".npm/_npx", "npx cache", true),
                entry(&home, ".npm/_logs", "npm logs", true),
                entry(&home, ".cache/yarn", "Yarn cache", true),
                entry(&home, "Library/Caches/pnpm", "pnpm cache", true),
                entry(&home, ".bun/install/cache", "Bun cache", true),
                // Python
                entry(&home, "Library/Caches/pip", "pip cache", true),
                entry(&home, ".cache/pip", "pip cache (Linux-style)", true),
                entry(&home, ".cache/uv", "uv cache", true),
                entry(&home, ".conda/pkgs", "Conda packages", true),
                entry(&home, "miniconda3/pkgs", "Miniconda packages", true),
                entry(&home, "anaconda3/pkgs", "Anaconda packages", true),
                // Rust
                entry(&home, ".cargo/registry/cache", "Cargo registry cache", true),
                entry(&home, ".cargo/registry/src", "Cargo registry source", true),
                entry(&home, ".rustup/downloads", "Rustup downloads", true),
                // Go
                entry(&home, "Library/Caches/go-build", "Go build cache", true),
                entry(&home, "go/pkg/mod/cache", "Go module cache", true),
                // Java/JVM
                entry(&home, ".gradle/caches", "Gradle caches", true),
                entry(&home, ".gradle/wrapper/dists", "Gradle wrapper dists", true),
                entry(&home, ".m2/repository", "Maven repository", true),
                // Ruby
                entry(&home, ".gem/cache", "Ruby gem cache", true),
                entry(&home, "Library/Caches/CocoaPods", "CocoaPods cache", true),
                // iOS/macOS
                entry(&home, "Library/Developer/Xcode/DerivedData", "Xcode DerivedData", true),
                entry(&home, "Library/Developer/Xcode/Archives", "Xcode Archives", false),
                entry(&home, "Library/Developer/CoreSimulator/Caches", "Simulator caches", true),
                entry(&home, "Library/Developer/CoreSimulator/Devices", "Simulator devices", false),
                // Homebrew
                entry_abs("/opt/homebrew/Caches", "Homebrew cache", true),
                entry(&home, "Library/Caches/Homebrew", "Homebrew download cache", true),
                // Docker
                entry(&home, "Library/Containers/com.docker.docker/Data/cache", "Docker cache", true),
                entry(&home, ".docker/buildx", "Docker BuildX cache", true),
            ],
        },

        // ─── AI/ML Tools ──────────────────────────────────
        // (Sweep unique — Mole doesn't cover all of these)
        CleanCategory {
            name: "AI/ML tools",
            items: vec![
                entry(&home, ".cache/huggingface/hub", "HuggingFace models", false),
                entry(&home, ".ollama/models", "Ollama models", false),
                entry(&home, ".cache/torch", "PyTorch cache", true),
                entry(&home, ".cache/lm_studio", "LM Studio cache", true),
                entry(&home, ".lmstudio/models", "LM Studio models", false),
                entry(&home, ".cache/whisper", "Whisper models", true),
                entry(&home, ".keras/models", "Keras models", true),
                // AI IDE tools (Sweep unique!)
                entry(&home, "Library/Caches/com.todesktop.230313mzl4w4u92", "Cursor cache", true),
                entry(&home, ".cursor/extensions", "Cursor old extensions", false),
                entry(&home, ".continue/models", "Continue.dev models", true),
                entry(&home, ".codex", "Codex CLI cache", true),
                entry(&home, "Library/Application Support/Claude/Cache", "Claude cache", true),
                entry(&home, "Library/Application Support/Claude/Sentry", "Claude sentry", true),
                entry(&home, ".cache/copilot", "GitHub Copilot cache", true),
            ],
        },

        // ─── IDEs & Editors ───────────────────────────────
        // (Sweep unique — Mole only does basic)
        CleanCategory {
            name: "IDEs & Editors",
            items: vec![
                entry(&home, "Library/Caches/JetBrains", "JetBrains caches", true),
                entry(&home, "Library/Caches/com.microsoft.VSCode", "VS Code cache", true),
                entry(&home, ".vscode/extensions/.obsolete", "VS Code old extensions", true),
                entry(&home, "Library/Caches/com.sublimetext.4", "Sublime Text cache", true),
                entry(&home, "Library/Caches/com.todesktop.230313mzl4w4u92", "Cursor cache", true),
                entry(&home, "Library/Caches/com.github.Electron", "Electron cache", true),
                entry(&home, "Library/Caches/Kiro", "Kiro cache", true),
            ],
        },

        // ─── Media & Creative ─────────────────────────────
        // (Sweep unique — Mole doesn't cover these)
        CleanCategory {
            name: "Media & Creative",
            items: vec![
                entry(&home, "Library/Caches/com.spotify.client", "Spotify cache", true),
                entry(&home, "Library/Caches/com.apple.Music", "Apple Music cache", true),
                entry(&home, "Library/Caches/com.apple.podcasts", "Podcasts cache", true),
                entry(&home, "Library/Caches/com.apple.photolibraryd", "Photos cache", true),
                entry(&home, "Movies/iMovie Theater.theater", "iMovie theater", false),
                entry(&home, "Library/Application Support/Spotify/PersistentCache", "Spotify offline cache", true),
            ],
        },

        // ─── Communication ────────────────────────────────
        // (Sweep unique)
        CleanCategory {
            name: "Communication",
            items: vec![
                entry(&home, "Library/Caches/com.tinyspeck.slackmacgap", "Slack cache", true),
                entry(&home, "Library/Caches/com.hnc.Discord", "Discord cache", true),
                entry(&home, "Library/Application Support/discord/Cache", "Discord media cache", true),
                entry(&home, "Library/Caches/us.zoom.xos", "Zoom cache", true),
                entry(&home, "Library/Caches/com.apple.MobileSMS", "Messages cache", true),
                entry(&home, "Library/Caches/com.apple.FaceTime", "FaceTime cache", true),
            ],
        },

        // ─── Time Machine ─────────────────────────────────
        CleanCategory {
            name: "Time Machine",
            items: vec![
                entry_abs("/Volumes/com.apple.TimeMachine.localsnapshots", "Local snapshots", false),
                entry(&home, "Library/Application Support/MobileSync/Backup", "iOS device backups", false),
            ],
        },

        // ─── Installers & Downloads ───────────────────────
        CleanCategory {
            name: "Installers & Downloads",
            items: vec![
                // We'll handle this with file extension scanning in the command
            ],
        },

        // ─── Misc System ──────────────────────────────────
        // (Sweep unique)
        CleanCategory {
            name: "Misc system",
            items: vec![
                entry(&home, "Library/Application Support/CrashReporter", "Crash reports", true),
                entry(&home, "Library/Caches/com.apple.commerce", "App Store cache", true),
                entry(&home, "Library/Caches/com.apple.helpd", "Help cache", true),
                entry(&home, "Library/Caches/com.apple.nsservicescache.plist", "Services cache", true),
                entry(&home, "Library/Caches/com.apple.Spotlight", "Spotlight cache", true),
                entry(&home, "Library/Application Support/com.apple.sharedfilelist", "Shared file lists", true),
            ],
        },
    ]
}

fn entry(home: &PathBuf, relative: &str, label: &'static str, safe: bool) -> CleanEntry {
    CleanEntry {
        path: home.join(relative),
        label,
        safe,
    }
}

fn entry_abs(path: &str, label: &'static str, safe: bool) -> CleanEntry {
    CleanEntry {
        path: PathBuf::from(path),
        label,
        safe,
    }
}
