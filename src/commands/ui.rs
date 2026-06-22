/// Shared UI components for consistent branding across all screens.
/// All screens should use these for uniform look, navigation, and footers.
///
/// COLOR CODE REFERENCE (used consistently across all screens):
/// - Green  (\x1b[32m) — success, freed space, healthy, deletable items
/// - Yellow (\x1b[33m) — warning, in-progress, scanning, moderate
/// - Red    (\x1b[31m) — critical, large usage, growth, errors
/// - Cyan   (\x1b[36m) — selected/highlighted item
/// - Gray   (\x1b[90m) — disabled, system/protected, hints, unchanged
/// - Bold   (\x1b[1m)  — emphasis, totals, sizes
use crossterm::event::{KeyCode, KeyEvent};

pub const REPO: &str = "github.com/raghavenderreddygrudhanti/sweep";

// ─── Standard Navigation ────────────────────────────────────────────────────

/// Standard key action result for all TUI screens.
#[derive(Debug, Clone, PartialEq)]
pub enum NavAction {
    Up,
    Down,
    Select,     // Enter or Right arrow — open/confirm
    Back,       // Esc, Left, Backspace — go back
    Quit,       // q — exit to main menu
    Toggle,     // Space — toggle selection
    Delete,     // d/D/Delete — delete
    SelectAll,  // a — select all
    ClearAll,   // n — clear selection
    Char(char), // Any other char
    None,       // Unknown/unhandled key
}

/// Map a key event to a standard NavAction.
/// Use this in every TUI screen instead of duplicating match blocks.
pub fn map_key(key: KeyEvent) -> NavAction {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => NavAction::Up,
        KeyCode::Down | KeyCode::Char('j') => NavAction::Down,
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => NavAction::Select,
        KeyCode::Esc | KeyCode::Backspace => NavAction::Back,
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('b') => NavAction::Back,
        KeyCode::Char('q') => NavAction::Quit,
        KeyCode::Char(' ') => NavAction::Toggle,
        KeyCode::Char('d') | KeyCode::Char('D') => NavAction::Delete,
        KeyCode::Delete => NavAction::Delete,
        KeyCode::Char('a') => NavAction::SelectAll,
        KeyCode::Char('n') => NavAction::ClearAll,
        KeyCode::Char(c) => NavAction::Char(c),
        _ => NavAction::None,
    }
}

// ─── Standard Footers ───────────────────────────────────────────────────────

/// Standard footer separator line (TUI).
pub fn footer_sep() -> &'static str {
    "  \x1b[90m─────────────────────────────────────────────\x1b[0m\r\n"
}

/// Footer for browse/explorer screens (TUI).
pub fn footer_browse() -> &'static str {
    "  \x1b[90m\u{2191}\u{2193} nav \u{b7} Enter open \u{b7} b back \u{b7} Space select \u{b7} d del \u{b7} q quit\x1b[0m\r\n"
}

/// Footer showing selected count + actions (TUI).
pub fn footer_selected(count: usize) -> String {
    format!("  \x1b[32m{} selected\x1b[0m \u{b7} \x1b[90mD delete \u{b7} Space toggle \u{b7} n clear \u{b7} b back \u{b7} q quit\x1b[0m\r\n", count)
}

/// Footer for list screens with selection (TUI).
pub fn footer_list() -> &'static str {
    "  \x1b[90m\u{2191}\u{2193} nav \u{b7} Space select \u{b7} d delete \u{b7} a all \u{b7} b back \u{b7} q quit\x1b[0m\r\n"
}

/// Footer for simple view screens (TUI).
pub fn footer_simple() -> &'static str {
    "  \x1b[90mb back \u{b7} q quit\x1b[0m\r\n"
}

/// Footer for non-TUI screens — wait for key to return.
pub fn wait_any_key() {
    println!("  \x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m");
    print!("  \x1b[90mPress any key to return...\x1b[0m ");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let _ = crossterm::terminal::enable_raw_mode();
    std::thread::sleep(std::time::Duration::from_millis(200));
    while crossterm::event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
        let _ = crossterm::event::read();
    }
    let _ = crossterm::event::read();
    let _ = crossterm::terminal::disable_raw_mode();
    println!();
}

// ─── Fancy Operation Names ──────────────────────────────────────────────────

pub fn action_name(op: &str) -> &'static str {
    match op {
        "clean" => "🧹 Sweeping away junk...",
        "delete" | "trash" => "🗑  Tossing to the void...",
        "scan" => "🔍 Hunting disk hogs...",
        "uninstall" => "💀 Evicting app...",
        "optimize" => "⚡ Turbocharging system...",
        "ai" => "🤖 Purging AI leftovers...",
        "docker" => "🐳 Draining containers...",
        "dev" => "🔨 Demolishing build artifacts...",
        "installer" => "📦 Shredding installers...",
        _ => "⏳ Working...",
    }
}

// ─── Spinners & Animation ───────────────────────────────────────────────────

const SPINNERS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SWEEP_FRAMES: &[&str] = &[
    "🧹    ", " 🧹   ", "  🧹  ", "   🧹 ", "    🧹", "   🧹 ", "  🧹  ", " 🧹   ",
];

pub fn spinner(frame: usize) -> &'static str {
    SPINNERS[frame % SPINNERS.len()]
}

pub fn sweep_anim(frame: usize) -> &'static str {
    SWEEP_FRAMES[frame % SWEEP_FRAMES.len()]
}

// ─── Logo & Headers ─────────────────────────────────────────────────────────

pub fn logo_tui_animated(frame: usize) -> String {
    let colors = ["\x1b[36m", "\x1b[32m", "\x1b[35m", "\x1b[33m", "\x1b[34m"];
    let c = colors[frame % colors.len()];
    let r = "\x1b[0m";

    let mut s = String::new();
    s.push_str("\r\n");
    s.push_str(&format!("    {}____{}  \r\n", c, r));
    s.push_str(&format!("   {}/ ___|\x1b[0m_      _____  ___ _ __\r\n", c));
    s.push_str(&format!(
        "   {}\\___ \\{}\\  \\ /\\ / / _ \\/ _ \\ '_ \\\r\n",
        c, r
    ));
    s.push_str(&format!(
        "    {}___) |{}\\  V  V /  __/  __/ |_) |\r\n",
        c, r
    ));
    s.push_str(&format!(
        "   {}|____/{}  \\_/\\_/ \\___|\\___| .__/\r\n",
        c, r
    ));
    s.push_str("                           |_|\r\n");
    s.push_str(&format!("   \x1b[32m{}\x1b[0m\r\n", REPO));
    s.push_str("   \x1b[90mFast system cleaner · Rust · macOS + Linux\x1b[0m\r\n");
    s
}

fn logo_tui() -> String {
    logo_tui_animated(0)
}

fn logo_print() {
    println!();
    println!("    \x1b[36m____\x1b[0m");
    println!("   \x1b[36m/ ___|\x1b[0m_      _____  ___ _ __");
    println!("   \x1b[36m\\___ \\\x1b[0m\\ \\ /\\ / / _ \\/ _ \\ '_ \\");
    println!("    \x1b[36m___) |\x1b[0m\\ V  V /  __/  __/ |_) |");
    println!("   \x1b[36m|____/\x1b[0m  \\_/\\_/ \\___|\\___| .__/");
    println!("                           |_|");
    println!("   \x1b[32m{}\x1b[0m", REPO);
    println!("   \x1b[90mFast system cleaner · Rust · macOS + Linux\x1b[0m");
}

pub fn print_header(subtitle: &str) {
    logo_print();
    println!();
    println!("  \x1b[1m>\x1b[0m  {}", subtitle);
    println!("  \x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m");
    println!();
}

pub fn tui_header(subtitle: &str) -> String {
    let mut out = logo_tui();
    out.push_str("\r\n");
    if !subtitle.is_empty() {
        out.push_str(&format!("  \x1b[90m›\x1b[0m  {}\r\n", subtitle));
    }
    out.push_str("  \x1b[90m─────────────────────────────────────────────\x1b[0m\r\n");
    out.push_str("\r\n");
    out
}

pub fn tui_header_animated(subtitle: &str, frame: usize) -> String {
    let mut out = logo_tui_animated(frame);
    out.push_str("\r\n");
    if !subtitle.is_empty() {
        out.push_str(&format!("  \x1b[90m›\x1b[0m  {}\r\n", subtitle));
    }
    out.push_str("  \x1b[90m─────────────────────────────────────────────\x1b[0m\r\n");
    out.push_str("\r\n");
    out
}
