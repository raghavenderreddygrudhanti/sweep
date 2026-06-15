# Issues to Create on GitHub

After pushing, create these issues to attract contributors.

---

## Good First Issues (label: `good-first-issue`)

### 1. Add VS Code extension cache cleaner
VS Code stores old extension versions in `~/.vscode/extensions/`. Add a cleaner that finds and removes older versions while keeping the latest.

### 2. Add Android Studio / Gradle cleaner
Paths: `~/.android/avd/`, `~/.gradle/caches/`, `~/.gradle/wrapper/dists/`, `~/Library/Android/sdk/`

### 3. Add Linux-specific paths to system cleaner
Currently macOS-focused. Add: `~/.cache/`, `/var/cache/`, `~/.local/share/Trash/`, journalctl vacuum, apt/yum cache.

### 4. Add Homebrew formula for easy install
Create a Homebrew tap so users can `brew install raghavender/tap/sweep`.

### 5. Add file age display in disk analyzer
Show ">90d", ">6mo", ">1y" next to old files in the analyzer view, like Mole does.

### 6. Add Conda environment cleaner
Find old conda envs: `~/miniconda3/envs/`, `~/anaconda3/envs/`. Show size, last used date.

### 7. Add whitelist / protected paths
Allow users to protect paths from cleaning via `~/.sweep/whitelist.txt`. Skip these during clean.

---

## Enhancements (label: `enhancement`)

### 8. Add --json output flag
All commands should support `--json` for scripting/automation. Output structured JSON instead of TUI.

### 9. Add filter/search in disk analyzer
Press `/` to type and filter entries by name in the disk analyzer.

### 10. Self-update command
`sweep update` — download latest release from GitHub and replace binary.

### 11. Large file finder (press L in analyzer)
Show top 20 largest files across the system, sorted by size.

### 12. Add AUR package for Arch Linux
Package sweep for Arch Linux users.

### 13. Add Debian/Ubuntu .deb package
Create GitHub release with .deb package for apt install.

---

## Platform (label: `platform`)

### 14. Windows support
Add Windows-specific paths for cache/temp/Downloads cleaning. Use `%APPDATA%`, `%LOCALAPPDATA%`, `%TEMP%`.

### 15. Raspberry Pi / ARM Linux support
Ensure cross-compilation works for arm64 Linux (Raspberry Pi, cloud ARM instances).
