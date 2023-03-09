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

    #[derive(Debug, glib::Properties)]
    #[properties(wrapper_type = super::MtrTimer)]
    pub struct MtrTimer {
        #[property(get, set = Self::set_active)]
        pub active: Cell<bool>,
        #[property(get, set, minimum = 1, maximum = 9, default = 4)]
        pub beats_per_bar: Cell<u32>,
        #[property(get, set, minimum = 20, maximum = 260, default = 100)]
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
            Self::derived_properties()
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("beat")
                    .param_types([bool::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }
    }

    impl MtrTimer {
        fn set_active(&self, active: bool) {
            self.active.set(active);
            let obj = self.obj();
            if active {
                let ms_per_beat = 60000 / (obj.beats_per_minute() as u64);
                let click_id = glib::timeout_add_local(
                    std::time::Duration::from_millis(ms_per_beat),
                    clone!(@strong obj => move || {
                        let imp = obj.imp();

                        let beat_in_bar = (imp.beat_in_bar.get() + 1) % obj.beats_per_bar();
                        imp.beat_in_bar.set(beat_in_bar);

                        obj.emit_by_name::<()>("beat", &[&(beat_in_bar == 0)]);
                        glib::Continue(true)
                    }),
                );
                self.click_id.replace(Some(click_id));

                self.beat_in_bar.set(0);
                obj.emit_by_name::<()>("beat", &[&true]);
            } else {
                if let Some(id) = self.click_id.take() {
                    id.remove();
                }
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
}
