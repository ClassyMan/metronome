use crate::clicker::MtrClicker;
use adw::subclass::prelude::*;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::thread;

enum TimerCommand {
    Stop,
    Bpm(u32),
    BeatsPerBar(u32),
    TempoRamp {
        enabled: bool,
        increment: u32,
        bars_interval: u32,
        target_bpm: u32,
    },
}

mod imp {
    use super::*;
    use crate::window::{
        BPB_DEFAULT, BPB_MAX, BPB_MIN, BPM_DEFAULT, BPM_MAX, BPM_MIN, RAMP_BARS_DEFAULT,
        RAMP_BARS_MAX, RAMP_BARS_MIN, RAMP_INCREMENT_DEFAULT, RAMP_INCREMENT_MAX,
        RAMP_INCREMENT_MIN, VOLUME_DEFAULT, VOLUME_MAX, VOLUME_MIN,
    };
    use std::cell::{Cell, RefCell};

    #[derive(Debug, glib::Properties)]
    #[properties(wrapper_type = super::MtrTimer)]
    pub struct MtrTimer {
        #[property(get, set = Self::set_active)]
        pub active: Cell<bool>,
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
        #[property(get, set)]
        pub ramp_status: RefCell<String>,
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
                beats_per_bar: Cell::new(BPB_DEFAULT),
                beats_per_minute: Cell::new(BPM_DEFAULT),
                tempo_ramp_enabled: Cell::new(false),
                tempo_ramp_increment: Cell::new(RAMP_INCREMENT_DEFAULT),
                tempo_ramp_bars: Cell::new(RAMP_BARS_DEFAULT),
                tempo_ramp_target: Cell::new(BPM_MAX),
                volume: Cell::new(VOLUME_DEFAULT),
                ramp_status: RefCell::new(String::new()),
                clicker: Default::default(),
                thread_cmd: RefCell::new(tx),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MtrTimer {}

    impl MtrTimer {
        fn set_active(&self, active: bool) {
            self.active.set(active);
            if !active {
                self.thread_cmd
                    .borrow()
                    .send(TimerCommand::Stop)
                    .unwrap_or_default();
                self.ramp_status.replace(String::new());
                self.obj().notify_ramp_status();
                return;
            }

            let mut beats_per_bar = self.beats_per_bar.get();
            let mut current_bpm = self.beats_per_minute.get();
            let ns_per_beat = 60_000_000_000 / (current_bpm as u64);
            let clicker = &self.clicker;
            let (tx, rx) = std::sync::mpsc::channel();
            self.thread_cmd.set(tx);

            let mut ramp_enabled = self.tempo_ramp_enabled.get();
            let mut ramp_increment = self.tempo_ramp_increment.get();
            let mut ramp_bars_interval = self.tempo_ramp_bars.get();
            let mut ramp_target_bpm = self.tempo_ramp_target.get();

            let timer_weak = glib::SendWeakRef::from(self.obj().downgrade());

            thread::spawn(clone!(@strong clicker => move || {
                let recv_period = std::time::Duration::from_millis(1);
                let mut ticktime = std::time::Duration::from_nanos(ns_per_beat);
                let mut lastiter = std::time::Instant::now() - ticktime;
                let mut bar_position = 0.0;
                let mut beat_in_bar = 0u32;
                let mut bars_completed = 0u32;

                loop {
                    let msg = rx.recv_timeout(recv_period);
                    match msg {
                        Ok(TimerCommand::Stop) => break,
                        Ok(TimerCommand::Bpm(bpm)) => {
                            if bpm != current_bpm {
                                current_bpm = bpm;
                                ticktime = std::time::Duration::from_nanos(
                                    60_000_000_000 / bpm as u64,
                                );
                                bars_completed = 0;
                            }
                        }
                        Ok(TimerCommand::BeatsPerBar(bpb)) => {
                            beat_in_bar = 0;
                            beats_per_bar = bpb;
                            bars_completed = 0;
                        }
                        Ok(TimerCommand::TempoRamp {
                            enabled,
                            increment,
                            bars_interval,
                            target_bpm,
                        }) => {
                            ramp_enabled = enabled;
                            ramp_increment = increment;
                            ramp_bars_interval = bars_interval;
                            ramp_target_bpm = target_bpm;
                            bars_completed = 0;
                        }
                        Err(_) => {}
                    }
                    let elapsed = lastiter.elapsed();
                    lastiter = std::time::Instant::now();
                    bar_position += elapsed.as_secs_f64() / ticktime.as_secs_f64();
                    if bar_position > 1.0 {
                        if beat_in_bar == 0 && beats_per_bar > 1 {
                            clicker.high();
                        } else {
                            clicker.low();
                        }
                        beat_in_bar = (beat_in_bar + 1) % beats_per_bar;
                        bar_position -= 1.0;

                        if beat_in_bar == 0 {
                            bars_completed += 1;

                            if ramp_enabled {
                                if bars_completed >= ramp_bars_interval
                                    && current_bpm < ramp_target_bpm
                                {
                                    bars_completed = 0;
                                    let new_bpm = (current_bpm + ramp_increment)
                                        .min(ramp_target_bpm)
                                        .min(BPM_MAX);
                                    if new_bpm != current_bpm {
                                        current_bpm = new_bpm;
                                        ticktime = std::time::Duration::from_nanos(
                                            60_000_000_000 / current_bpm as u64,
                                        );
                                        let weak = timer_weak.clone();
                                        let next_bpm = (current_bpm + ramp_increment)
                                            .min(ramp_target_bpm)
                                            .min(BPM_MAX);
                                        let status = if current_bpm >= ramp_target_bpm {
                                            format!("Reached target: {} BPM", current_bpm)
                                        } else {
                                            format!(
                                                "Bar {}/{} \u{2022} next: {} BPM",
                                                bars_completed + 1,
                                                ramp_bars_interval,
                                                next_bpm
                                            )
                                        };
                                        glib::MainContext::default().invoke(move || {
                                            if let Some(timer) = weak.upgrade() {
                                                timer.set_beats_per_minute(new_bpm);
                                                timer.set_ramp_status(status);
                                            }
                                        });
                                    }
                                } else {
                                    let weak = timer_weak.clone();
                                    let bars_done = bars_completed;
                                    let bars_total = ramp_bars_interval;
                                    let next_bpm = (current_bpm + ramp_increment)
                                        .min(ramp_target_bpm)
                                        .min(BPM_MAX);
                                    let at_target = current_bpm >= ramp_target_bpm;
                                    let status = if at_target {
                                        format!("Reached target: {} BPM", current_bpm)
                                    } else {
                                        format!(
                                            "Bar {}/{} \u{2022} next: {} BPM",
                                            bars_done, bars_total, next_bpm
                                        )
                                    };
                                    glib::MainContext::default().invoke(move || {
                                        if let Some(timer) = weak.upgrade() {
                                            timer.set_ramp_status(status);
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            }));
        }

        fn send_ramp_command(&self) {
            self.thread_cmd
                .borrow()
                .send(TimerCommand::TempoRamp {
                    enabled: self.tempo_ramp_enabled.get(),
                    increment: self.tempo_ramp_increment.get(),
                    bars_interval: self.tempo_ramp_bars.get(),
                    target_bpm: self.tempo_ramp_target.get(),
                })
                .unwrap_or_default();
        }

        fn set_beats_per_bar(&self, bpb: u32) {
            self.beats_per_bar.set(bpb);
            self.thread_cmd
                .borrow()
                .send(TimerCommand::BeatsPerBar(bpb))
                .unwrap_or_default();
        }

        fn set_beats_per_minute(&self, bpm: u32) {
            self.beats_per_minute.set(bpm);
            self.thread_cmd
                .borrow()
                .send(TimerCommand::Bpm(bpm))
                .unwrap_or_default();
        }

        fn set_tempo_ramp_enabled(&self, val: bool) {
            self.tempo_ramp_enabled.set(val);
            self.send_ramp_command();
        }

        fn set_tempo_ramp_increment(&self, val: u32) {
            self.tempo_ramp_increment.set(val);
            self.send_ramp_command();
        }

        fn set_tempo_ramp_bars(&self, val: u32) {
            self.tempo_ramp_bars.set(val);
            self.send_ramp_command();
        }

        fn set_tempo_ramp_target(&self, val: u32) {
            self.tempo_ramp_target.set(val);
            self.send_ramp_command();
        }

        fn set_volume(&self, volume: f64) {
            self.volume.set(volume);
            self.clicker.set_volume(volume);
        }
    }
}

glib::wrapper! {
    pub struct MtrTimer(ObjectSubclass<imp::MtrTimer>);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::{BPB_DEFAULT, BPB_MAX, VOLUME_DEFAULT};
    use glib::subclass::prelude::ObjectSubclassIsExt;

    fn ensure_runtime() {
        gst::init().expect("GStreamer must initialize for timer tests");
    }

    #[test]
    fn test_volume_defaults_to_full() {
        ensure_runtime();
        let timer: MtrTimer = glib::Object::new();
        assert!(
            (timer.volume() - VOLUME_DEFAULT).abs() < f64::EPSILON,
            "expected {VOLUME_DEFAULT}, got {}",
            timer.volume()
        );
    }

    #[test]
    fn test_set_volume_propagates_to_clicker() {
        ensure_runtime();
        let timer: MtrTimer = glib::Object::new();
        timer.set_volume(0.42);
        let clicker_volume = timer.imp().clicker.imp().player.volume();
        assert!(
            (clicker_volume - 0.42).abs() < f64::EPSILON,
            "volume did not propagate: expected 0.42, got {clicker_volume}"
        );
    }

    #[test]
    fn test_beats_per_bar_defaults_to_four() {
        ensure_runtime();
        let timer: MtrTimer = glib::Object::new();
        assert_eq!(timer.beats_per_bar(), BPB_DEFAULT);
    }

    #[test]
    fn test_beats_per_bar_accepts_values_above_old_limit() {
        ensure_runtime();
        let timer: MtrTimer = glib::Object::new();
        timer.set_beats_per_bar(24);
        assert_eq!(timer.beats_per_bar(), 24);
    }

    #[test]
    fn test_beats_per_bar_accepts_max() {
        ensure_runtime();
        let timer: MtrTimer = glib::Object::new();
        timer.set_beats_per_bar(BPB_MAX);
        assert_eq!(timer.beats_per_bar(), BPB_MAX);
    }

    #[test]
    fn test_volume_change_does_not_affect_bpb() {
        ensure_runtime();
        let timer: MtrTimer = glib::Object::new();
        timer.set_beats_per_bar(7);
        timer.set_volume(0.3);
        assert_eq!(timer.beats_per_bar(), 7);
    }
}
