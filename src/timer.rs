use crate::clicker::MtrClicker;
use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::thread;
use std::time::Instant;

enum TimerCommand {
    Stop,
}

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;
    use std::cell::Cell;

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
        pub clicker: MtrClicker,
        thread_cmd: std::cell::RefCell<std::sync::mpsc::Sender<TimerCommand>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTimer {
        const NAME: &'static str = "MtrTimer";
        type Type = super::MtrTimer;

        fn new() -> Self {
            let (tx, _rx) = std::sync::mpsc::channel();
            Self {
                active: Default::default(),
                beats_per_bar: std::cell::Cell::new(4),
                beats_per_minute: std::cell::Cell::new(100),
                beat_in_bar: Default::default(),
                start_time: std::cell::Cell::new(Instant::now()),
                clicker: Default::default(),
                thread_cmd: std::cell::RefCell::new(tx),
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
            if active {
                let mut beat_in_bar = self.beat_in_bar.get();
                let beats_per_bar = self.beats_per_bar.get();
                let ns_per_beat = 60_000_000_000 / (self.beats_per_minute.get() as u64);
                let clicker = &self.clicker;
                let (tx, rx) = std::sync::mpsc::channel();
                self.thread_cmd.set(tx);
                thread::spawn(clone!(@strong clicker => move || {
                    let period = std::time::Duration::from_millis(1);
                    let ticktime = std::time::Duration::from_nanos(ns_per_beat);
                    let mut lasttick = std::time::Instant::now() - ticktime;

                    loop {
                        let msg = rx.recv_timeout(period);
                        match msg {
                            Ok(command) => match command {
                                TimerCommand::Stop => break,
                            },
                            Err(_e) => (),
                        }
                        let elapsed = lasttick.elapsed();
                        if elapsed > ticktime {
                            if beat_in_bar == 0 {
                                clicker.high();
                            } else {
                                clicker.low();
                            }
                            beat_in_bar = (beat_in_bar + 1) % beats_per_bar;
                            lasttick += ticktime;
                        }
                    }
                }));

                self.beat_in_bar.set(0);
            } else {
                self.thread_cmd.borrow().send(TimerCommand::Stop);
            }
        }
    }
}

glib::wrapper! {
    pub struct MtrTimer(ObjectSubclass<imp::MtrTimer>);
}
