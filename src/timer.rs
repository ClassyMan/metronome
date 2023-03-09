use adw::subclass::prelude::*;
use glib::clone;
use glib::subclass::Signal;
use glib::subclass::SignalType;
use glib::Type;
use gtk::glib;
use gtk::{self, prelude::*};
use once_cell::sync::Lazy;
use std::cell::{Cell, RefCell};
use std::time::Instant;

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct MtrTimer {
        pub active: Cell<bool>,
        pub beats_per_bar: Cell<u32>,
        pub beats_per_minute: Cell<u32>,
        pub beat_in_bar: Cell<u32>,
        pub start_time: Cell<Instant>,
        pub click_id: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTimer {
        const NAME: &'static str = "MtrTimer";
        type Type = super::MtrTimer;

        fn new() -> Self {
            Self {
                active: std::cell::Cell::<bool>::new(false),
                beats_per_bar: std::cell::Cell::<u32>::new(4),
                beats_per_minute: std::cell::Cell::<u32>::new(100),
                beat_in_bar: std::cell::Cell::<u32>::new(0),
                start_time: std::cell::Cell::<Instant>::new(Instant::now()),
                click_id: RefCell::new(None),
            }
        }
    }

    impl ObjectImpl for MtrTimer {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecBoolean::new(
                        "active",
                        "Active",
                        "Active",
                        false,
                        glib::ParamFlags::WRITABLE,
                    ),
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

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("beat")
                    .param_types([SignalType::from(bool::static_type())])
                    .build()]
            });

            SIGNALS.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "beats-per-bar" => self.beats_per_bar.get().to_value(),
                "beats-per-minute" => self.beats_per_minute.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "active" => self.obj().set_active(value.get::<bool>().unwrap()),
                "beats-per-bar" => self.beats_per_bar.set(value.get::<u32>().unwrap()),
                "beats-per-minute" => self.beats_per_minute.set(value.get::<u32>().unwrap()),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct MtrTimer(ObjectSubclass<imp::MtrTimer>);
}

impl MtrTimer {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn set_active(&self, active: bool) {
        let imp = imp::MtrTimer::from_instance(&self);
        imp.active.set(active);

        if active {
            let ms_per_beat = 60000 / (imp.beats_per_minute.get() as u64);
            let click_id = glib::timeout_add_local(
                std::time::Duration::from_millis(ms_per_beat),
                clone!(@strong self as this => @default-return glib::Continue(false), move || {
                    let imp = imp::MtrTimer::from_instance(&this);

                    let beat_in_bar = (imp.beat_in_bar.get() + 1) % imp.beats_per_bar.get();
                    imp.beat_in_bar.set(beat_in_bar);

                    this.emit_by_name::<()>("beat", &[&(beat_in_bar == 0)]);
                    glib::Continue(true)
                }),
            );
            imp.click_id.replace(Some(click_id));

            imp.beat_in_bar.set(0);
            self.emit_by_name::<()>("beat", &[&true]);
        } else {
            if let Some(id) = imp.click_id.take() {
                id.remove();
            }
        }
    }
}
