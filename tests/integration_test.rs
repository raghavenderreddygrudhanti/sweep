//! Integration tests for Sweep using /tmp/sweep-test sandbox.
//! Run with: cargo test -- --nocapture

use std::path::Path;
use std::fs;

// Import the crate's public modules via the binary
// Since this is a binary crate, we test via command execution

#[test]
fn test_scan_size_native() {
    // Test that the scanner works on the test directory
    let test_dir = Path::new("/tmp/sweep-test");
    if !test_dir.exists() {
        println!("SKIP: /tmp/sweep-test not found. Create test data first.");
        return;
    }

    // Run sweep with --json to get parseable output
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sweep"))
        .args(["recommend", "--json"])
        .output()
        .expect("Failed to run sweep");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("recommendations"), "JSON output should contain recommendations");
    assert!(stdout.contains("total_reclaimable"), "JSON output should contain total_reclaimable");
    println!("recommend --json output:\n{}", stdout);
}

#[test]
fn test_timeline_no_cache() {
    // Timeline with no previous data should not crash
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sweep"))
        .args(["timeline", "--json"])
        .output()
        .expect("Failed to run sweep");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("changes"), "Timeline JSON should have changes field");
    println!("timeline --json output:\n{}", stdout);
}

#[test]
fn test_history_json_empty() {
    // History on clean system should not crash
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sweep"))
        .args(["history", "--json"])
        .output()
        .expect("Failed to run sweep");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON (array)
    assert!(stdout.starts_with('['), "History JSON should be an array, got: {}", stdout);
    println!("history --json output:\n{}", stdout);
}

#[test]
fn test_help_shows_new_commands() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sweep"))
        .args(["--help"])
        .output()
        .expect("Failed to run sweep");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("timeline"), "--help should list timeline command");
    assert!(stdout.contains("recommend"), "--help should list recommend command");
    assert!(stdout.contains("--json"), "--help should list --json flag");
    assert!(stdout.contains("--force"), "--help should list --force flag");
    println!("--help output:\n{}", stdout);
}

#[test]
fn test_version() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sweep"))
        .args(["--version"])
        .output()
        .expect("Failed to run sweep");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.2.0"), "Version should be 0.2.0, got: {}", stdout);
}
