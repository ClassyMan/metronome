use crate::application::MtrApplication;
use crate::config::{APP_ID, PROFILE};
use crate::scales_page::MtrScalesPage;
use crate::theme_dialog::MtrThemeDialog;
use crate::theme_manager::ThemeManager;
use crate::timer::MtrTimer;
use crate::timerbutton::MtrTimerButton;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::time::Instant;

pub const BPB_MIN: u32 = 1;
pub const BPB_MAX: u32 = 99;
pub const BPB_DEFAULT: u32 = 4;

pub const BPM_MIN: u32 = 20;
pub const BPM_MAX: u32 = 260;
pub const BPM_DEFAULT: u32 = 100;

pub const RAMP_INCREMENT_MIN: u32 = 1;
pub const RAMP_INCREMENT_MAX: u32 = 50;
pub const RAMP_INCREMENT_DEFAULT: u32 = 5;
pub const RAMP_BARS_MIN: u32 = 1;
pub const RAMP_BARS_MAX: u32 = 32;
pub const RAMP_BARS_DEFAULT: u32 = 4;

pub const VOLUME_MIN: f64 = 0.0;
pub const VOLUME_MAX: f64 = 1.0;
pub const VOLUME_DEFAULT: f64 = 1.0;

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/com/adrienplazas/Metronome/ui/window.ui")]
    #[properties(wrapper_type = super::MtrApplicationWindow)]
    pub struct MtrApplicationWindow {
        #[template_child]
        pub timer_button: TemplateChild<MtrTimerButton>,
        #[template_child]
        pub timer: TemplateChild<MtrTimer>,
        #[template_child]
        pub bpm_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub bg_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub scales_page: TemplateChild<MtrScalesPage>,
        #[property(get, set = Self::set_beats_per_bar, minimum = BPB_MIN, maximum = BPB_MAX, default = BPB_DEFAULT)]
        pub beats_per_bar: Cell<u32>,
        #[property(get, set = Self::set_beats_per_minute, minimum = BPM_MIN, maximum = BPM_MAX, default = BPM_DEFAULT)]
        pub beats_per_minute: Cell<u32>,
        #[property(get, set = Self::set_tempo_ramp_enabled)]
        pub tempo_ramp_enabled: Cell<bool>,
        #[property(get, set = Self::set_tempo_ramp_increment, minimum = RAMP_INCREMENT_MIN, maximum = RAMP_INCREMENT_MAX, default = RAMP_INCREMENT_DEFAULT)]
        pub tempo_ramp_increment: Cell<u32>,
        #[property(get, set = Self::set_tempo_ramp_bars, minimum = RAMP_BARS_MIN, maximum = RAMP_BARS_MAX, default = RAMP_BARS_DEFAULT)]
        pub tempo_ramp_bars: Cell<u32>,
        #[property(get, set = Self::set_tempo_ramp_target, minimum = BPM_MIN, maximum = BPM_MAX, default = BPM_MAX)]
        pub tempo_ramp_target: Cell<u32>,
        #[property(get, set = Self::set_volume, minimum = VOLUME_MIN, maximum = VOLUME_MAX, default = VOLUME_DEFAULT)]
        pub volume: Cell<f64>,
        pub tap_history: RefCell<Vec<Instant>>,
        pub settings: gio::Settings,
        pub theme_manager: RefCell<ThemeManager>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrApplicationWindow {
        const NAME: &'static str = "MtrApplicationWindow";
        type Type = super::MtrApplicationWindow;
        type ParentType = adw::ApplicationWindow;

        fn new() -> Self {
            Self {
                timer_button: Default::default(),
                timer: Default::default(),
                bpm_label: Default::default(),
                bg_picture: Default::default(),
                scales_page: Default::default(),
                beats_per_bar: std::cell::Cell::new(BPB_DEFAULT),
                beats_per_minute: std::cell::Cell::new(BPM_DEFAULT),
                tempo_ramp_enabled: std::cell::Cell::new(false),
                tempo_ramp_increment: std::cell::Cell::new(RAMP_INCREMENT_DEFAULT),
                tempo_ramp_bars: std::cell::Cell::new(RAMP_BARS_DEFAULT),
                tempo_ramp_target: std::cell::Cell::new(BPM_MAX),
                volume: std::cell::Cell::new(VOLUME_DEFAULT),
                tap_history: RefCell::new(Vec::new()),
                settings: gio::Settings::new(APP_ID),
                theme_manager: RefCell::new(ThemeManager::new()),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.install_action("win.decrease-bpb", None, |win, _, _| {
                let bpb = win.beats_per_bar();
                if bpb > BPB_MIN {
                    win.set_beats_per_bar(bpb - 1);
                }
            });

            klass.install_action("win.increase-bpb", None, |win, _, _| {
                let bpb = win.beats_per_bar();
                if bpb < BPB_MAX {
                    win.set_beats_per_bar(bpb + 1);
                }
            });

            klass.install_action("win.decrease-bpm", None, |win, _, _| {
                win.add_beats_per_minute(-1);
            });

            klass.install_action("win.increase-bpm", None, |win, _, _| {
                win.add_beats_per_minute(1);
            });

            klass.install_action("win.tap", None, |win, _, _| {
                win.tap();
            });

            klass.install_action("win.show-theme-dialog", None, |win, _, _| {
                win.show_theme_dialog();
            });

            klass.install_action("win.show-background-dialog", None, |win, _, _| {
                win.show_background_dialog();
            });

        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            MtrTimerButton::ensure_type();
            MtrTimer::ensure_type();
            MtrScalesPage::ensure_type();
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MtrApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Devel Profile
            if PROFILE == "Devel" {
                obj.add_css_class("devel");
            }
            obj.load_settings();

            // Flash BPM label on ramp increment
            let bpm_label = self.bpm_label.clone();
            self.timer
                .connect_notify_local(Some("ramp-status"), move |timer, _| {
                    let status = timer.ramp_status();
                    if !status.is_empty() && !status.starts_with("Bar") {
                        // "Reached target" or just after an increment — skip flash
                    }
                    if !status.is_empty()
                        && status.contains("next:")
                        && status.starts_with("Bar 1/")
                    {
                        // Just incremented (bar counter reset to 1)
                        bpm_label.add_css_class("ramp-flash");
                        let label = bpm_label.clone();
                        glib::timeout_add_local_once(
                            std::time::Duration::from_millis(400),
                            move || {
                                label.remove_css_class("ramp-flash");
                            },
                        );
                    }
                });

            let mut theme_manager = self.theme_manager.borrow_mut();
            theme_manager.load_builtin_themes();
            theme_manager.load_user_themes();
            theme_manager.restore_active_theme();
        }
    }

    impl WidgetImpl for MtrApplicationWindow {}

    impl WindowImpl for MtrApplicationWindow {}

    impl ApplicationWindowImpl for MtrApplicationWindow {}

    impl AdwApplicationWindowImpl for MtrApplicationWindow {}

    impl MtrApplicationWindow {
        fn set_beats_per_bar(&self, bpb: u32) {
            self.beats_per_bar.set(bpb);

            if let Err(err) = self.settings.set_uint("beats-per-bar", bpb) {
                log::warn!("Failed to save the beats per bar, {}", &err);
            }

            self.obj()
                .action_set_enabled("win.decrease-bpb", bpb != BPB_MIN);
            self.obj()
                .action_set_enabled("win.increase-bpb", bpb != BPB_MAX);

            self.obj().notify_beats_per_bar();
        }

        fn set_beats_per_minute(&self, bpm: u32) {
            self.beats_per_minute.set(bpm);

            if let Err(err) = self.settings.set_uint("beats-per-minute", bpm) {
                log::warn!("Failed to save the beats per minute, {}", &err);
            }

            self.obj()
                .action_set_enabled("win.decrease-bpm", bpm != BPM_MIN);
            self.obj()
                .action_set_enabled("win.increase-bpm", bpm != BPM_MAX);

            self.obj().notify_beats_per_minute();
        }

        fn set_tempo_ramp_enabled(&self, val: bool) {
            self.tempo_ramp_enabled.set(val);
            self.settings.set_boolean("tempo-ramp-enabled", val).ok();
            self.obj().notify_tempo_ramp_enabled();
        }

        fn set_tempo_ramp_increment(&self, val: u32) {
            self.tempo_ramp_increment.set(val);
            self.settings.set_uint("tempo-ramp-increment", val).ok();
            self.obj().notify_tempo_ramp_increment();
        }

        fn set_tempo_ramp_bars(&self, val: u32) {
            self.tempo_ramp_bars.set(val);
            self.settings.set_uint("tempo-ramp-bars", val).ok();
            self.obj().notify_tempo_ramp_bars();
        }

        fn set_tempo_ramp_target(&self, val: u32) {
            self.tempo_ramp_target.set(val);
            self.settings.set_uint("tempo-ramp-target", val).ok();
            self.obj().notify_tempo_ramp_target();
        }

        fn set_volume(&self, volume: f64) {
            self.volume.set(volume);
            if let Err(err) = self.settings.set_double("volume", volume) {
                log::warn!("Failed to save volume, {}", &err);
            }
            self.obj().notify_volume();
        }
    }
}

glib::wrapper! {
    pub struct MtrApplicationWindow(ObjectSubclass<imp::MtrApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup;
}

#[gtk::template_callbacks]
impl MtrApplicationWindow {
    pub fn new(app: &MtrApplication) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    fn add_beats_per_minute(&self, value: i32) {
        let bpm = (self.beats_per_minute() as i32 + value).clamp(BPM_MIN as i32, BPM_MAX as i32);
        self.set_beats_per_minute(bpm as u32);
    }

    fn tap(&self) {
        let imp = self.imp();
        let now = Instant::now();
        let mut history = imp.tap_history.borrow_mut();

        // Discard taps older than 3 seconds
        history.retain(|t| now.duration_since(*t).as_secs_f64() < 3.0);
        history.push(now);

        if history.len() >= 2 {
            let total_intervals: f64 = history
                .windows(2)
                .map(|pair| pair[1].duration_since(pair[0]).as_secs_f64())
                .sum();
            let avg_interval = total_intervals / (history.len() - 1) as f64;
            let bpm = (60.0 / avg_interval) as u32;
            drop(history);
            self.set_beats_per_minute(bpm.clamp(BPM_MIN, BPM_MAX));
        }
    }

    fn load_settings(&self) {
        let imp = self.imp();
        self.set_beats_per_bar(imp.settings.uint("beats-per-bar"));
        self.set_beats_per_minute(imp.settings.uint("beats-per-minute"));
        self.set_tempo_ramp_enabled(imp.settings.boolean("tempo-ramp-enabled"));
        self.set_tempo_ramp_increment(imp.settings.uint("tempo-ramp-increment"));
        self.set_tempo_ramp_bars(imp.settings.uint("tempo-ramp-bars"));
        self.set_tempo_ramp_target(imp.settings.uint("tempo-ramp-target"));
        self.set_volume(imp.settings.double("volume"));
        self.apply_background();
        imp.scales_page.bind_settings(&imp.settings);
    }

    fn show_theme_dialog(&self) {
        let dialog = MtrThemeDialog::new(self);
        dialog.present(self);
    }

    fn show_background_dialog(&self) {
        let imp = self.imp();
        let settings = &imp.settings;

        let dialog = adw::Dialog::builder()
            .title("Background Image")
            .content_width(360)
            .content_height(400)
            .build();

        let toolbar = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        toolbar.add_top_bar(&header);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);

        let group = adw::PreferencesGroup::new();

        // Image path row with choose button
        let image_row = adw::ActionRow::builder().title("Image").build();
        let current_path = settings.string("background-image-path").to_string();
        let path_label = gtk::Label::new(if current_path.is_empty() {
            Some("None")
        } else {
            std::path::Path::new(&current_path)
                .file_name()
                .and_then(|name| name.to_str())
                .or(Some("Selected"))
        });
        path_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        path_label.set_max_width_chars(20);
        path_label.add_css_class("dim-label");

        let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        button_box.set_valign(gtk::Align::Center);

        let choose_button = gtk::Button::from_icon_name("document-open-symbolic");
        choose_button.set_tooltip_text(Some("Choose image"));
        choose_button.add_css_class("flat");

        let clear_button = gtk::Button::from_icon_name("edit-clear-symbolic");
        clear_button.set_tooltip_text(Some("Remove background"));
        clear_button.add_css_class("flat");

        button_box.append(&choose_button);
        button_box.append(&clear_button);

        image_row.add_suffix(&path_label);
        image_row.add_suffix(&button_box);
        group.add(&image_row);

        // Style dropdown
        let style_row = adw::ComboRow::builder().title("Style").build();
        let style_model = gtk::StringList::new(&["Cover", "Contain", "Fill", "Tile"]);
        style_row.set_model(Some(&style_model));
        let current_style = settings.string("background-style").to_string();
        let style_idx = match current_style.as_str() {
            "cover" => 0,
            "contain" => 1,
            "fill" => 2,
            "tile" => 3,
            _ => 0,
        };
        style_row.set_selected(style_idx);
        group.add(&style_row);

        // Opacity slider
        let opacity_row = adw::ActionRow::builder().title("Opacity").build();
        let opacity_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.05);
        opacity_scale.set_value(settings.double("background-opacity"));
        opacity_scale.set_hexpand(true);
        opacity_scale.set_valign(gtk::Align::Center);
        opacity_scale.set_draw_value(true);
        opacity_scale.set_value_pos(gtk::PositionType::Right);
        opacity_row.add_suffix(&opacity_scale);
        group.add(&opacity_row);

        content.append(&group);
        toolbar.set_content(Some(&content));
        dialog.set_child(Some(&toolbar));

        // Choose button handler
        {
            let win_weak = self.downgrade();
            let path_label = path_label.clone();
            choose_button.connect_clicked(move |button| {
                let Some(window) = win_weak.upgrade() else {
                    return;
                };
                let path_label = path_label.clone();
                let win_weak = window.downgrade();

                let filter = gtk::FileFilter::new();
                filter.add_mime_type("image/*");
                filter.set_name(Some("Images"));
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);

                let file_dialog = gtk::FileDialog::builder()
                    .title("Choose Background Image")
                    .filters(&filters)
                    .build();

                let root = button.root().and_downcast::<gtk::Window>();
                file_dialog.open(root.as_ref(), gio::Cancellable::NONE, move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            let path_str = path.to_string_lossy().to_string();
                            if let Some(window) = win_weak.upgrade() {
                                window
                                    .imp()
                                    .settings
                                    .set_string("background-image-path", &path_str)
                                    .ok();
                                window.apply_background();
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    path_label.set_text(name);
                                }
                            }
                        }
                    }
                });
            });
        }

        // Clear button handler
        {
            let win_weak = self.downgrade();
            let path_label = path_label.clone();
            clear_button.connect_clicked(move |_| {
                if let Some(window) = win_weak.upgrade() {
                    window
                        .imp()
                        .settings
                        .set_string("background-image-path", "")
                        .ok();
                    window.apply_background();
                    path_label.set_text("None");
                }
            });
        }

        // Style changed handler
        {
            let win_weak = self.downgrade();
            style_row.connect_selected_notify(move |row: &adw::ComboRow| {
                let style = match row.selected() {
                    0 => "cover",
                    1 => "contain",
                    2 => "fill",
                    3 => "tile",
                    _ => "cover",
                };
                if let Some(window) = win_weak.upgrade() {
                    window
                        .imp()
                        .settings
                        .set_string("background-style", style)
                        .ok();
                    window.apply_background();
                }
            });
        }

        // Opacity changed handler
        {
            let win_weak = self.downgrade();
            opacity_scale.connect_value_changed(move |scale| {
                if let Some(window) = win_weak.upgrade() {
                    window
                        .imp()
                        .settings
                        .set_double("background-opacity", scale.value())
                        .ok();
                    window.apply_background();
                }
            });
        }

        dialog.present(self);
    }

    fn apply_background(&self) {
        let imp = self.imp();
        let path = imp.settings.string("background-image-path").to_string();
        let opacity = imp.settings.double("background-opacity");
        let style = imp.settings.string("background-style").to_string();

        if path.is_empty() || !std::path::Path::new(&path).exists() {
            imp.bg_picture.set_visible(false);
            return;
        }

        let file = gio::File::for_path(&path);
        imp.bg_picture.set_file(Some(&file));
        imp.bg_picture.set_opacity(opacity);
        imp.bg_picture.set_visible(true);

        let fit = match style.as_str() {
            "cover" => gtk::ContentFit::Cover,
            "contain" => gtk::ContentFit::Contain,
            "fill" => gtk::ContentFit::Fill,
            "tile" => gtk::ContentFit::Cover, // GTK4 Picture doesn't tile; cover is closest
            _ => gtk::ContentFit::Cover,
        };
        imp.bg_picture.set_content_fit(fit);
    }

}
