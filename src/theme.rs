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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_theme() -> Theme {
        Theme {
            name: "Test".to_string(),
            version: 1,
            dark: false,
            colors: ThemeColors {
                accent_bg_color: "#ff0000".to_string(),
                accent_fg_color: None,
                theme_bg_color: None,
                theme_fg_color: None,
                window_fg_color: None,
                view_bg_color: None,
                borders: None,
            },
        }
    }

    fn make_full_theme() -> Theme {
        Theme {
            name: "Full".to_string(),
            version: 1,
            dark: true,
            colors: ThemeColors {
                accent_bg_color: "#bd93f9".to_string(),
                accent_fg_color: Some("#282a36".to_string()),
                theme_bg_color: Some("#282a36".to_string()),
                theme_fg_color: Some("#f8f8f2".to_string()),
                window_fg_color: Some("#6272a4".to_string()),
                view_bg_color: Some("#44475a".to_string()),
                borders: Some("#44475a".to_string()),
            },
        }
    }

    #[test]
    fn test_to_css_minimal_theme_only_emits_accent() {
        let theme = make_minimal_theme();
        let css = theme.to_css();
        assert_eq!(css, "@define-color accent_bg_color #ff0000;\n");
    }

    #[test]
    fn test_to_css_full_theme_emits_all_colors() {
        let theme = make_full_theme();
        let css = theme.to_css();
        assert!(css.contains("@define-color accent_bg_color #bd93f9;"));
        assert!(css.contains("@define-color accent_fg_color #282a36;"));
        assert!(css.contains("@define-color theme_bg_color #282a36;"));
        assert!(css.contains("@define-color theme_fg_color #f8f8f2;"));
        assert!(css.contains("@define-color window_fg_color #6272a4;"));
        assert!(css.contains("@define-color view_bg_color #44475a;"));
        assert!(css.contains("@define-color borders #44475a;"));
        assert_eq!(css.lines().count(), 7);
    }

    #[test]
    fn test_json_roundtrip_minimal() {
        let theme = make_minimal_theme();
        let json = theme.to_json().unwrap();
        let parsed = Theme::from_json(&json).unwrap();
        assert_eq!(parsed.name, "Test");
        assert_eq!(parsed.colors.accent_bg_color, "#ff0000");
        assert!(!parsed.dark);
        assert!(parsed.colors.accent_fg_color.is_none());
    }

    #[test]
    fn test_json_roundtrip_full() {
        let theme = make_full_theme();
        let json = theme.to_json().unwrap();
        let parsed = Theme::from_json(&json).unwrap();
        assert_eq!(parsed.name, "Full");
        assert!(parsed.dark);
        assert_eq!(parsed.colors.borders.as_deref(), Some("#44475a"));
    }

    #[test]
    fn test_dark_defaults_to_false() {
        let json = r##"{"name":"Nope","version":1,"colors":{"accent_bg_color":"#000"}}"##;
        let theme = Theme::from_json(json).unwrap();
        assert!(!theme.dark);
    }

    #[test]
    fn test_optional_colors_omitted_in_json() {
        let theme = make_minimal_theme();
        let json = theme.to_json().unwrap();
        assert!(!json.contains("accent_fg_color"));
        assert!(!json.contains("borders"));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(Theme::sanitize_filename("Ocean"), "ocean.json");
        assert_eq!(
            Theme::sanitize_filename("Gruvbox Dark"),
            "gruvbox-dark.json"
        );
        assert_eq!(
            Theme::sanitize_filename("My Theme! #2"),
            "my-theme---2.json"
        );
    }

    #[test]
    fn test_invalid_json_returns_error() {
        assert!(Theme::from_json("not json").is_err());
        assert!(Theme::from_json(r#"{"name":"X"}"#).is_err());
    }

    #[test]
    fn test_sanitize_filename_preserves_alphanumeric() {
        assert_eq!(Theme::sanitize_filename("abc123"), "abc123.json");
    }

    #[test]
    fn test_to_css_does_not_emit_none_colors() {
        let theme = make_minimal_theme();
        let css = theme.to_css();
        assert!(!css.contains("theme_bg_color"));
        assert!(!css.contains("borders"));
        assert!(!css.contains("view_bg_color"));
    }
}
