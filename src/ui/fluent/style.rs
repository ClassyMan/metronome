/// WinUI 3 Fluent Design style implementations for iced 0.14 Catalog traits.

use super::palette::Theme;
use iced::widget::{button, container, pick_list, scrollable, slider, text, text_input};
use iced::overlay::menu;
use iced::{Background, Border, Color, Shadow};

// ── button::Catalog ──────────────────────────────────────────────────

impl button::Catalog for Theme {
    type Class<'a> = button::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(button_primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: button::Status) -> button::Style {
        class(self, status)
    }
}

pub fn button_primary(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let base = button::Style {
        background: Some(Background::Color(palette.accent_default)),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(palette.accent_secondary)),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(palette.accent_tertiary)),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(palette.accent_disabled)),
            text_color: palette.text_disabled,
            ..base
        },
    }
}

#[allow(dead_code)]
pub fn button_secondary(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let base = button::Style {
        background: Some(Background::Color(palette.control_fill_default)),
        text_color: palette.text_primary,
        border: Border {
            color: palette.control_stroke_default,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(palette.control_fill_secondary)),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(palette.control_fill_tertiary)),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(palette.control_fill_disabled)),
            text_color: palette.text_disabled,
            border: Border {
                color: palette.control_stroke_default,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..base
        },
    }
}

pub fn button_subtle(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let base = button::Style {
        background: Some(Background::Color(palette.subtle_fill_transparent)),
        text_color: palette.text_primary,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Active => base,
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(palette.subtle_fill_secondary)),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(palette.subtle_fill_tertiary)),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(palette.subtle_fill_disabled)),
            text_color: palette.text_disabled,
            ..base
        },
    }
}

// ── container::Catalog ───────────────────────────────────────────────

impl container::Catalog for Theme {
    type Class<'a> = container::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(container_transparent)
    }

    fn style(&self, class: &Self::Class<'_>) -> container::Style {
        class(self)
    }
}

fn container_transparent(_theme: &Theme) -> container::Style {
    container::Style::default()
}

#[allow(dead_code)]
pub fn container_card(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(Background::Color(palette.card_background)),
        border: Border {
            color: palette.card_stroke,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

// ── slider::Catalog ──────────────────────────────────────────────────

impl slider::Catalog for Theme {
    type Class<'a> = slider::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(slider_default)
    }

    fn style(&self, class: &Self::Class<'_>, status: slider::Status) -> slider::Style {
        class(self, status)
    }
}

fn slider_default(theme: &Theme, status: slider::Status) -> slider::Style {
    let palette = theme.palette();
    let accent = match status {
        slider::Status::Active => palette.accent_default,
        slider::Status::Hovered => palette.accent_secondary,
        slider::Status::Dragged => palette.accent_tertiary,
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(accent),
                Background::Color(palette.control_strong_fill_default),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 8.0 },
            background: Background::Color(accent),
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        },
    }
}

// ── text::Catalog ────────────────────────────────────────────────────

impl text::Catalog for Theme {
    type Class<'a> = text::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|_theme| text::Style { color: None })
    }

    fn style(&self, class: &Self::Class<'_>) -> text::Style {
        class(self)
    }
}

// ── pick_list::Catalog ───────────────────────────────────────────────

impl pick_list::Catalog for Theme {
    type Class<'a> = pick_list::StyleFn<'a, Self>;

    fn default<'a>() -> <Self as pick_list::Catalog>::Class<'a> {
        Box::new(pick_list_default)
    }

    fn style(
        &self,
        class: &<Self as pick_list::Catalog>::Class<'_>,
        status: pick_list::Status,
    ) -> pick_list::Style {
        class(self, status)
    }
}

fn pick_list_default(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let palette = theme.palette();
    let base = pick_list::Style {
        text_color: palette.text_primary,
        placeholder_color: palette.text_secondary,
        handle_color: palette.text_secondary,
        background: Background::Color(palette.control_fill_default),
        border: Border {
            color: palette.control_stroke_default,
            width: 1.0,
            radius: 4.0.into(),
        },
    };
    match status {
        pick_list::Status::Active => base,
        pick_list::Status::Hovered => pick_list::Style {
            background: Background::Color(palette.control_fill_secondary),
            ..base
        },
        pick_list::Status::Opened { .. } => pick_list::Style {
            background: Background::Color(palette.control_fill_tertiary),
            border: Border {
                color: palette.accent_default,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..base
        },
    }
}

// ── menu::Catalog ────────────────────────────────────────────────────

impl menu::Catalog for Theme {
    type Class<'a> = menu::StyleFn<'a, Self>;

    fn default<'a>() -> <Self as menu::Catalog>::Class<'a> {
        Box::new(menu_default)
    }

    fn style(&self, class: &<Self as menu::Catalog>::Class<'_>) -> menu::Style {
        class(self)
    }
}

fn menu_default(theme: &Theme) -> menu::Style {
    let palette = theme.palette();
    menu::Style {
        background: Background::Color(palette.background_secondary),
        border: Border {
            color: palette.control_stroke_default,
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: palette.text_primary,
        selected_text_color: Color::WHITE,
        selected_background: Background::Color(palette.accent_default),
        shadow: Shadow {
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.26 },
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 16.0,
        },
    }
}

// ── text_input::Catalog ──────────────────────────────────────────────

impl text_input::Catalog for Theme {
    type Class<'a> = text_input::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(text_input_default)
    }

    fn style(&self, class: &Self::Class<'_>, status: text_input::Status) -> text_input::Style {
        class(self, status)
    }
}

fn text_input_default(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.palette();
    let base = text_input::Style {
        background: Background::Color(palette.control_fill_default),
        border: Border {
            color: palette.control_stroke_default,
            width: 1.0,
            radius: 4.0.into(),
        },
        icon: palette.text_secondary,
        placeholder: palette.text_tertiary,
        value: palette.text_primary,
        selection: palette.accent_default,
    };
    match status {
        text_input::Status::Active => base,
        text_input::Status::Hovered => text_input::Style {
            background: Background::Color(palette.control_fill_secondary),
            ..base
        },
        text_input::Status::Focused { .. } => text_input::Style {
            background: Background::Color(palette.control_fill_secondary),
            border: Border {
                color: palette.accent_default,
                width: 2.0,
                radius: 4.0.into(),
            },
            ..base
        },
        text_input::Status::Disabled => text_input::Style {
            background: Background::Color(palette.control_fill_disabled),
            value: palette.text_disabled,
            placeholder: palette.text_disabled,
            ..base
        },
    }
}

// ── scrollable::Catalog ──────────────────────────────────────────────

impl scrollable::Catalog for Theme {
    type Class<'a> = scrollable::StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(scrollable_default)
    }

    fn style(&self, class: &Self::Class<'_>, status: scrollable::Status) -> scrollable::Style {
        class(self, status)
    }
}

fn scrollable_default(theme: &Theme, _status: scrollable::Status) -> scrollable::Style {
    let palette = theme.palette();
    let rail = scrollable::Rail {
        background: Some(Background::Color(palette.control_fill_default)),
        border: Border {
            radius: 4.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        scroller: scrollable::Scroller {
            background: Background::Color(palette.control_strong_fill_default),
            border: Border {
                radius: 4.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(palette.background_secondary),
            border: Border {
                radius: (u32::MAX).into(),
                width: 1.0,
                color: palette.control_stroke_default,
            },
            shadow: Shadow::default(),
            icon: palette.text_primary,
        },
    }
}
