//! Docker cleanup — dangling images, unused volumes, build cache.

use std::process::Command;

/// Get Docker disk usage info.
pub fn docker_disk_usage() -> Option<String> {
    let output = Command::new("docker")
        .args(["system", "df"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Clean Docker system (dangling images, build cache, unused volumes).
pub fn docker_prune(dry_run: bool) -> u64 {
    if dry_run {
        println!("  Would run: docker system prune -af --volumes");
        return 0;
    }

    let output = Command::new("docker")
        .args(["system", "prune", "-af", "--volumes"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // Parse "Total reclaimed space: X.Y GB" from output
            if let Some(line) = stdout.lines().find(|l| l.contains("reclaimed")) {
                println!("  {}", line);
            }
            0 // Docker reports its own freed space
        }
        Ok(_) => {
            eprintln!("  ⚠ Docker prune failed");
            0
        }
        Err(_) => {
            eprintln!("  ⚠ Docker not found or not running");
            0
        }
    }
}
