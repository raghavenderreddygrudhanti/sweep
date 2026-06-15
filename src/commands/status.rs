use sysinfo::{System, Disks};
use bytesize::ByteSize;
use std::process::Command;
use std::io::{self, Write};
use crossterm::{terminal, cursor, execute, event};

pub fn run() {
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    let mut sys = System::new_all();
    // First refresh to get baseline
    sys.refresh_all();
    std::thread::sleep(std::time::Duration::from_millis(500));

    loop {
        sys.refresh_all();

        // Move to top and overwrite (no Clear — prevents flicker)
        let _ = execute!(stdout, cursor::MoveTo(0, 0));

        let out = build_status(&sys);
        let _ = stdout.write_all(out.as_bytes());
        // Clear any leftover lines from previous frame
        let _ = stdout.write_all(b"\x1b[J"); // Clear from cursor to end of screen
        let _ = stdout.flush();

        if event::poll(std::time::Duration::from_millis(1000)).unwrap_or(false) {
            if let Ok(event::Event::Key(key)) = event::read() {
                if key.code == event::KeyCode::Char('q') || key.code == event::KeyCode::Esc {
                    break;
                }
            }
        }
    }

    let _ = execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();
}

fn build_status(sys: &System) -> String {
    let cpu_cores = sys.cpus().len();
    let total_cpu: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_cores.max(1) as f32;
    let used_mem = sys.used_memory();
    let total_mem = sys.total_memory();
    let mem_pct = (used_mem as f64 / total_mem as f64 * 100.0) as u64;
    let host = System::host_name().unwrap_or_else(|| "Mac".into());
    let uptime = System::uptime();
    let days = uptime / 86400;
    let hours = (uptime % 86400) / 3600;
    let health = compute_health(total_cpu, mem_pct);

    let mut o = String::new();

    o.push_str(&format!("\r\n  \x1b[1mStatus\x1b[0m  \x1b[32m●\x1b[0m \x1b[1m{}\x1b[0m  {} · {}cores · {} · up {}d{}h\x1b[K\r\n",
        health, host, cpu_cores, ByteSize::b(total_mem), days, hours));
    o.push_str("  \x1b[90m─────────────────────────────────────\x1b[0m\x1b[K\r\n\r\n");

    // CPU
    o.push_str(&format!("  \x1b[33mCPU\x1b[0m   {} {:>4.0}%\x1b[K\r\n", bar(total_cpu as u64, 100), total_cpu));
    for (i, cpu) in sys.cpus().iter().take(4).enumerate() {
        let u = cpu.cpu_usage();
        o.push_str(&format!("    C{} {} {:>4.0}%\x1b[K\r\n", i+1, bar(u as u64, 100), u));
    }
    if cpu_cores > 4 { o.push_str(&format!("    \x1b[90m(+{} more)\x1b[0m\x1b[K\r\n", cpu_cores - 4)); }
    o.push_str("\x1b[K\r\n");

    // Memory
    o.push_str(&format!("  \x1b[33mMEM\x1b[0m   {} {:>3}%  {}/{}\x1b[K\r\n",
        bar(mem_pct, 100), mem_pct, ByteSize::b(used_mem), ByteSize::b(total_mem)));
    let swap_used = sys.used_swap();
    let swap_total = sys.total_swap();
    if swap_total > 0 {
        let sp = (swap_used as f64 / swap_total as f64 * 100.0) as u64;
        o.push_str(&format!("  \x1b[33mSWP\x1b[0m   {} {:>3}%  {}/{}\x1b[K\r\n",
            bar(sp, 100), sp, ByteSize::b(swap_used), ByteSize::b(swap_total)));
    }
    o.push_str("\x1b[K\r\n");

    // Disk
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        if disk.mount_point().to_string_lossy() == "/" {
            let total = disk.total_space();
            let avail = disk.available_space();
            let used = total - avail;
            let pct = (used as f64 / total as f64 * 100.0) as u64;
            o.push_str(&format!("  \x1b[33mDISK\x1b[0m  {} {:>3}%  \x1b[32m{} free\x1b[0m\x1b[K\r\n",
                bar(pct, 100), pct, ByteSize::b(avail)));
        }
    }
    o.push_str("\x1b[K\r\n");

    // Battery
    if let Some(batt) = get_battery_info() {
        o.push_str(&format!("  \x1b[33mBAT\x1b[0m   {} {:>3}%  \x1b[32m{}\x1b[0m · {} cycles\x1b[K\r\n",
            bar(batt.level, 100), batt.level, batt.status, batt.cycles));
        o.push_str("\x1b[K\r\n");
    }

    // Processes
    o.push_str("  \x1b[33mPROCS\x1b[0m\x1b[K\r\n");
    let mut procs: Vec<_> = sys.processes().values()
        .map(|p| (p.name().to_string(), p.cpu_usage(), p.memory()))
        .filter(|(_, cpu, _)| *cpu > 3.0)
        .collect();
    procs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (name, cpu, mem) in procs.iter().take(5) {
        let n: &str = if name.len() > 14 { &name[..14] } else { name.as_str() };
        let hot = if *cpu > 80.0 { " \x1b[31mhot\x1b[0m" } else { "" };
        o.push_str(&format!("    {:<14} {:>5.1}% {:>8}{}\x1b[K\r\n", n, cpu, ByteSize::b(*mem), hot));
    }
    o.push_str("\x1b[K\r\n");

    o.push_str("  \x1b[90m─────────────────────────────────────\x1b[0m\x1b[K\r\n");
    o.push_str("  \x1b[90mq quit · refreshes every 1s\x1b[0m\x1b[K\r\n");

    o
}

fn bar(value: u64, max: u64) -> String {
    let width: usize = 15;
    let filled = (value as f64 / max as f64 * width as f64) as usize;
    let empty = width.saturating_sub(filled);
    let b = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
    if value > 80 { format!("\x1b[31m{}\x1b[0m", b) }
    else if value > 60 { format!("\x1b[33m{}\x1b[0m", b) }
    else { format!("\x1b[32m{}\x1b[0m", b) }
}

fn compute_health(cpu: f32, mem: u64) -> u64 {
    let mut s = 100u64;
    if cpu > 80.0 { s -= 20; } else if cpu > 50.0 { s -= 10; }
    if mem > 85 { s -= 20; } else if mem > 70 { s -= 10; }
    s
}

struct BatteryInfo { level: u64, status: String, cycles: u64 }

fn get_battery_info() -> Option<BatteryInfo> {
    let output = Command::new("pmset").args(["-g", "batt"]).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let level = stdout.split_whitespace()
        .find(|s| s.ends_with("%;") || s.ends_with('%'))
        .and_then(|s| s.trim_end_matches(|c| c == '%' || c == ';').parse::<u64>().ok())
        .unwrap_or(0);
    let status = if stdout.contains("charging") { "Charging" }
        else if stdout.contains("charged") { "Charged" }
        else { "Battery" };
    let sp = Command::new("system_profiler").args(["SPPowerDataType"]).output().ok()?;
    let sp_out = String::from_utf8_lossy(&sp.stdout);
    let cycles = sp_out.lines()
        .find(|l| l.contains("Cycle Count"))
        .and_then(|l| l.split_whitespace().last())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    Some(BatteryInfo { level, status: status.to_string(), cycles })
}
