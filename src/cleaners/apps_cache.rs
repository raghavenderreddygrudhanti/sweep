//! App-specific cache cleaner — Spotify, Slack, Discord, Teams, VS Code, etc.

use std::path::PathBuf;

/// Known app caches that can safely be cleaned.
pub fn app_cache_paths() -> Vec<(PathBuf, &'static str)> {
    let home = crate::error::home_or_exit();
    let caches = home.join("Library/Caches");
    let support = home.join("Library/Application Support");

    vec![
        // Communication
        (caches.join("com.tinyspeck.slackmacgap"), "Slack cache"),
        (support.join("Slack/Cache"), "Slack data cache"),
        (support.join("Slack/Service Worker/CacheStorage"), "Slack Service Workers"),
        (caches.join("com.hnc.Discord"), "Discord cache"),
        (support.join("discord/Cache"), "Discord data cache"),
        (caches.join("com.microsoft.teams2"), "Microsoft Teams cache"),
        (support.join("zoom.us/data"), "Zoom data"),

        // Media
        (caches.join("com.spotify.client"), "Spotify cache"),
        (support.join("Spotify/PersistentCache"), "Spotify persistent cache"),
        (caches.join("com.apple.Music"), "Apple Music cache"),
        (caches.join("com.apple.podcasts"), "Podcasts cache"),

        // Productivity
        (caches.join("com.microsoft.Word"), "Microsoft Word cache"),
        (caches.join("com.microsoft.Excel"), "Microsoft Excel cache"),
        (caches.join("com.microsoft.Powerpoint"), "Microsoft PowerPoint cache"),
        (caches.join("com.microsoft.onenote.mac"), "OneNote cache"),
        (caches.join("com.microsoft.Outlook"), "Outlook cache"),

        // Dev tools
        (support.join("Code/CachedData"), "VS Code cached data"),
        (support.join("Code/CachedExtensionVSIXs"), "VS Code extension cache"),
        (support.join("Code/Cache"), "VS Code cache"),
        (caches.join("com.postmanlabs.mac"), "Postman cache"),
        (caches.join("com.insomnia.app"), "Insomnia cache"),

        // Cloud & Storage
        (caches.join("com.apple.iCloudDrive"), "iCloud Drive cache"),
        (caches.join("com.microsoft.OneDrive"), "OneDrive cache"),
        (caches.join("com.google.GoogleDrive"), "Google Drive cache"),
        (caches.join("com.getdropbox.dropbox"), "Dropbox cache"),

        // Other
        (caches.join("com.adobe.Reader"), "Adobe Reader cache"),
        (caches.join("com.adobe.Photoshop"), "Photoshop cache"),
        (caches.join("com.figma.Desktop"), "Figma cache"),
        (caches.join("notion.id"), "Notion cache"),
        (caches.join("com.linear"), "Linear cache"),
    ]
    .into_iter()
    .filter(|(p, _)| p.exists())
    .collect()
}
