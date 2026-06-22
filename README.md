# Sweep

**Fast, intelligent system cleaner for macOS and Linux. Written in Rust.**

```bash
cargo install sweep-cli
# or
brew install raghavenderreddygrudhanti/tap/sweep
```

## Why Sweep?

| | Sweep | Mole | CleanMyMac |
|---|---|---|---|
| Price | **Free** | Free CLI / $$ app | $90/year |
| Clean locations | **106** | ~60 | ~80 |
| Platform | **macOS + Linux** | macOS only | macOS only |
| Binary size | **2.9 MB** | ~15 MB | 180 MB |
| Speed | **3-5 seconds** | 30-60 seconds | Slow GUI |
| Duplicate finder | **Yes** | No | Yes |
| Watch mode (bg monitor) | **Yes (unique)** | No | No |
| Timeline (what grew) | **Yes (unique)** | No | No |
| Smart recommendations | **Scored engine** | Basic | Basic |
| Archive old files | **Yes (unique)** | No | No |
| AI analysis (Ollama) | **Optional** | No | No |
| Orphan detection | **Yes** | Yes | Yes |
| Whitelist/protected paths | **Yes** | Yes | No |
| JSON output for scripting | **Yes** | Yes | No |
| Operation audit log | **Yes** | Yes | No |
| Cross-platform | **Yes** | No | No |
| Open source | **MIT** | GPL-3 | Proprietary |

## Quick Start

```bash
sweep              # Interactive menu
sweep clean        # Clean all caches, logs, build artifacts (106 locations)
sweep dupes        # Find duplicate files
sweep recommend    # Smart recommendations with scoring engine
sweep uninstall    # Remove apps + remnants
sweep scan ~       # Disk explorer
sweep timeline     # What grew or shrank since last check
sweep watch        # Background monitor, alert when disk is low
sweep optimize     # Refresh system caches and services
sweep status       # Real-time system health dashboard
sweep --whitelist  # View/manage protected paths
```

Add `--dry-run` to preview without deleting. Add `--json` for scripting output.

## Features

### Clean (106 locations, 11 categories)
- System (crash reports, logs, GPU caches)
- Browsers (Chrome, Safari, Firefox, Brave, Arc, Edge — skips running browsers)
- Cloud & Office (Teams, Outlook, Slack, Zoom, iCloud, OneDrive)
- Developer tools (npm, Yarn, pnpm, Bun, pip, uv, Conda, Cargo, Go, Gradle, Maven, Ruby, Xcode, Docker, Homebrew)
- AI/ML (HuggingFace, Ollama, PyTorch, LM Studio, Cursor, Claude, Codex, Copilot)
- IDEs (JetBrains, VS Code, Sublime, Kiro)
- Media (Spotify, Apple Music, Podcasts, Photos)
- Communication (Discord, Slack, Zoom, Messages, FaceTime)
- Time Machine (local snapshots, iOS backups)
- Misc system (CrashReporter, App Store, Spotlight)
- Orphan detection (leftover app data, broken launch agents, orphan dotfiles)

### Duplicate Finder
- Parallel hashing across all CPU cores
- Content-based (not filename) — zero false positives
- Select folders to scan (Documents, Downloads, Pictures, Photos Library)
- Choose minimum file size (100 KB to 50 MB)
- Keeps newest copy, deletes rest

### Smart Recommendations (Scoring Engine)
```
Score = Size + Age + Regenerability + Safety signals

+30 if > 1 GB
+20 if not accessed 90+ days
+20 if regenerable cache
+15 if app already installed
-40 if recently used
-60 if user-owned
-100 if sensitive (keychains, .ssh)

80+ = SAFE CLEAN | 40-79 = REVIEW | <40 = KEEP
```
- Archive option for old Documents/Desktop files (compress + free space)
- Impact prediction (free space before/after)
- Whitelist enforced as hard pre-filter

### Watch Mode
- Background disk monitor
- Checks every 5 minutes
- macOS notification when space drops below threshold
- Press `q` to stop

### Timeline
- Shows what grew or shrank since last scan
- Parallel scanning with real-time progress
- Checkbox-style TUI

## Architecture

```
src/
├── main.rs              CLI (clap)
├── scanner/             Parallel filesystem scanner (walkdir + rayon)
├── cleaners/            106 clean locations, orphan detection
├── commands/            All UI screens (TUI + progressive)
├── recommend_engine.rs  Scoring algorithm
├── whitelist.rs         Protected paths
├── oplog.rs             Operation audit log
├── cache.rs             Size cache for timeline
└── history.rs           Operation history
```

Single binary, no runtime dependencies. Built with:
- walkdir + rayon (parallel scanning)
- crossterm (TUI)
- sysinfo (system monitoring)
- clap (CLI)

## Safety

- Whitelist: 21 default protected patterns (Documents, .ssh, keychains, etc.)
- Never touches browser profiles (only Library/Caches paths)
- Skips running browsers
- Refuses to run as root (unless --force)
- Symlinks never followed
- Silent skip on permission denied (no password prompts)
- All operations logged to ~/.sweep/operations.log

## License

MIT
