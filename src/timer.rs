use crate::clicker::MtrClicker;
use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::thread;

enum TimerCommand {
    Stop,
    BPM(u32),
    BeatsPerBar(u32),
}

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;
    use std::cell::{Cell, RefCell};

    #[derive(Debug, glib::Properties)]
    #[properties(wrapper_type = super::MtrTimer)]
    pub struct MtrTimer {
        #[property(get, set = Self::set_active)]
        pub active: Cell<bool>,
        #[property(get, set = Self::set_beats_per_bar, minimum = 1, maximum = 9, default = 4)]
        pub beats_per_bar: Cell<u32>,
        #[property(get, set = Self::set_beats_per_minute, minimum = 20, maximum = 260, default = 100)]
        pub beats_per_minute: Cell<u32>,
        pub clicker: MtrClicker,
        thread_cmd: RefCell<std::sync::mpsc::Sender<TimerCommand>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTimer {
        const NAME: &'static str = "MtrTimer";
        type Type = super::MtrTimer;

        fn new() -> Self {
            let (tx, _rx) = std::sync::mpsc::channel();
            Self {
                active: Default::default(),
                beats_per_bar: Cell::new(4),
                beats_per_minute: Cell::new(100),
                clicker: Default::default(),
                thread_cmd: RefCell::new(tx),
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
                let mut beats_per_bar = self.beats_per_bar.get();
                let ns_per_beat = 60_000_000_000 / (self.beats_per_minute.get() as u64);
                let clicker = &self.clicker;
                let (tx, rx) = std::sync::mpsc::channel();
                self.thread_cmd.set(tx);
                thread::spawn(clone!(@strong clicker => move || {
                    let recv_period = std::time::Duration::from_millis(1);
                    let mut ticktime = std::time::Duration::from_nanos(ns_per_beat);
                    let mut lastiter = std::time::Instant::now() - ticktime;
                    let mut bar_position = 0.0;
                    let mut beat_in_bar = 0;

                    loop {
                        let msg = rx.recv_timeout(recv_period);
                        match msg {
                            Ok(TimerCommand::Stop) => break,
                            Ok(TimerCommand::BPM(bpm)) => ticktime = std::time::Duration::from_nanos(60_000_000_000 / bpm as u64),
                            Ok(TimerCommand::BeatsPerBar(bpb)) => {
                                beat_in_bar = 0;
                                beats_per_bar = bpb;
                            },
                            Err(_) => {}
                        }
                        let elapsed = lastiter.elapsed();
                        lastiter = std::time::Instant::now();
                        bar_position += elapsed.as_secs_f64() / ticktime.as_secs_f64();
                        if bar_position > 1.0 {
                            if beat_in_bar == 0 {
                                clicker.high();
                            } else {
                                clicker.low();
                            }
                            beat_in_bar = (beat_in_bar + 1) % beats_per_bar;
                            bar_position -= 1.0;
                        }
                    }
                }));
            } else {
                self.thread_cmd
                    .borrow()
                    .send(TimerCommand::Stop)
                    .unwrap_or(());
            }
        }

        fn set_beats_per_bar(&self, bpb: u32) {
            self.beats_per_bar.set(bpb);
            self.thread_cmd
                .borrow()
                .send(TimerCommand::BeatsPerBar(bpb))
                .unwrap_or(());
        }

        fn set_beats_per_minute(&self, bpm: u32) {
            self.beats_per_minute.set(bpm);
            self.thread_cmd
                .borrow()
                .send(TimerCommand::BPM(bpm))
                .unwrap_or(());
        }
    }
}

glib::wrapper! {
    pub struct MtrTimer(ObjectSubclass<imp::MtrTimer>);
}
