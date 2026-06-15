use colored::*;
use crate::cleaners::docker;

pub fn run(dry_run: bool) {
    let mode = if dry_run { "(preview)" } else { "" };
    println!("\n  {} {}\n", "🐳 Docker Cleanup".bold().blue(), mode.yellow());

    match docker::docker_disk_usage() {
        Some(usage) => {
            for line in usage.lines().take(5) {
                println!("  {}", line.dimmed());
            }
            println!();
            if !dry_run {
                docker::docker_prune(false);
                println!("  🎉 Docker cleaned.\n");
            } else {
                println!("  💾 Run `sweep docker` to prune.\n");
            }
        }
        None => {
            println!("  ⚠  Docker not found or not running.\n");
        }
    }

    
}
