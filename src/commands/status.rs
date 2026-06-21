use sysinfo::{System, Disks};
use bytesize::ByteSize;
use std::process::Command;
use std::io::{self, Write};
use crossterm::{terminal, cursor, execute, event};

pub fn run() {
    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide);

    // Show loading immediately so screen isn't blank
    let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All));
    let mut loading = super::ui::tui_header("\x1b[33mSystem Status\x1b[0m");
    loading.push_str("  \x1b[33mLoading system info...\x1b[0m\r\n");
    let _ = stdout.write_all(loading.as_bytes());
    let _ = stdout.flush();

    let mut sys = System::new_all();
    sys.refresh_all();
    std::thread::sleep(std::time::Duration::from_millis(500));
    sys.refresh_all();

    loop {
        sys.refresh_all();
        let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All));

        let out = build_status(&sys);
        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();

        if event::poll(std::time::Duration::from_millis(1000)).unwrap_or(false) {
            if let Ok(event::Event::Key(key)) = event::read() {
                match key.code {
                    event::KeyCode::Char('q') | event::KeyCode::Char('b')
                    | event::KeyCode::Esc => break,
                    _ => {}
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
    let os_ver = System::os_version().unwrap_or_else(|| "?".into());
    let uptime = System::uptime();
    let days = uptime / 86400;
    let hours = (uptime % 86400) / 3600;
    let health = compute_health(total_cpu, mem_pct);
    let health_dot = if health >= 80 { "\x1b[32m\u{25cf}\x1b[0m" }
        else if health >= 50 { "\x1b[33m\u{25cf}\x1b[0m" }
        else { "\x1b[31m\u{25cf}\x1b[0m" };

    let mut o = String::new();

    // Compact header
    o.push_str(&super::ui::tui_header("\x1b[33mSystem Status\x1b[0m"));

    // System info line
    o.push_str(&format!("  {} Health \x1b[1m{}\x1b[0m  \x1b[90m{} \u{b7} macOS {} \u{b7} up {}d {}h\x1b[0m\r\n",
        health_dot, health, host, os_ver, days, hours));
    o.push_str("\r\n");

    // ─── CPU ───
    o.push_str("  \x1b[1;33mCPU\x1b[0m\r\n");
    o.push_str(&format!("  Total  {} {:>5.1}%\r\n", bar(total_cpu as u64, 100), total_cpu));
    for (i, cpu) in sys.cpus().iter().take(4).enumerate() {
        let u = cpu.cpu_usage();
        o.push_str(&format!("  Core{} {} {:>5.1}%\r\n", i + 1, bar(u as u64, 100), u));
    }
    if cpu_cores > 4 {
        o.push_str(&format!("  \x1b[90m(+{} more cores)\x1b[0m\r\n", cpu_cores - 4));
    }
    o.push_str("\r\n");

    // ─── Memory ───
    o.push_str("  \x1b[1;33mMemory\x1b[0m\r\n");
    let free_mem = total_mem - used_mem;
    o.push_str(&format!("  Used {} {:>3}%  {}/{}\r\n",
        bar(mem_pct, 100), mem_pct, ByteSize::b(used_mem), ByteSize::b(total_mem)));
    o.push_str(&format!("  Free {} {:>3}%  \x1b[32m{}\x1b[0m\r\n",
        bar_green(100 - mem_pct, 100), 100 - mem_pct, ByteSize::b(free_mem)));
    let swap_used = sys.used_swap();
    let swap_total = sys.total_swap();
    if swap_total > 0 {
        let sp = (swap_used as f64 / swap_total as f64 * 100.0) as u64;
        o.push_str(&format!("  Swap {} {:>3}%  {}/{}\r\n",
            bar(sp, 100), sp, ByteSize::b(swap_used), ByteSize::b(swap_total)));
    }
    o.push_str("\r\n");

    // ─── Disk ───
    o.push_str("  \x1b[1;33mDisk\x1b[0m\r\n");
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        if disk.mount_point().to_string_lossy() == "/" {
            let total = disk.total_space();
            let avail = disk.available_space();
            let used = total - avail;
            let pct = (used as f64 / total as f64 * 100.0) as u64;
            o.push_str(&format!("  Main {} {:>3}%  {} / {} (\x1b[32m{} free\x1b[0m)\r\n",
                bar(pct, 100), pct, ByteSize::b(used), ByteSize::b(total), ByteSize::b(avail)));
        }
    }
    o.push_str("\r\n");

    // ─── Power ───
    if let Some(batt) = get_battery_info() {
        o.push_str("  \x1b[1;33mPower\x1b[0m\r\n");
        o.push_str(&format!("  {} {} {:>3}%  \x1b[90m{} cycles\x1b[0m\r\n",
            batt.status, bar_green(batt.level, 100), batt.level, batt.cycles));
        o.push_str("\r\n");
    }

    // ─── Top Processes ───
    o.push_str("  \x1b[1;33mTop Processes\x1b[0m\r\n");
    let mut procs: Vec<_> = sys.processes().values()
        .map(|p| (p.name().to_string(), p.cpu_usage(), p.memory()))
        .filter(|(_, cpu, _)| *cpu > 3.0)
        .collect();
    procs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    if procs.is_empty() {
        o.push_str("  \x1b[90mAll quiet\x1b[0m\r\n");
    }
    for (name, cpu, mem) in procs.iter().take(5) {
        let n = if name.len() > 18 { &name[..18] } else { name.as_str() };
        let hot = if *cpu > 80.0 { " \x1b[31m\u{25cf}\x1b[0m" } else { "" };
        o.push_str(&format!("  {:<18} {:>5.1}% {:>8}{}\r\n",
            n, cpu, ByteSize::b(*mem), hot));
    }
    o.push_str("\r\n");

    // Footer
    o.push_str("  \x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m\r\n");
    o.push_str("  \x1b[90mb back \u{b7} q quit \u{b7} refreshes every 1s\x1b[0m\r\n");

    o
}

fn bar(value: u64, max: u64) -> String {
    let width: usize = 12;
    let filled = (value as f64 / max as f64 * width as f64).min(width as f64) as usize;
    let empty = width.saturating_sub(filled);
    let b = format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty));
    if value > 80 { format!("\x1b[31m{}\x1b[0m", b) }
    else if value > 60 { format!("\x1b[33m{}\x1b[0m", b) }
    else { format!("\x1b[32m{}\x1b[0m", b) }
}

fn bar_green(value: u64, max: u64) -> String {
    let width: usize = 12;
    let filled = (value as f64 / max as f64 * width as f64).min(width as f64) as usize;
    let empty = width.saturating_sub(filled);
    format!("\x1b[32m{}\x1b[0m\x1b[90m{}\x1b[0m", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
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
    let status = if stdout.contains("charging") { "\u{26a1} Charging" }
        else if stdout.contains("charged") { "\u{2713} Charged" }
        else { "\u{1f50b} Battery" };
    let sp = Command::new("system_profiler").args(["SPPowerDataType"]).output().ok()?;
    let sp_out = String::from_utf8_lossy(&sp.stdout);
    let cycles = sp_out.lines()
        .find(|l| l.contains("Cycle Count"))
        .and_then(|l| l.split_whitespace().last())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    Some(BatteryInfo { level, status: status.to_string(), cycles })
}
