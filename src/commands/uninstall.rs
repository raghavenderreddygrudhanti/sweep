use bytesize::ByteSize;
use colored::*;
use crate::cleaners::apps;

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    println!("\n  {} {}\n", "🗑  App Uninstaller".bold().red(), mode.yellow());

    let apps_list = apps::find_installed_apps();

    for (i, app) in apps_list.iter().take(20).enumerate() {
        let remnants = apps::find_app_remnants(app);
        let size_colored = if app.size > 2_000_000_000 {
            ByteSize::b(app.size).to_string().red()
        } else if app.size > 500_000_000 {
            ByteSize::b(app.size).to_string().yellow()
        } else {
            ByteSize::b(app.size).to_string().normal()
        };

        let extra = if !remnants.is_empty() {
            format!(" \x1b[90m+{} leftovers\x1b[0m", remnants.len())
        } else { "".into() };

        println!("  {:>2}. {:>9}  {}{}",
            (i + 1).to_string().dimmed(),
            size_colored.bold(),
            app.name.white(),
            extra);
    }

    let total: u64 = apps_list.iter().map(|a| a.size).sum();
    println!("\n  {}", "─".repeat(40).dimmed());
    println!("  {} apps · Total: {}\n", apps_list.len(), ByteSize::b(total).to_string().bold());

    super::footer::wait_for_key();
}
