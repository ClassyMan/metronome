use crate::application::MtrApplication;
use crate::clicker::MtrClicker;
use crate::config::{APP_ID, PROFILE};
use crate::timer::MtrTimer;
use crate::timerbutton::MtrTimerButton;
use adw::subclass::prelude::*;
use gtk::{gio, glib, prelude::*};
use std::time::Instant;

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
        pub time_signature_2_4_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub time_signature_3_4_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub time_signature_4_4_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub time_signature_6_8_button: TemplateChild<gtk::ToggleButton>,
        pub clicker: MtrClicker,
        #[property(get, set = Self::set_beats_per_bar, minimum = 1, maximum = 9, default = 4)]
        pub beats_per_bar: Cell<u32>,
        #[property(get, set = Self::set_beats_per_minute, minimum = 20, maximum = 260, default = 100)]
        pub beats_per_minute: Cell<u32>,
        pub tap_time: Cell<Instant>,
        pub settings: gio::Settings,
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
                time_signature_2_4_button: Default::default(),
                time_signature_3_4_button: Default::default(),
                time_signature_4_4_button: Default::default(),
                time_signature_6_8_button: Default::default(),
                clicker: Default::default(),
                beats_per_bar: std::cell::Cell::new(4),
                beats_per_minute: std::cell::Cell::new(100),
                tap_time: std::cell::Cell::new(Instant::now()),
                settings: gio::Settings::new(APP_ID),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.install_action("win.decrease-bpm", None, |win, _, _| {
                win.add_beats_per_minute(-1);
            });

            klass.install_action("win.increase-bpm", None, |win, _, _| {
                win.add_beats_per_minute(1);
            });

            klass.install_action("win.tap", None, |win, _, _| {
                win.tap();
            });
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            MtrTimerButton::ensure_type();
            MtrTimer::ensure_type();
            obj.init_template();
        }
    }

    impl ObjectImpl for MtrApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // Devel Profile
            if PROFILE == "Devel" {
                obj.add_css_class("devel");
            }
            obj.load_settings();
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }
    }

    impl WidgetImpl for MtrApplicationWindow {}

    impl WindowImpl for MtrApplicationWindow {}

    impl ApplicationWindowImpl for MtrApplicationWindow {}

    impl AdwApplicationWindowImpl for MtrApplicationWindow {}

    impl MtrApplicationWindow {
        fn set_beats_per_bar(&self, bpm: u32) {
            self.beats_per_bar.set(bpm);

            if let Some(button) = match bpm {
                2 => Some(self.time_signature_2_4_button.get()),
                3 => Some(self.time_signature_3_4_button.get()),
                4 => Some(self.time_signature_4_4_button.get()),
                6 => Some(self.time_signature_6_8_button.get()),
                _ => None,
            } {
                button.set_active(true);
            }

            if let Err(err) = self.settings.set_uint("beats-per-bar", bpm) {
                log::warn!("Failed to save the beats per bar, {}", &err);
            }

            self.obj().notify_beats_per_bar();
        }

        fn set_beats_per_minute(&self, bpm: u32) {
            self.beats_per_minute.set(bpm);

            if let Err(err) = self.settings.set_uint("beats-per-minute", bpm) {
                log::warn!("Failed to save the beats per minute, {}", &err);
            }

            self.obj().notify_beats_per_minute();
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
        let bpm = (self.beats_per_minute() as i32 + value).clamp(20, 260);
        self.set_beats_per_minute(bpm as u32);
    }

    fn tap(&self) {
        let imp = self.imp();
        let now = Instant::now();
        let duration = now - imp.tap_time.get();
        let bpm = 60.0 / duration.as_secs_f64();
        imp.tap_time.set(now);
        self.set_beats_per_minute((bpm as u32).clamp(20, 260));
        imp.clicker.low();
    }

    fn load_settings(&self) {
        let imp = self.imp();
        self.set_beats_per_bar(imp.settings.uint("beats-per-bar"));
        self.set_beats_per_minute(imp.settings.uint("beats-per-minute"));
    }

    #[template_callback]
    fn on_time_signature_2_4_button_active(
        &self,
        _pspec: &glib::ParamSpec,
        button: &gtk::ToggleButton,
    ) {
        if button.is_active() {
            self.set_beats_per_bar(2);
        }
    }

    #[template_callback]
    fn on_time_signature_3_4_button_active(
        &self,
        _pspec: &glib::ParamSpec,
        button: &gtk::ToggleButton,
    ) {
        if button.is_active() {
            self.set_beats_per_bar(3);
        }
    }

    #[template_callback]
    fn on_time_signature_4_4_button_active(
        &self,
        _pspec: &glib::ParamSpec,
        button: &gtk::ToggleButton,
    ) {
        if button.is_active() {
            self.set_beats_per_bar(4);
        }
    }

    #[template_callback]
    fn on_time_signature_6_8_button_active(
        &self,
        _pspec: &glib::ParamSpec,
        button: &gtk::ToggleButton,
    ) {
        if button.is_active() {
            self.set_beats_per_bar(8);
        }
    }

    #[template_callback]
    fn on_beat(&self, high: bool) {
        let imp = self.imp();
        if high {
            imp.clicker.high();
        } else {
            imp.clicker.low();
        }
    }
}
