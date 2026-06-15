//! Size cache — stores overview sizes to disk for instant load on next run.
//! Cache TTL: 24 hours. Stored at ~/.sweep/sizes.json

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_TTL_SECS: u64 = 24 * 3600; // 24 hours

fn cache_path() -> PathBuf {
    let dir = dirs::home_dir().unwrap_or_default().join(".sweep");
    let _ = fs::create_dir_all(&dir);
    dir.join("sizes.json")
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct CacheEntry {
    size: u64,
    timestamp: u64,
}

pub fn load_cached_sizes() -> HashMap<String, u64> {
    let path = cache_path();
    let mut result = HashMap::new();

    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(entries) = serde_json::from_str::<HashMap<String, CacheEntry>>(&content) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            for (key, entry) in entries {
                if now - entry.timestamp < CACHE_TTL_SECS {
                    result.insert(key, entry.size);
                }
            }
        }
    }

    result
}

pub fn save_size(path: &str, size: u64) {
    let cache_file = cache_path();
    let mut entries: HashMap<String, CacheEntry> = if let Ok(content) = fs::read_to_string(&cache_file) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    entries.insert(path.to_string(), CacheEntry { size, timestamp: now });

    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let _ = fs::write(&cache_file, json);
    }
}

pub fn save_all(sizes: &HashMap<String, u64>) {
    let cache_file = cache_path();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let entries: HashMap<String, CacheEntry> = sizes.iter()
        .map(|(k, &v)| (k.clone(), CacheEntry { size: v, timestamp: now }))
        .collect();

    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let _ = fs::write(&cache_file, json);
    }
}
