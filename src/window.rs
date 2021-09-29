use crate::application::MtrApplication;
use crate::clicker::MtrClicker;
use crate::config::{APP_ID, PROFILE};
use crate::timer::MtrTimer;
use crate::timerbutton::MtrTimerButton;
use adw::subclass::prelude::*;
use glib::clone;
use glib::ParamSpec;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::*;
use once_cell::sync::Lazy;
use std::cell::Cell;
use std::time::Instant;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let builder =
                gtk::Builder::from_resource("/com/adrienplazas/Metronome/ui/shortcuts.ui");
            let shortcuts = builder.object("shortcuts").unwrap();

            obj.set_help_overlay(Some(&shortcuts));

            // Devel Profile
            if PROFILE == "Devel" {
                obj.style_context().add_class("devel");
            }

            self.timer
                .connect_local(
                    "beat",
                    false,
                    clone!(@strong obj as this => move |args| {
                        let high = args[1].get::<bool>().unwrap();

                        let imp = imp::MtrApplicationWindow::from_instance(&this);
                        if high { imp.clicker.high(); } else { imp.clicker.low(); }

                        None
                    }),
                )
                .unwrap();

            self.time_signature_2_4_button.get().connect_notify_local(
                Some("active"),
                clone!(@strong obj as this => move |button, _| {
                    let imp = imp::MtrApplicationWindow::from_instance(&this);
                    if button.is_active() {
                        imp.beats_per_bar.set(2);
                        this.notify("beats-per-bar");
                    }
                }),
            );

            self.time_signature_3_4_button.get().connect_notify_local(
                Some("active"),
                clone!(@strong obj as this => move |button, _| {
                    let imp = imp::MtrApplicationWindow::from_instance(&this);
                    if button.is_active() {
                        imp.beats_per_bar.set(3);
                        this.notify("beats-per-bar");
                    }
                }),
            );

            self.time_signature_4_4_button.get().connect_notify_local(
                Some("active"),
                clone!(@strong obj as this => move |button, _| {
                    let imp = imp::MtrApplicationWindow::from_instance(&this);
                    if button.is_active() {
                        imp.beats_per_bar.set(4);
                        this.notify("beats-per-bar");
                    }
                }),
            );

            self.time_signature_6_8_button.get().connect_notify_local(
                Some("active"),
                clone!(@strong obj as this => move |button, _| {
                    let imp = imp::MtrApplicationWindow::from_instance(&this);
                    if button.is_active() {
                        imp.beats_per_bar.set(6);
                        this.notify("beats-per-bar");
                    }
                }),
            );
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_uint(
                        "beats-per-bar",
                        "Beats per bar",
                        "Beats per bar",
                        1,
                        9,
                        4,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_uint(
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

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "beats-per-bar" => self.beats_per_bar.get().to_value(),
                "beats-per-minute" => self.beats_per_minute.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "beats-per-bar" => self.beats_per_bar.set(value.get::<u32>().unwrap()),
                "beats-per-minute" => self.beats_per_minute.set(value.get::<u32>().unwrap()),
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
        let window: Self = glib::Object::new(&[]).expect("Failed to create MtrApplicationWindow");
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
                this.add_to_bpm(-1);
            })
        );

        action!(
            self,
            "increase-bpm",
            clone!(@weak self as this => move |_, _| {
                this.add_to_bpm(1);
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

    fn set_beats_per_minute(&self, bpm: u32) {
        let imp = imp::MtrApplicationWindow::from_instance(&self);
        imp.beats_per_minute.set(bpm.clamp(20, 260));
        self.notify("beats-per-minute");
    }

    fn add_to_bpm(&self, value: i32) {
        let imp = imp::MtrApplicationWindow::from_instance(&self);
        self.set_beats_per_minute(imp.beats_per_minute.get().wrapping_add(value as u32));
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
}
