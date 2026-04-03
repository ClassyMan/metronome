/// WinUI 3 Fluent Design color palette and theme definition.

use iced::Color;

/// WinUI 3 Fluent Design color tokens.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub accent_default: Color,
    pub accent_secondary: Color,
    pub accent_tertiary: Color,
    pub accent_disabled: Color,
    pub background_base: Color,
    pub background_secondary: Color,
    pub background_tertiary: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub text_disabled: Color,
    pub control_fill_default: Color,
    pub control_fill_secondary: Color,
    pub control_fill_tertiary: Color,
    pub control_fill_disabled: Color,
    pub control_stroke_default: Color,
    pub control_stroke_secondary: Color,
    pub control_strong_fill_default: Color,
    pub control_strong_fill_disabled: Color,
    pub control_strong_stroke_default: Color,
    pub subtle_fill_transparent: Color,
    pub subtle_fill_secondary: Color,
    pub subtle_fill_tertiary: Color,
    pub subtle_fill_disabled: Color,
    pub card_background: Color,
    pub card_stroke: Color,
    pub divider_stroke: Color,
    pub focus_stroke_outer: Color,
    pub focus_stroke_inner: Color,
    pub system_attention: Color,
    pub system_success: Color,
    pub system_caution: Color,
    pub system_critical: Color,
}

impl Palette {
    pub const DARK: Palette = Palette {
        accent_default: Color { r: 0.463, g: 0.725, b: 0.929, a: 1.0 },
        accent_secondary: Color { r: 0.463, g: 0.725, b: 0.929, a: 0.898 },
        accent_tertiary: Color { r: 0.463, g: 0.725, b: 0.929, a: 0.800 },
        accent_disabled: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.247 },
        background_base: Color { r: 0.125, g: 0.125, b: 0.125, a: 1.0 },
        background_secondary: Color { r: 0.110, g: 0.110, b: 0.110, a: 1.0 },
        background_tertiary: Color { r: 0.075, g: 0.075, b: 0.075, a: 1.0 },
        text_primary: Color::WHITE,
        text_secondary: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.786 },
        text_tertiary: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.541 },
        text_disabled: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.365 },
        control_fill_default: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.059 },
        control_fill_secondary: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.082 },
        control_fill_tertiary: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.031 },
        control_fill_disabled: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.043 },
        control_stroke_default: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.071 },
        control_stroke_secondary: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.094 },
        control_strong_fill_default: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.545 },
        control_strong_fill_disabled: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.247 },
        control_strong_stroke_default: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.545 },
        subtle_fill_transparent: Color::TRANSPARENT,
        subtle_fill_secondary: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.059 },
        subtle_fill_tertiary: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.031 },
        subtle_fill_disabled: Color::TRANSPARENT,
        card_background: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.051 },
        card_stroke: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.063 },
        divider_stroke: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.082 },
        focus_stroke_outer: Color::WHITE,
        focus_stroke_inner: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.698 },
        system_attention: Color { r: 0.463, g: 0.725, b: 0.929, a: 1.0 },
        system_success: Color { r: 0.424, g: 0.796, b: 0.373, a: 1.0 },
        system_caution: Color { r: 0.988, g: 0.882, b: 0.0, a: 1.0 },
        system_critical: Color { r: 1.0, g: 0.600, b: 0.643, a: 1.0 },
    };

    pub const LIGHT: Palette = Palette {
        accent_default: Color { r: 0.0, g: 0.471, b: 0.831, a: 1.0 },
        accent_secondary: Color { r: 0.0, g: 0.471, b: 0.831, a: 0.898 },
        accent_tertiary: Color { r: 0.0, g: 0.471, b: 0.831, a: 0.800 },
        accent_disabled: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.220 },
        background_base: Color { r: 0.961, g: 0.961, b: 0.961, a: 1.0 },
        background_secondary: Color { r: 0.933, g: 0.933, b: 0.933, a: 1.0 },
        background_tertiary: Color { r: 0.980, g: 0.980, b: 0.980, a: 1.0 },
        text_primary: Color { r: 0.894, g: 0.894, b: 0.894, a: 1.0 },
        text_secondary: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.624 },
        text_tertiary: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.443 },
        text_disabled: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.365 },
        control_fill_default: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.702 },
        control_fill_secondary: Color { r: 0.961, g: 0.961, b: 0.961, a: 0.502 },
        control_fill_tertiary: Color { r: 0.961, g: 0.961, b: 0.961, a: 0.302 },
        control_fill_disabled: Color { r: 0.961, g: 0.961, b: 0.961, a: 0.302 },
        control_stroke_default: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.059 },
        control_stroke_secondary: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.161 },
        control_strong_fill_default: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.447 },
        control_strong_fill_disabled: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.318 },
        control_strong_stroke_default: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.447 },
        subtle_fill_transparent: Color::TRANSPARENT,
        subtle_fill_secondary: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.039 },
        subtle_fill_tertiary: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.024 },
        subtle_fill_disabled: Color::TRANSPARENT,
        card_background: Color { r: 1.0, g: 1.0, b: 1.0, a: 0.702 },
        card_stroke: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.059 },
        divider_stroke: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.082 },
        focus_stroke_outer: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.894 },
        focus_stroke_inner: Color::WHITE,
        system_attention: Color { r: 0.0, g: 0.471, b: 0.831, a: 1.0 },
        system_success: Color { r: 0.047, g: 0.533, b: 0.208, a: 1.0 },
        system_caution: Color { r: 0.620, g: 0.533, b: 0.098, a: 1.0 },
        system_critical: Color { r: 0.769, g: 0.220, b: 0.173, a: 1.0 },
    };
}

/// Fluent Design theme with Dark and Light variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn palette(&self) -> &'static Palette {
        match self {
            Theme::Dark => &Palette::DARK,
            Theme::Light => &Palette::LIGHT,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

impl iced::theme::Base for Theme {
    fn default(preference: iced::theme::Mode) -> Self {
        match preference {
            iced::theme::Mode::Dark => Theme::Dark,
            iced::theme::Mode::Light => Theme::Light,
            iced::theme::Mode::None => Theme::Dark,
        }
    }

    fn mode(&self) -> iced::theme::Mode {
        match self {
            Theme::Dark => iced::theme::Mode::Dark,
            Theme::Light => iced::theme::Mode::Light,
        }
    }

    fn base(&self) -> iced::theme::Style {
        let palette = self.palette();
        iced::theme::Style {
            background_color: palette.background_base,
            text_color: palette.text_primary,
        }
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        let palette = Theme::palette(self);
        Some(iced::theme::Palette {
            background: palette.background_base,
            text: palette.text_primary,
            primary: palette.accent_default,
            success: palette.system_success,
            danger: palette.system_critical,
            warning: palette.system_caution,
        })
    }

    fn name(&self) -> &str {
        match self {
            Theme::Dark => "Fluent Dark",
            Theme::Light => "Fluent Light",
        }
    }
}
