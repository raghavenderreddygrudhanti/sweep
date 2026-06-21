use bytesize::ByteSize;
use colored::*;
use crate::cleaners::optimize;
use crate::cleaners::DeleteMode;

pub fn run(dry_run: bool, _mode: DeleteMode) {
    let mode = if dry_run { "(preview)" } else { "" };
    super::ui::print_header(&format!("\x1b[1;33m\u{1f4e6} Installer Cleanup\x1b[0m {}", mode));

    let installers = optimize::find_installers();

    if installers.is_empty() {
        println!("  ✨ No installer files found.");
        super::ui::wait_any_key();
        return;
    }

    let mut total: u64 = 0;
    for (path, size) in &installers {
        println!("  ✓ {:>9}  {}",
            ByteSize::b(*size).to_string().bold(),
            path.file_name().unwrap_or_default().to_string_lossy().dimmed());
        total += size;
    }

    println!("\n  {}", "─".repeat(40).dimmed());
    if dry_run {
        println!("  💾 {} files · Would free: {}",
            installers.len(), ByteSize::b(total).to_string().bold().green());
    } else {
        for (path, _) in &installers { let _ = std::fs::remove_file(path); }
        println!("  🎉 Freed: {}", ByteSize::b(total).to_string().bold().green());
    }

    super::ui::wait_any_key();
}
