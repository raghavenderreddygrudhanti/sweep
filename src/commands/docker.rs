use colored::*;
use crate::cleaners::docker;

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    super::ui::print_header(&format!("\x1b[1;34m\u{1f433} Docker Cleanup\x1b[0m {}", mode));

    match docker::docker_disk_usage() {
        Some(usage) => {
            for line in usage.lines().take(5) {
                println!("  {}", line.dimmed());
            }
            println!();
            if !dry_run {
                docker::docker_prune(false);
                println!("  🎉 Docker cleaned.");
            } else {
                println!("  💾 Run `sweep docker` to prune.");
            }
        }
        None => {
            println!("  ⚠  Docker not found or not running.");
        }
    }

    super::ui::wait_any_key();
}
