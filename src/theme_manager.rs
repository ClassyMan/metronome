use crate::config::APP_ID;
use crate::theme::{Theme, ThemeEntry};
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::path::PathBuf;

const BUILTIN_THEMES: &[&str] = &[
    "monokai",
    "dracula",
    "nord",
    "catppuccin-mocha",
    "gruvbox-dark",
    "solarized-dark",
    "ocean",
    "forest",
    "berry",
    "sunset",
];

#[derive(Debug)]
pub struct ThemeManager {
    css_provider: gtk::CssProvider,
    themes: Vec<ThemeEntry>,
    active_theme_name: String,
    user_themes_dir: PathBuf,
    settings: gio::Settings,
}

impl ThemeManager {
    pub fn new() -> Self {
        let provider = gtk::CssProvider::new();
        let display = gdk::Display::default().expect("Could not get default display");
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );

        let data_dir = glib::user_data_dir().join("metronome").join("themes");
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir).ok();
        }

        let settings = gio::Settings::new(APP_ID);

        Self {
            css_provider: provider,
            themes: Vec::new(),
            active_theme_name: String::new(),
            user_themes_dir: data_dir,
            settings,
        }
    }

    pub fn load_builtin_themes(&mut self) {
        for theme_name in BUILTIN_THEMES {
            let resource_path = format!("/com/adrienplazas/Metronome/themes/{}.json", theme_name);
            if let Ok(bytes) =
                gio::resources_lookup_data(&resource_path, gio::ResourceLookupFlags::NONE)
            {
                if let Ok(json) = std::str::from_utf8(&bytes) {
                    if let Ok(theme) = Theme::from_json(json) {
                        self.themes.push(ThemeEntry {
                            file_name: format!("{}.json", theme_name),
                            theme,
                            builtin: true,
                        });
                    }
                }
            }
        }
    }

    pub fn load_user_themes(&mut self) {
        let entries = match std::fs::read_dir(&self.user_themes_dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(theme) = Theme::from_json(&json) {
                    let file_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    self.themes.push(ThemeEntry {
                        theme,
                        builtin: false,
                        file_name,
                    });
                }
            }
        }
    }

    pub fn apply_theme(&mut self, name: &str) {
        self.active_theme_name = name.to_string();
        let style_manager = adw::StyleManager::default();

        if name.is_empty() {
            self.css_provider.load_from_string("");
            style_manager.set_color_scheme(adw::ColorScheme::Default);
        } else if let Some(entry) = self.themes.iter().find(|entry| entry.theme.name == name) {
            let css = entry.theme.to_css();
            self.css_provider.load_from_string(&css);
            if entry.theme.dark {
                style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            } else {
                style_manager.set_color_scheme(adw::ColorScheme::Default);
            }
        } else {
            log::warn!("Theme '{}' not found, falling back to default", name);
            self.css_provider.load_from_string("");
            style_manager.set_color_scheme(adw::ColorScheme::Default);
            self.active_theme_name = String::new();
        }

        self.settings
            .set_string("active-theme", &self.active_theme_name)
            .ok();
    }

    pub fn active_theme_name(&self) -> &str {
        &self.active_theme_name
    }

    pub fn themes(&self) -> &[ThemeEntry] {
        &self.themes
    }

    pub fn save_user_theme(&mut self, theme: Theme) -> Result<(), String> {
        let json = theme.to_json().map_err(|err| err.to_string())?;
        let file_name = Theme::sanitize_filename(&theme.name);
        let path = self.user_themes_dir.join(&file_name);
        std::fs::write(&path, &json).map_err(|err| err.to_string())?;

        if let Some(existing) = self
            .themes
            .iter_mut()
            .find(|entry| !entry.builtin && entry.theme.name == theme.name)
        {
            existing.theme = theme;
            existing.file_name = file_name;
        } else {
            self.themes.push(ThemeEntry {
                theme,
                builtin: false,
                file_name,
            });
        }

        Ok(())
    }

    pub fn delete_user_theme(&mut self, name: &str) -> Result<(), String> {
        let index = self
            .themes
            .iter()
            .position(|entry| !entry.builtin && entry.theme.name == name)
            .ok_or_else(|| format!("Theme '{}' not found or is built-in", name))?;

        let entry = &self.themes[index];
        let path = self.user_themes_dir.join(&entry.file_name);
        std::fs::remove_file(&path).map_err(|err| err.to_string())?;
        self.themes.remove(index);

        if self.active_theme_name == name {
            self.apply_theme("");
        }

        Ok(())
    }

    pub fn import_theme(&mut self, path: &std::path::Path) -> Result<String, String> {
        let json = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
        let theme = Theme::from_json(&json).map_err(|err| err.to_string())?;
        let name = theme.name.clone();
        self.save_user_theme(theme)?;
        Ok(name)
    }

    pub fn export_theme(&self, name: &str, dest: &std::path::Path) -> Result<(), String> {
        let entry = self
            .themes
            .iter()
            .find(|entry| entry.theme.name == name)
            .ok_or_else(|| format!("Theme '{}' not found", name))?;
        let json = entry.theme.to_json().map_err(|err| err.to_string())?;
        std::fs::write(dest, &json).map_err(|err| err.to_string())
    }

    pub fn restore_active_theme(&mut self) {
        let name = self.settings.string("active-theme").to_string();
        self.apply_theme(&name);
    }
}
