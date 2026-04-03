/// Settings persistence — serde JSON config file replacing GSettings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CONFIG_DIR: &str = "metronome";
const CONFIG_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_bpm")]
    pub bpm: u32,
    #[serde(default = "default_bpb")]
    pub beats_per_bar: u8,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default = "default_tempo_percent")]
    pub tab_tempo_percent: f64,
    #[serde(default = "default_guitar_volume")]
    pub tab_guitar_volume: u8,
    #[serde(default = "default_metronome_volume")]
    pub tab_metronome_volume: u8,
    #[serde(default)]
    pub tab_metronome_enabled: bool,
    #[serde(default)]
    pub tab_guitar_tone: usize,
    #[serde(default)]
    pub scale_root: usize,
    #[serde(default)]
    pub scale_family: usize,
    #[serde(default)]
    pub scale_mode: usize,
}

fn default_bpm() -> u32 { 100 }
fn default_bpb() -> u8 { 4 }
fn default_volume() -> f32 { 1.0 }
fn default_tempo_percent() -> f64 { 100.0 }
fn default_guitar_volume() -> u8 { 100 }
fn default_metronome_volume() -> u8 { 100 }

impl Default for Settings {
    fn default() -> Self {
        Self {
            bpm: default_bpm(),
            beats_per_bar: default_bpb(),
            volume: default_volume(),
            tab_tempo_percent: default_tempo_percent(),
            tab_guitar_volume: default_guitar_volume(),
            tab_metronome_volume: default_metronome_volume(),
            tab_metronome_enabled: false,
            tab_guitar_tone: 0,
            scale_root: 0,
            scale_family: 0,
            scale_mode: 0,
        }
    }
}

impl Settings {
    fn config_path() -> Option<PathBuf> {
        dirs_next().map(|dir| dir.join(CONFIG_FILE))
    }

    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| std::fs::read_to_string(&path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
        })
        .map(|config| config.join(CONFIG_DIR))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.bpm, 100);
        assert_eq!(settings.beats_per_bar, 4);
        assert!((settings.volume - 1.0).abs() < f32::EPSILON);
        assert_eq!(settings.tab_tempo_percent, 100.0);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let settings = Settings {
            bpm: 140,
            beats_per_bar: 7,
            volume: 0.5,
            scale_root: 3,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.bpm, 140);
        assert_eq!(restored.beats_per_bar, 7);
        assert_eq!(restored.scale_root, 3);
    }

    #[test]
    fn test_deserialize_partial_json() {
        let json = r#"{"bpm": 180}"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.bpm, 180);
        assert_eq!(settings.beats_per_bar, 4); // default
        assert_eq!(settings.tab_tempo_percent, 100.0); // default
    }

    #[test]
    fn test_deserialize_empty_json() {
        let json = "{}";
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.bpm, 100);
    }

    #[test]
    fn test_config_path_exists() {
        // Should resolve to something on Linux
        let path = Settings::config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("metronome"));
    }
}
