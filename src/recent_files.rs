/// Recent files store for the tab player.
/// Persists the last 10 opened file paths as JSON in the app's data directory.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 10;
const FILENAME: &str = "recent-tabs.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFile {
    pub path: String,
    pub title: String,
}

pub fn store_path() -> PathBuf {
    let data_dir = glib::user_data_dir().join("metronome");
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join(FILENAME)
}

pub fn load() -> Vec<RecentFile> {
    let path = store_path();
    match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save(files: &[RecentFile]) {
    let path = store_path();
    if let Ok(json) = serde_json::to_string_pretty(files) {
        std::fs::write(&path, json).ok();
    }
}

pub fn add(file_path: &Path, title: &str) {
    let path_str = file_path.to_string_lossy().to_string();
    let mut files = load();

    // Remove duplicate
    files.retain(|entry| entry.path != path_str);

    // Add to front
    files.insert(0, RecentFile {
        path: path_str,
        title: title.to_string(),
    });

    // Keep max
    files.truncate(MAX_RECENT);

    save(&files);
}

use gtk::glib;
