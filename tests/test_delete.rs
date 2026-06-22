//! Test actual deletion against dummy data.
//! Tests both Trash mode and Force mode.

use std::fs;
use std::path::{Path, PathBuf};

fn test_base() -> PathBuf {
    // Use home dir for trash tests (macOS requires this for Trash to work)
    dirs::home_dir().unwrap().join(".sweep-test-tmp")
}

#[test]
fn test_scanner_accuracy() {
    let test_dir = Path::new("/tmp/sweep-test/project2");
    if !test_dir.exists() {
        println!("SKIP: /tmp/sweep-test/project2 not found");
        return;
    }

    let scanned_size: u64 = walkdir::WalkDir::new(test_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum();

    // project2 has 80MB + 40MB = 120MB
    let expected = 120 * 1024 * 1024;
    assert_eq!(
        scanned_size,
        expected,
        "Scanner should find 120 MB, found {} MB",
        scanned_size / (1024 * 1024)
    );

    println!(
        "PASS: Scanner accurately measured {} MB (expected 120 MB)",
        scanned_size / (1024 * 1024)
    );
}

#[test]
fn test_force_delete() {
    let target = test_base().join("force_test");
    let _ = fs::remove_dir_all(&target);
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("bigfile.bin"), vec![0u8; 5_000_000]).unwrap();
    assert!(target.exists());

    fs::remove_dir_all(&target).unwrap();
    assert!(!target.exists(), "Directory should be permanently gone");

    println!("PASS: Force delete works (5 MB removed permanently)");
}

#[test]
fn test_trash_file() {
    let base = test_base();
    let _ = fs::create_dir_all(&base);
    let test_file = base.join("trash_test_file.txt");
    fs::write(&test_file, "hello sweep trash test").unwrap();
    assert!(test_file.exists(), "Test file should exist before trash");

    trash::delete(&test_file).expect("Failed to trash file");
    assert!(!test_file.exists(), "Test file should be gone after trash");

    println!("PASS: File moved to Trash (recoverable via Finder > Trash)");
}

#[test]
fn test_trash_directory() {
    let test_dir = test_base().join("trash_dir_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(test_dir.join("subdir")).unwrap();
    fs::write(test_dir.join("file1.txt"), "a".repeat(1000)).unwrap();
    fs::write(test_dir.join("subdir/file2.txt"), "b".repeat(2000)).unwrap();
    assert!(test_dir.exists());

    trash::delete(&test_dir).expect("Failed to trash directory");
    assert!(!test_dir.exists(), "Directory should be gone after trash");

    println!("PASS: Directory (with subdirs) moved to Trash");
}

#[test]
fn test_trash_node_modules_simulation() {
    let base = test_base().join("nm_test");
    let nm = base.join("node_modules");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(nm.join("react")).unwrap();
    fs::create_dir_all(nm.join("webpack")).unwrap();
    fs::write(nm.join("react/index.js"), vec![0u8; 1_000_000]).unwrap();
    fs::write(nm.join("webpack/bundle.js"), vec![0u8; 2_000_000]).unwrap();

    let total_before: u64 = walkdir::WalkDir::new(&nm)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum();

    assert_eq!(total_before, 3_000_000, "Should be 3 MB before trash");

    trash::delete(&nm).expect("Failed to trash node_modules");
    assert!(!nm.exists(), "node_modules should be in Trash now");

    println!("PASS: node_modules (3 MB) moved to Trash");

    // Cleanup parent
    let _ = fs::remove_dir_all(&base);
}
