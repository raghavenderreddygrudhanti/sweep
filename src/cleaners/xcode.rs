//! Xcode cleaner — DerivedData, simulators, archives, device support.

use std::path::PathBuf;

pub fn xcode_paths() -> Vec<(PathBuf, &'static str)> {
    let home = crate::error::home_or_exit();
    vec![
        (home.join("Library/Developer/Xcode/DerivedData"), "Xcode DerivedData"),
        (home.join("Library/Developer/Xcode/Archives"), "Xcode Archives"),
        (home.join("Library/Developer/Xcode/iOS DeviceSupport"), "iOS Device Support"),
        (home.join("Library/Developer/Xcode/watchOS DeviceSupport"), "watchOS Device Support"),
        (home.join("Library/Developer/CoreSimulator/Caches"), "Simulator Caches"),
        (home.join("Library/Developer/CoreSimulator/Devices"), "Simulator Devices"),
        (home.join("Library/Caches/com.apple.dt.Xcode"), "Xcode Caches"),
        (home.join("Library/Developer/Xcode/Products"), "Xcode Products"),
        (home.join("Library/Developer/Xcode/UserData/IB Support"), "Interface Builder cache"),
    ]
    .into_iter()
    .filter(|(p, _)| p.exists())
    .collect()
}
