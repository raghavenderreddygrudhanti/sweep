# Contributing to Sweep

Thanks for your interest in contributing! Sweep is an open-source project and we welcome contributions of all kinds.

## Quick Start

```bash
git clone https://github.com/raghavenderreddygrudhanti/sweep.git
cd sweep
cargo build --release
./target/release/sweep
```

## How to Contribute

### Bug fixes and small improvements
Send a pull request directly. No need to open an issue first.

### New features
Open an issue first to discuss the approach. This avoids wasted effort.

### Good first issues
Look for issues labeled `good-first-issue`. These are well-scoped tasks for new contributors.

## Project Structure

- `src/main.rs` — CLI definition (uses clap)
- `src/scanner/` — Filesystem scanning (parallel, uses walkdir + rayon)
- `src/cleaners/` — Each cleaner module knows about specific junk types
- `src/commands/` — UI rendering and user interaction (crossterm TUI)
- `src/cache.rs` — Size cache for instant disk analyzer loads
- `src/history.rs` — Operation log for undo support

## Adding a New Cleaner

1. Create `src/cleaners/your_cleaner.rs`
2. Define paths/patterns it knows about
3. Add `pub mod your_cleaner;` to `src/cleaners/mod.rs`
4. Create a command in `src/commands/` or integrate into existing one
5. Add tests if applicable

Example cleaner structure:
```rust
// src/cleaners/jetbrains.rs
use std::path::PathBuf;
use dirs;

pub fn cache_paths() -> Vec<(PathBuf, &'static str)> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        (home.join("Library/Caches/JetBrains"), "JetBrains IDE caches"),
        (home.join(".local/share/JetBrains"), "JetBrains data (Linux)"),
    ]
    .into_iter()
    .filter(|(p, _)| p.exists())
    .collect()
}
```

## Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Keep functions short and focused
- Use descriptive names
- Add `///` doc comments on public functions
- Handle errors gracefully (no unwrap in production paths)

## Testing

```bash
cargo test
```

Always test with `--dry-run` before testing actual deletion:
```bash
./target/release/sweep clean --dry-run
```

## Pull Request Guidelines

1. One logical change per PR
2. Keep the PR description short — explain what and why
3. Run `cargo build --release` to verify it compiles
4. Test on your machine before submitting
5. Screenshots welcome for UI changes

## Areas We Need Help

| Area | Description |
|---|---|
| 🐧 Linux paths | Add Linux-specific cache/config paths to all cleaners |
| 📦 Packaging | Homebrew formula, AUR, Debian package |
| 🧹 JetBrains cleaner | IntelliJ, WebStorm, PyCharm caches |
| 🧹 Xcode cleaner | DerivedData, archives, simulators |
| 🧹 VS Code cleaner | Extensions, cache, user data |
| 🧹 Android Studio | Gradle cache, AVD images, SDK |
| 🎨 TUI polish | Better color themes, responsive layout |
| 🧪 Tests | Unit tests for scanner and cleaners |
| 📖 Docs | Video demo, more screenshots |
