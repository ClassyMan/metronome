use crate::theme_editor::MtrThemeEditor;
use crate::window::MtrApplicationWindow;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/adrienplazas/Metronome/ui/theme-dialog.ui")]
    pub struct MtrThemeDialog {
        #[template_child]
        pub theme_flow: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub new_theme_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub import_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrThemeDialog {
        const NAME: &'static str = "MtrThemeDialog";
        type Type = super::MtrThemeDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MtrThemeDialog {}
    impl WidgetImpl for MtrThemeDialog {}
    impl AdwDialogImpl for MtrThemeDialog {}
}

glib::wrapper! {
    pub struct MtrThemeDialog(ObjectSubclass<imp::MtrThemeDialog>)
        @extends adw::Dialog, gtk::Widget;
}

struct ThemeSnapshot {
    name: String,
    accent_color: String,
    dark: bool,
    builtin: bool,
}

impl MtrThemeDialog {
    pub fn new(window: &MtrApplicationWindow) -> Self {
        let dialog: Self = glib::Object::new();
        dialog.setup(window);
        dialog
    }

    fn setup(&self, window: &MtrApplicationWindow) {
        let theme_imp = self.imp();

        let (active_name, snapshots) = {
            let manager = window.imp().theme_manager.borrow();
            let active = manager.active_theme_name().to_string();
            let snaps: Vec<ThemeSnapshot> = manager
                .themes()
                .iter()
                .map(|entry| ThemeSnapshot {
                    name: entry.theme.name.clone(),
                    accent_color: entry.theme.colors.accent_bg_color.clone(),
                    dark: entry.theme.dark,
                    builtin: entry.builtin,
                })
                .collect();
            (active, snaps)
        };

        theme_imp
            .theme_flow
            .set_sort_func(|_, _| gtk::Ordering::Equal);

        // Default theme card
        let default_card = build_swatch_card("Default", "#808080", false, active_name.is_empty());
        theme_imp.theme_flow.append(&default_card);

        // Theme cards
        let mut active_index: i32 = if active_name.is_empty() { 0 } else { -1 };

        for (idx, snap) in snapshots.iter().enumerate() {
            let is_active = snap.name == active_name;
            if is_active {
                active_index = (idx + 1) as i32;
            }
            let card = build_swatch_card(&snap.name, &snap.accent_color, snap.dark, is_active);

            if !snap.builtin {
                let action_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                action_box.set_halign(gtk::Align::Center);
                action_box.set_margin_top(4);

                let edit_btn = gtk::Button::from_icon_name("document-edit-symbolic");
                edit_btn.add_css_class("flat");
                edit_btn.add_css_class("circular");
                edit_btn.set_tooltip_text(Some("Edit"));

                let delete_btn = gtk::Button::from_icon_name("user-trash-symbolic");
                delete_btn.add_css_class("flat");
                delete_btn.add_css_class("circular");
                delete_btn.set_tooltip_text(Some("Delete"));

                action_box.append(&edit_btn);
                action_box.append(&delete_btn);

                if let Some(content_box) = card.first_child().and_then(|c| c.first_child()) {
                    if let Some(vbox) = content_box.downcast_ref::<gtk::Box>() {
                        vbox.append(&action_box);
                    }
                }

                let theme_name_edit = snap.name.clone();
                let window_weak = window.downgrade();
                let dialog_weak = self.downgrade();
                edit_btn.connect_clicked(move |_| {
                    let Some(window) = window_weak.upgrade() else {
                        return;
                    };
                    let theme = {
                        let manager = window.imp().theme_manager.borrow();
                        manager
                            .themes()
                            .iter()
                            .find(|entry| entry.theme.name == theme_name_edit)
                            .map(|entry| entry.theme.clone())
                    };
                    if let Some(theme) = theme {
                        let editor = MtrThemeEditor::new_edit(&window, &theme);
                        if let Some(dialog) = dialog_weak.upgrade() {
                            let window_weak = window.downgrade();
                            let dialog_weak = dialog.downgrade();
                            editor.connect_closure(
                                "theme-saved",
                                false,
                                glib::closure_local!(move |_editor: &MtrThemeEditor| {
                                    if let (Some(window), Some(dialog)) =
                                        (window_weak.upgrade(), dialog_weak.upgrade())
                                    {
                                        dialog.refresh(&window);
                                    }
                                }),
                            );
                            editor.present(&dialog);
                        }
                    }
                });

                let theme_name_del = snap.name.clone();
                let window_weak = window.downgrade();
                let dialog_weak = self.downgrade();
                delete_btn.connect_clicked(move |_| {
                    let Some(window) = window_weak.upgrade() else {
                        return;
                    };
                    let mut manager = window.imp().theme_manager.borrow_mut();
                    if let Err(err) = manager.delete_user_theme(&theme_name_del) {
                        log::warn!("Failed to delete theme: {}", err);
                    }
                    drop(manager);
                    if let Some(dialog) = dialog_weak.upgrade() {
                        dialog.refresh(&window);
                    }
                });
            }

            theme_imp.theme_flow.append(&card);
        }

        // Select the active card
        if active_index >= 0 {
            if let Some(child) = theme_imp.theme_flow.child_at_index(active_index) {
                theme_imp.theme_flow.select_child(&child);
            }
        }

        // Selection changed → apply theme
        {
            let window_weak = window.downgrade();
            let snapshots_names: Vec<String> = snapshots.iter().map(|s| s.name.clone()).collect();
            theme_imp
                .theme_flow
                .connect_selected_children_changed(move |flow| {
                    let selected = flow.selected_children();
                    let Some(child) = selected.first() else {
                        return;
                    };
                    let idx = child.index();
                    let theme_name = if idx == 0 {
                        ""
                    } else if let Some(name) = snapshots_names.get((idx - 1) as usize) {
                        name.as_str()
                    } else {
                        return;
                    };
                    if let Some(window) = window_weak.upgrade() {
                        window
                            .imp()
                            .theme_manager
                            .borrow_mut()
                            .apply_theme(theme_name);
                    }
                });
        }

        // New Theme button
        {
            let window_weak = window.downgrade();
            let dialog_weak = self.downgrade();
            theme_imp.new_theme_button.connect_clicked(move |_| {
                let Some(window) = window_weak.upgrade() else {
                    return;
                };
                let editor = MtrThemeEditor::new_create(&window);
                if let Some(dialog) = dialog_weak.upgrade() {
                    let window_weak = window.downgrade();
                    let dialog_weak = dialog.downgrade();
                    editor.connect_closure(
                        "theme-saved",
                        false,
                        glib::closure_local!(move |_editor: &MtrThemeEditor| {
                            if let (Some(window), Some(dialog)) =
                                (window_weak.upgrade(), dialog_weak.upgrade())
                            {
                                dialog.refresh(&window);
                            }
                        }),
                    );
                    editor.present(&dialog);
                }
            });
        }

        // Import button — multi-file
        {
            let window_weak = window.downgrade();
            let dialog_weak = self.downgrade();
            theme_imp.import_button.connect_clicked(move |button| {
                let Some(window) = window_weak.upgrade() else {
                    return;
                };
                let dialog_weak = dialog_weak.clone();
                let window_weak = window.downgrade();

                let filter = gtk::FileFilter::new();
                filter.add_pattern("*.json");
                filter.set_name(Some("Theme files (*.json)"));
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);

                let file_dialog = gtk::FileDialog::builder()
                    .title("Import Themes")
                    .filters(&filters)
                    .build();

                let root = button.root().and_downcast::<gtk::Window>();
                file_dialog.open_multiple(root.as_ref(), gio::Cancellable::NONE, move |result| {
                    if let Ok(files) = result {
                        if let Some(window) = window_weak.upgrade() {
                            let mut manager = window.imp().theme_manager.borrow_mut();
                            let mut last_name = String::new();
                            for idx in 0..files.n_items() {
                                if let Some(file) = files.item(idx).and_downcast::<gio::File>() {
                                    if let Some(path) = file.path() {
                                        match manager.import_theme(&path) {
                                            Ok(name) => last_name = name,
                                            Err(err) => {
                                                log::warn!("Failed to import {:?}: {}", path, err);
                                            }
                                        }
                                    }
                                }
                            }
                            if !last_name.is_empty() {
                                manager.apply_theme(&last_name);
                            }
                            drop(manager);
                            if let Some(dialog) = dialog_weak.upgrade() {
                                dialog.refresh(&window);
                            }
                        }
                    }
                });
            });
        }
    }

    fn refresh(&self, window: &MtrApplicationWindow) {
        let new_dialog = MtrThemeDialog::new(window);
        self.close();
        new_dialog.present(window);
    }
}

fn build_swatch_card(name: &str, accent_hex: &str, dark: bool, selected: bool) -> gtk::Widget {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 4);
    card.set_halign(gtk::Align::Center);

    let swatch = gtk::DrawingArea::new();
    swatch.set_size_request(72, 72);
    swatch.set_halign(gtk::Align::Center);

    let accent =
        gdk::RGBA::parse(accent_hex).unwrap_or_else(|_| gdk::RGBA::new(0.5, 0.5, 0.5, 1.0));
    let is_dark = dark;
    let is_selected = selected;

    swatch.set_draw_func(move |_, cr, width, height| {
        let bg_r: f64;
        let bg_g: f64;
        let bg_b: f64;
        if is_dark {
            bg_r = 0.15;
            bg_g = 0.15;
            bg_b = 0.17;
        } else {
            bg_r = 0.95;
            bg_g = 0.95;
            bg_b = 0.95;
        }

        let radius = 12.0;
        let w = width as f64;
        let h = height as f64;

        // Rounded rectangle background
        cr.new_sub_path();
        cr.arc(
            w - radius,
            radius,
            radius,
            -std::f64::consts::FRAC_PI_2,
            0.0,
        );
        cr.arc(
            w - radius,
            h - radius,
            radius,
            0.0,
            std::f64::consts::FRAC_PI_2,
        );
        cr.arc(
            radius,
            h - radius,
            radius,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
        );
        cr.arc(
            radius,
            radius,
            radius,
            std::f64::consts::PI,
            3.0 * std::f64::consts::FRAC_PI_2,
        );
        cr.close_path();
        cr.set_source_rgb(bg_r, bg_g, bg_b);
        let _ = cr.fill_preserve();

        // Border
        if is_selected {
            cr.set_source_rgba(
                accent.red() as f64,
                accent.green() as f64,
                accent.blue() as f64,
                1.0,
            );
            cr.set_line_width(3.0);
        } else {
            cr.set_source_rgba(0.5, 0.5, 0.5, 0.3);
            cr.set_line_width(1.0);
        }
        let _ = cr.stroke();

        // Accent circle in the center
        let cx = w / 2.0;
        let cy = h / 2.0;
        cr.arc(cx, cy, 16.0, 0.0, 2.0 * std::f64::consts::PI);
        cr.set_source_rgb(
            accent.red() as f64,
            accent.green() as f64,
            accent.blue() as f64,
        );
        let _ = cr.fill();

        // Checkmark on selected
        if is_selected {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.set_line_width(2.5);
            cr.move_to(cx - 6.0, cy);
            cr.line_to(cx - 2.0, cy + 5.0);
            cr.line_to(cx + 7.0, cy - 5.0);
            let _ = cr.stroke();
        }
    });

    card.append(&swatch);

    let label = gtk::Label::new(Some(name));
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_max_width_chars(10);
    label.add_css_class("caption");
    card.append(&label);

    card.upcast()
}
