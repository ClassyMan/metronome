use crate::theme::{Theme, ThemeColors};
use crate::window::MtrApplicationWindow;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, glib};

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/adrienplazas/Metronome/ui/theme-editor.ui")]
    pub struct MtrThemeEditor {
        #[template_child]
        pub cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub save_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub name_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub dark_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub accent_bg_button: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub accent_fg_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub accent_fg_button: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub theme_bg_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub theme_bg_button: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub theme_fg_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub theme_fg_button: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub window_fg_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub window_fg_button: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub view_bg_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub view_bg_button: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub borders_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub borders_button: TemplateChild<gtk::ColorDialogButton>,
        pub app_window: RefCell<Option<MtrApplicationWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrThemeEditor {
        const NAME: &'static str = "MtrThemeEditor";
        type Type = super::MtrThemeEditor;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("editor.cancel", None, |editor, _, _| {
                editor.close();
            });

            klass.install_action("editor.save", None, |editor, _, _| {
                editor.save_theme();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MtrThemeEditor {
        fn signals() -> &'static [glib::subclass::Signal] {
            use std::sync::OnceLock;
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![glib::subclass::Signal::builder("theme-saved").build()])
        }
    }

    impl WidgetImpl for MtrThemeEditor {}
    impl AdwDialogImpl for MtrThemeEditor {}
}

glib::wrapper! {
    pub struct MtrThemeEditor(ObjectSubclass<imp::MtrThemeEditor>)
        @extends adw::Dialog, gtk::Widget;
}

impl MtrThemeEditor {
    pub fn new_create(window: &MtrApplicationWindow) -> Self {
        let editor: Self = glib::Object::new();
        editor.imp().app_window.replace(Some(window.clone()));
        editor.setup_actions();
        editor
    }

    pub fn new_edit(window: &MtrApplicationWindow, theme: &Theme) -> Self {
        let editor: Self = glib::Object::new();
        editor.imp().app_window.replace(Some(window.clone()));
        editor.set_title("Edit Theme");
        editor.populate(theme);
        editor.setup_actions();
        editor
    }

    fn setup_actions(&self) {
        let editor_imp = self.imp();

        editor_imp
            .cancel_button
            .set_action_name(Some("editor.cancel"));
        editor_imp.save_button.set_action_name(Some("editor.save"));
    }

    fn populate(&self, theme: &Theme) {
        let editor_imp = self.imp();
        editor_imp.name_entry.set_text(&theme.name);
        editor_imp.dark_switch.set_active(theme.dark);

        set_color_button(&editor_imp.accent_bg_button, &theme.colors.accent_bg_color);

        if let Some(ref color) = theme.colors.accent_fg_color {
            editor_imp.accent_fg_switch.set_active(true);
            set_color_button(&editor_imp.accent_fg_button, color);
        }
        if let Some(ref color) = theme.colors.theme_bg_color {
            editor_imp.theme_bg_switch.set_active(true);
            set_color_button(&editor_imp.theme_bg_button, color);
        }
        if let Some(ref color) = theme.colors.theme_fg_color {
            editor_imp.theme_fg_switch.set_active(true);
            set_color_button(&editor_imp.theme_fg_button, color);
        }
        if let Some(ref color) = theme.colors.window_fg_color {
            editor_imp.window_fg_switch.set_active(true);
            set_color_button(&editor_imp.window_fg_button, color);
        }
        if let Some(ref color) = theme.colors.view_bg_color {
            editor_imp.view_bg_switch.set_active(true);
            set_color_button(&editor_imp.view_bg_button, color);
        }
        if let Some(ref color) = theme.colors.borders {
            editor_imp.borders_switch.set_active(true);
            set_color_button(&editor_imp.borders_button, color);
        }
    }

    fn save_theme(&self) {
        let editor_imp = self.imp();
        let name = editor_imp.name_entry.text().to_string();
        if name.is_empty() {
            return;
        }

        let theme = Theme {
            name: name.clone(),
            version: 1,
            dark: editor_imp.dark_switch.is_active(),
            colors: ThemeColors {
                accent_bg_color: color_to_hex(&editor_imp.accent_bg_button.rgba()),
                accent_fg_color: if editor_imp.accent_fg_switch.is_active() {
                    Some(color_to_hex(&editor_imp.accent_fg_button.rgba()))
                } else {
                    None
                },
                theme_bg_color: if editor_imp.theme_bg_switch.is_active() {
                    Some(color_to_hex(&editor_imp.theme_bg_button.rgba()))
                } else {
                    None
                },
                theme_fg_color: if editor_imp.theme_fg_switch.is_active() {
                    Some(color_to_hex(&editor_imp.theme_fg_button.rgba()))
                } else {
                    None
                },
                window_fg_color: if editor_imp.window_fg_switch.is_active() {
                    Some(color_to_hex(&editor_imp.window_fg_button.rgba()))
                } else {
                    None
                },
                view_bg_color: if editor_imp.view_bg_switch.is_active() {
                    Some(color_to_hex(&editor_imp.view_bg_button.rgba()))
                } else {
                    None
                },
                borders: if editor_imp.borders_switch.is_active() {
                    Some(color_to_hex(&editor_imp.borders_button.rgba()))
                } else {
                    None
                },
            },
        };

        if let Some(window) = editor_imp.app_window.borrow().as_ref() {
            let mut manager = window.imp().theme_manager.borrow_mut();
            match manager.save_user_theme(theme) {
                Ok(()) => {
                    manager.apply_theme(&name);
                    drop(manager);
                    self.emit_by_name::<()>("theme-saved", &[]);
                    self.close();
                }
                Err(err) => {
                    log::warn!("Failed to save theme: {}", err);
                }
            }
        }
    }
}

fn set_color_button(button: &gtk::ColorDialogButton, hex: &str) {
    if let Ok(rgba) = gdk::RGBA::parse(hex) {
        button.set_rgba(&rgba);
    }
}

fn color_to_hex(rgba: &gdk::RGBA) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (rgba.red() * 255.0) as u8,
        (rgba.green() * 255.0) as u8,
        (rgba.blue() * 255.0) as u8,
    )
}
