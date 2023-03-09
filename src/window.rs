use crate::application::MtrApplication;
use crate::clicker::MtrClicker;
use crate::config::{APP_ID, PROFILE};
use crate::timer::MtrTimer;
use crate::timerbutton::MtrTimerButton;
use adw::subclass::prelude::*;
use gtk::{
    gio,
    glib::{self, clone},
    prelude::*,
};
use gtk_macros::*;
use log::warn;
use std::time::Instant;

mod imp {
    use super::*;
    use once_cell::sync::Lazy;
    use std::cell::Cell;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/com/adrienplazas/Metronome/ui/window.ui")]
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
        pub beats_per_bar: Cell<u32>,
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
                timer_button: TemplateChild::default(),
                timer: TemplateChild::default(),
                time_signature_2_4_button: TemplateChild::default(),
                time_signature_3_4_button: TemplateChild::default(),
                time_signature_4_4_button: TemplateChild::default(),
                time_signature_6_8_button: TemplateChild::default(),
                clicker: MtrClicker::new(),
                beats_per_bar: std::cell::Cell::<u32>::new(4),
                beats_per_minute: std::cell::Cell::<u32>::new(100),
                tap_time: std::cell::Cell::<Instant>::new(Instant::now()),
                settings: gio::Settings::new(APP_ID),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            MtrTimerButton::static_type();
            MtrTimer::static_type();
            obj.init_template();
        }
    }

    impl ObjectImpl for MtrApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let builder =
                gtk::Builder::from_resource("/com/adrienplazas/Metronome/ui/shortcuts.ui");
            let shortcuts = builder.object("shortcuts").unwrap();

            obj.set_help_overlay(Some(&shortcuts));

            // Devel Profile
            if PROFILE == "Devel" {
                obj.style_context().add_class("devel");
            }

            self.timer.connect_local(
                "beat",
                false,
                clone!(@strong obj as this => move |args| {
                    let high = args[1].get::<bool>().unwrap();

                    let imp = imp::MtrApplicationWindow::from_instance(&this);
                    if high { imp.clicker.high(); } else { imp.clicker.low(); }

                    None
                }),
            );

            self.time_signature_2_4_button.get().connect_notify_local(
                Some("active"),
                clone!(@strong obj as this => move |button, _| {
                    if button.is_active() {
                        this.set_beats_per_bar(2);
                        this.notify("beats-per-bar");
                    }
                }),
            );

            self.time_signature_3_4_button.get().connect_notify_local(
                Some("active"),
                clone!(@strong obj as this => move |button, _| {
                    if button.is_active() {
                        this.set_beats_per_bar(3);
                        this.notify("beats-per-bar");
                    }
                }),
            );

            self.time_signature_4_4_button.get().connect_notify_local(
                Some("active"),
                clone!(@strong obj as this => move |button, _| {
                    if button.is_active() {
                        this.set_beats_per_bar(4);
                        this.notify("beats-per-bar");
                    }
                }),
            );

            self.time_signature_6_8_button.get().connect_notify_local(
                Some("active"),
                clone!(@strong obj as this => move |button, _| {
                    if button.is_active() {
                        this.set_beats_per_bar(6);
                        this.notify("beats-per-bar");
                    }
                }),
            );

            obj.load_settings();
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecUInt::new(
                        "beats-per-bar",
                        "Beats per bar",
                        "Beats per bar",
                        1,
                        9,
                        4,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpecUInt::new(
                        "beats-per-minute",
                        "Beats per minute",
                        "Beats per minute",
                        20,
                        260,
                        100,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "beats-per-bar" => self.beats_per_bar.get().to_value(),
                "beats-per-minute" => self.beats_per_minute.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = self.obj();
            match pspec.name() {
                "beats-per-bar" => obj.set_beats_per_bar(value.get::<u32>().unwrap()),
                "beats-per-minute" => obj.set_beats_per_minute(value.get::<u32>().unwrap()),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for MtrApplicationWindow {}

    impl WindowImpl for MtrApplicationWindow {}

    impl ApplicationWindowImpl for MtrApplicationWindow {}

    impl AdwApplicationWindowImpl for MtrApplicationWindow {}
}

glib::wrapper! {
    pub struct MtrApplicationWindow(ObjectSubclass<imp::MtrApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

impl MtrApplicationWindow {
    pub fn new(app: &MtrApplication) -> Self {
        let window = glib::Object::new::<Self>();
        window.set_application(Some(app));

        window.setup_actions();

        // Set icons for shell
        gtk::Window::set_default_icon_name(APP_ID);

        window
    }

    fn setup_actions(&self) {
        action!(
            self,
            "decrease-bpm",
            clone!(@weak self as this => move |_, _| {
                this.add_beats_per_minute(-1);
            })
        );

        action!(
            self,
            "increase-bpm",
            clone!(@weak self as this => move |_, _| {
                this.add_beats_per_minute(1);
            })
        );

        action!(
            self,
            "tap",
            clone!(@weak self as this => move |_, _| {
                this.tap();
            })
        );
    }

    fn set_beats_per_bar(&self, bpm: u32) {
        let imp = imp::MtrApplicationWindow::from_instance(&self);
        imp.beats_per_bar.set(bpm.clamp(1, 9));

        if let Some(button) = match bpm {
            2 => Some(imp.time_signature_2_4_button.get()),
            3 => Some(imp.time_signature_3_4_button.get()),
            4 => Some(imp.time_signature_4_4_button.get()),
            6 => Some(imp.time_signature_6_8_button.get()),
            _ => None,
        } {
            button.set_active(true);
        }

        if let Err(err) = imp
            .settings
            .set_uint("beats-per-bar", imp.beats_per_bar.get())
        {
            warn!("Failed to save the beats per bar, {}", &err);
        }

        self.notify("beats-per-bar");
    }

    fn set_beats_per_minute(&self, bpm: u32) {
        let imp = imp::MtrApplicationWindow::from_instance(&self);
        imp.beats_per_minute.set(bpm.clamp(20, 260));

        if let Err(err) = imp
            .settings
            .set_uint("beats-per-minute", imp.beats_per_minute.get())
        {
            warn!("Failed to save the beats per minute, {}", &err);
        }

        self.notify("beats-per-minute");
    }

    fn add_beats_per_minute(&self, value: i32) {
        let imp = imp::MtrApplicationWindow::from_instance(&self);
        let bpm = imp.beats_per_minute.get() as i32 + value;
        self.set_beats_per_minute(bpm as u32);
    }

    fn tap(&self) {
        let imp = imp::MtrApplicationWindow::from_instance(&self);
        let now = Instant::now();
        let duration = now - imp.tap_time.get();
        let bpm = 60.0 / duration.as_secs_f64();
        imp.tap_time.set(now);
        self.set_beats_per_minute(bpm as u32);
        imp.clicker.low();
    }

    fn load_settings(&self) {
        let imp = imp::MtrApplicationWindow::from_instance(&self);
        self.set_beats_per_bar(imp.settings.uint("beats-per-bar"));
        self.set_beats_per_minute(imp.settings.uint("beats-per-minute"));
    }
}
