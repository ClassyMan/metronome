use gtk::glib;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static SETTINGS: OnceLock<Mutex<SettingsInner>> = OnceLock::new();

struct SettingsInner {
    data: HashMap<String, Value>,
    path: PathBuf,
}

fn defaults() -> HashMap<String, Value> {
    let mut map = HashMap::new();
    map.insert("window-width".into(), json!(-1));
    map.insert("window-height".into(), json!(-1));
    map.insert("is-maximized".into(), json!(false));
    map.insert("beats-per-bar".into(), json!(4));
    map.insert("beats-per-minute".into(), json!(100));
    map.insert("active-theme".into(), json!("Monokai"));
    map.insert("tempo-ramp-enabled".into(), json!(false));
    map.insert("tempo-ramp-increment".into(), json!(5));
    map.insert("tempo-ramp-bars".into(), json!(4));
    map.insert("tempo-ramp-target".into(), json!(260));
    map.insert("background-image-path".into(), json!(""));
    map.insert("background-opacity".into(), json!(0.15));
    map.insert("background-style".into(), json!("cover"));
    map
}

fn config_path() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("config.json")
}

fn init_settings() -> Mutex<SettingsInner> {
    let path = config_path();
    let mut data = defaults();

    if path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<HashMap<String, Value>>(&contents) {
                for (key, value) in parsed {
                    data.insert(key, value);
                }
            }
        }
    }

    Mutex::new(SettingsInner { data, path })
}

fn with_settings<F, R>(func: F) -> R
where
    F: FnOnce(&mut SettingsInner) -> R,
{
    let mutex = SETTINGS.get_or_init(init_settings);
    let mut inner = mutex.lock().unwrap();
    func(&mut inner)
}

fn save(inner: &SettingsInner) {
    if let Ok(json) = serde_json::to_string_pretty(&inner.data) {
        std::fs::write(&inner.path, json).ok();
    }
}

#[derive(Debug, Clone)]
pub struct PortableSettings;

impl PortableSettings {
    pub fn new(_app_id: &str) -> Self {
        let _ = SETTINGS.get_or_init(init_settings);
        PortableSettings
    }

    pub fn uint(&self, key: &str) -> u32 {
        with_settings(|inner| inner.data.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as u32)
    }

    pub fn set_uint(&self, key: &str, val: u32) -> Result<(), glib::BoolError> {
        with_settings(|inner| {
            inner.data.insert(key.into(), json!(val));
            save(inner);
        });
        Ok(())
    }

    pub fn boolean(&self, key: &str) -> bool {
        with_settings(|inner| {
            inner
                .data
                .get(key)
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
    }

    pub fn set_boolean(&self, key: &str, val: bool) -> Result<(), glib::BoolError> {
        with_settings(|inner| {
            inner.data.insert(key.into(), json!(val));
            save(inner);
        });
        Ok(())
    }

    pub fn string(&self, key: &str) -> glib::GString {
        with_settings(|inner| {
            let val = inner.data.get(key).and_then(|v| v.as_str()).unwrap_or("");
            glib::GString::from(val)
        })
    }

    pub fn set_string(&self, key: &str, val: &str) -> Result<(), glib::BoolError> {
        with_settings(|inner| {
            inner.data.insert(key.into(), json!(val));
            save(inner);
        });
        Ok(())
    }

    pub fn double(&self, key: &str) -> f64 {
        with_settings(|inner| inner.data.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0))
    }

    pub fn set_double(&self, key: &str, val: f64) -> Result<(), glib::BoolError> {
        with_settings(|inner| {
            inner.data.insert(key.into(), json!(val));
            save(inner);
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_has_all_keys() {
        let defs = defaults();
        assert_eq!(defs.len(), 13);
        assert_eq!(defs["beats-per-bar"], json!(4));
        assert_eq!(defs["active-theme"], json!("Monokai"));
        assert_eq!(defs["background-opacity"], json!(0.15));
    }

    #[test]
    fn test_defaults_types_are_correct() {
        let defs = defaults();
        assert!(defs["tempo-ramp-enabled"].is_boolean());
        assert!(defs["beats-per-minute"].is_number());
        assert!(defs["active-theme"].is_string());
        assert!(defs["background-opacity"].is_f64());
    }
}
