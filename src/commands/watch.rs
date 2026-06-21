//! Watch mode — background monitor that alerts when disk space is low.
//! Checks every 5 minutes and shows macOS notification when threshold crossed.

use std::io::{self, Write};
use bytesize::ByteSize;
use sysinfo::Disks;

pub fn run(threshold_gb: u64) {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();

    super::ui::print_header("\x1b[1;36mWatch Mode\x1b[0m");

    let threshold_bytes = threshold_gb * 1024 * 1024 * 1024;
    println!("  Monitoring disk space...");
    println!("  Alert threshold: \x1b[33m{}\x1b[0m free", ByteSize::b(threshold_bytes));
    println!("  Check interval: every 5 minutes");
    println!();
    println!("  \x1b[90mPress Ctrl+C to stop\x1b[0m\n");

    let mut last_alert = false;
    let mut checks: u64 = 0;

    loop {
        let free = get_free_space();
        checks += 1;

        let pct_free = if let Some(total) = get_total_space() {
            (free as f64 / total as f64 * 100.0) as u64
        } else { 100 };

        let status = if free < threshold_bytes {
            if !last_alert {
                // Send macOS notification
                send_notification(free);
                last_alert = true;
            }
            format!("\x1b[31m\u{26a0} LOW: {} free ({}%)\x1b[0m", ByteSize::b(free), pct_free)
        } else {
            last_alert = false;
            format!("\x1b[32m\u{2713} OK: {} free ({}%)\x1b[0m", ByteSize::b(free), pct_free)
        };

        // Update status line
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        print!("\r\x1b[K  [{}] Check #{}: {}", timestamp, checks, status);
        let _ = io::stdout().flush();

        // Sleep 5 minutes (check for Ctrl+C via crossterm)
        for _ in 0..300 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            // Check if user wants to quit
            if crossterm::event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
                if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                    if key.code == crossterm::event::KeyCode::Char('q')
                        || key.code == crossterm::event::KeyCode::Char('c')
                        || key.code == crossterm::event::KeyCode::Esc {
                        println!("\n\n  \x1b[90mWatch stopped.\x1b[0m\n");
                        return;
                    }
                }
            }
        }
    }
}

/// Send a macOS notification.
fn send_notification(free: u64) {
    let msg = format!("Disk space low! Only {} free. Run 'sweep clean' to free space.",
        ByteSize::b(free));

    let _ = std::process::Command::new("osascript")
        .args(["-e", &format!(
            "display notification \"{}\" with title \"Sweep\" subtitle \"Low Disk Space\"",
            msg
        )])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status();
}

fn get_free_space() -> u64 {
    let disks = Disks::new_with_refreshed_list();
    disks.list().iter()
        .find(|d| d.mount_point().to_string_lossy() == "/")
        .map(|d| d.available_space())
        .unwrap_or(0)
}

fn get_total_space() -> Option<u64> {
    let disks = Disks::new_with_refreshed_list();
    disks.list().iter()
        .find(|d| d.mount_point().to_string_lossy() == "/")
        .map(|d| d.total_space())
}
