# 🧹 Sweep

**Fast system cleaner for macOS and Linux. 10x faster than shell-based tools.**

Sweep combines CleanMyMac, AppCleaner, DaisyDisk, and iStat Menus into a single 4MB binary — written in Rust, no dependencies, MIT licensed.

```bash
brew install sweep   # coming soon
# or
curl -fsSL https://sweep.dev/install.sh | bash
```

## Why Sweep?

| | Sweep | Mole | CleanMyMac |
|---|---|---|---|
| Price | Free | Free CLI / $$ app | $$$$ subscription |
| Platform | macOS + Linux | macOS only | macOS only |
| Speed | Parallel Rust (3-5s scan) | Shell scripts (30-60s) | Slow GUI |
| AI/ML aware | ✅ HuggingFace, Ollama, torch | ❌ | ❌ |
| Docker cleanup | ✅ | ❌ | ❌ |
| Dependencies | None (single binary) | Shell + Go + Homebrew | Full app bundle |
| License | MIT | GPL-3 | Proprietary |
| Undo/recovery | Moves to Trash (Finder) | Moves to Trash | Proprietary |

## Quick Start

```bash
sweep              # Interactive menu
sweep scan ~       # Analyze disk (interactive explorer)
sweep clean        # Clean system caches
sweep ai           # Clean AI/ML caches (HuggingFace, Ollama)
sweep dev          # Clean build artifacts (node_modules, target)
sweep docker       # Clean Docker junk
sweep uninstall    # Remove apps + remnants
sweep optimize     # Flush DNS, rebuild caches
sweep installer    # Remove .dmg/.pkg files
sweep status       # Real-time system monitor
```

Add `--dry-run` to any command to preview without deleting.

## Features

### 📊 Disk Analyzer (Interactive)
- Browse directories with arrow keys
- Progressive scanning (shows results as they arrive)
- Multi-select with Space, bulk delete
- Moves to Trash via Finder (recoverable)
- Size cache for instant 2nd load

### 🤖 AI/ML Cache Cleaning (Unique to Sweep)
Finds and cleans:
- HuggingFace models & datasets (~20-100 GB)
- Ollama downloaded models
- PyTorch/TensorFlow caches
- Conda/pip package caches
- LM Studio models

### ⚡ Developer Artifact Cleaning
- `node_modules` older than 7 days
- Cargo `target/` directories
- Python `.venv` environments
- Build/dist directories
- Selectable — choose what to keep

### 💻 Real-time System Monitor
- CPU per-core usage
- Memory + swap
- Disk space
- Battery health + cycles
- Top processes (with "hot" indicator)
- Auto-refreshes every second

### 🗑 App Uninstaller
- Finds all installed apps with sizes
- Detects remnants (preferences, caches, launch agents)
- Shows leftover count per app

### ⚙ System Optimization
- Flush DNS cache
- Rebuild Launch Services
- Refresh Dock & Finder
- Remove .DS_Store files
- Clean browser caches

## Architecture

```
src/
├── main.rs              CLI entry (clap)
├── cache.rs             Size cache (instant 2nd load)
├── history.rs           Operation log (undo support)
├── scanner/
│   └── mod.rs           Parallel filesystem scanner
├── cleaners/
│   ├── system.rs        System cache paths
│   ├── browser.rs       Chrome, Safari, Firefox, Brave, Arc, Edge
│   ├── ai.rs            HuggingFace, Ollama, torch, conda, pip
│   ├── dev.rs           node_modules, target, .venv, build
│   ├── docker.rs        Docker system prune
│   ├── apps.rs          App uninstaller + remnant finder
│   ├── trash.rs         Trash management
│   └── optimize.rs      System optimization tasks
└── commands/
    ├── interactive.rs   Arrow-key menu
    ├── scan.rs          Interactive disk explorer
    ├── clean.rs         System cache cleaner
    ├── ai.rs            AI/ML cache cleaner
    ├── dev.rs           Build artifact cleaner (selectable)
    ├── docker.rs        Docker cleanup
    ├── uninstall.rs     App uninstaller
    ├── optimize.rs      System optimization
    ├── installer.rs     .dmg/.pkg finder
    ├── status.rs        Real-time system monitor
    └── footer.rs        Shared quit/continue prompt
```

## Building

```bash
# Requires Rust 1.70+
cargo build --release
./target/release/sweep
```

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

Easy ways to help:
- 🐧 **Linux support** — add Linux-specific paths and optimizations
- 🧹 **New cleaners** — Xcode, Android Studio, JetBrains, VS Code
- 🎨 **UI improvements** — better TUI rendering, color themes
- 📦 **Packaging** — Homebrew formula, AUR package, Debian .deb
- 🧪 **Tests** — unit tests for cleaners and scanner
- 📖 **Docs** — better examples, screenshots, video demo

## License

MIT — use it however you want.
