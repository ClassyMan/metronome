use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub accent_bg_color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_fg_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_bg_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_fg_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_fg_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_bg_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub borders: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub version: u32,
    #[serde(default)]
    pub dark: bool,
    pub colors: ThemeColors,
}

#[derive(Debug, Clone)]
pub struct ThemeEntry {
    pub theme: Theme,
    pub builtin: bool,
    pub file_name: String,
}

impl Theme {
    pub fn to_css(&self) -> String {
        let mut css = String::new();
        css.push_str(&format!(
            "@define-color accent_bg_color {};\n",
            self.colors.accent_bg_color
        ));
        if let Some(ref color) = self.colors.accent_fg_color {
            css.push_str(&format!("@define-color accent_fg_color {};\n", color));
        }
        if let Some(ref color) = self.colors.theme_bg_color {
            css.push_str(&format!("@define-color theme_bg_color {};\n", color));
        }
        if let Some(ref color) = self.colors.theme_fg_color {
            css.push_str(&format!("@define-color theme_fg_color {};\n", color));
        }
        if let Some(ref color) = self.colors.window_fg_color {
            css.push_str(&format!("@define-color window_fg_color {};\n", color));
        }
        if let Some(ref color) = self.colors.view_bg_color {
            css.push_str(&format!("@define-color view_bg_color {};\n", color));
        }
        if let Some(ref color) = self.colors.borders {
            css.push_str(&format!("@define-color borders {};\n", color));
        }
        css
    }

    pub fn from_json(json: &str) -> Result<Theme, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn sanitize_filename(name: &str) -> String {
        let sanitized: String = name
            .to_lowercase()
            .chars()
            .map(|ch| if ch.is_alphanumeric() { ch } else { '-' })
            .collect();
        format!("{}.json", sanitized)
    }
}
