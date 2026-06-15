use bytesize::ByteSize;
use colored::*;
use crate::cleaners::optimize;

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    println!("\n  {} {}\n", "📦 Installer Cleanup".bold().yellow(), mode.yellow());

    let installers = optimize::find_installers();

    if installers.is_empty() {
        println!("  ✨ No installer files found.\n");
        
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
        println!("  💾 {} files · Would free: {}\n",
            installers.len(), ByteSize::b(total).to_string().bold().green());
    } else {
        for (path, _) in &installers { let _ = std::fs::remove_file(path); }
        println!("  🎉 Freed: {}\n", ByteSize::b(total).to_string().bold().green());
    }

    
}
