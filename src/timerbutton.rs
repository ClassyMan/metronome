use crate::timerbuttonmark::MtrTimerButtonMark;
use adw::subclass::prelude::*;
use glib::clone;
use glib::ParamSpec;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gdk, glib, graphene, gsk, CompositeTemplate};
use once_cell::sync::Lazy;
use std::cell::{Cell, RefCell};
use std::time::Instant;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/adrienplazas/Metronome/ui/timerbutton.ui")]
    pub struct MtrTimerButton {
        #[template_child]
        pub start_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub marks_overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        pub beats_per_bar: Cell<u32>,
        pub beats_per_minute: Cell<u32>,
        pub start_time: Cell<Instant>,
        pub running_id: RefCell<Option<gtk::TickCallbackId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTimerButton {
        const NAME: &'static str = "MtrTimerButton";
        type Type = super::MtrTimerButton;
        type ParentType = gtk::Widget;

        fn new() -> Self {
            Self {
                start_button: TemplateChild::default(),
                pause_button: TemplateChild::default(),
                marks_overlay: TemplateChild::default(),
                stack: TemplateChild::default(),
                beats_per_bar: std::cell::Cell::<u32>::new(4),
                beats_per_minute: std::cell::Cell::<u32>::new(100),
                start_time: std::cell::Cell::<Instant>::new(Instant::now()),
                running_id: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_layout_manager_type::<gtk::BinLayout>();
            klass.set_css_name("timerbutton");
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MtrTimerButton {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.setup_signals();
            obj.update_marks();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_boolean(
                        "active",
                        "Active",
                        "Active",
                        false,
                        glib::ParamFlags::READABLE,
                    ),
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

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "active" => obj.active().to_value(),
                "beats-per-bar" => self.beats_per_bar.get().to_value(),
                "beats-per-minute" => self.beats_per_minute.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "beats-per-bar" => obj.set_beats_per_bar(value.get::<u32>().unwrap()),
                "beats-per-minute" => obj.set_beats_per_minute(value.get::<u32>().unwrap()),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for MtrTimerButton {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            let width = widget.width() as f64;
            let height = widget.height() as f64;
            let style_ctx = widget.style_context();

            let s_per_beat = 60.0 / self.beats_per_minute.get() as f64;
            let s_per_bar = s_per_beat * self.beats_per_bar.get() as f64;

            let now = Instant::now();
            let elapsed = now - self.start_time.get();

            let (progress, bar) = if self.running_id.borrow().is_some() {
                let bar = elapsed.as_secs_f64() / s_per_bar;
                (bar.fract() as f32, bar.trunc() as i32)
            } else {
                (0.0, 0)
            };

            let fg_color = style_ctx.color();
            let transparent = gdk::RGBA {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.0,
            };

            let fill = gsk::ColorStop::new(progress, fg_color);
            let void = gsk::ColorStop::new(progress, transparent);
            let stops = if bar % 2 == 0 {
                [fill, void]
            } else {
                [void, fill]
            };

            snapshot.append_conic_gradient(
                &graphene::Rect::new(0.0, 0.0, width as f32, height as f32),
                &graphene::Point::new(width as f32 / 2.0, height as f32 / 2.0),
                0.0,
                &stops,
            );

            self.parent_snapshot(widget, snapshot);
        }
    }
}

glib::wrapper! {
    pub struct MtrTimerButton(ObjectSubclass<imp::MtrTimerButton>)
        @extends gtk::Widget;
}

impl MtrTimerButton {
    pub fn new() -> Self {
        let this: Self = glib::Object::new(&[]).expect("Failed to create MtrTimerButton");

        this
    }

    fn update_marks(&self) {
        let imp = imp::MtrTimerButton::from_instance(&self);

        while let Some(child) = imp.marks_overlay.get().first_child() {
            child.unparent();
        }

        let beats_per_bar = imp.beats_per_bar.get();
        for i in 0..beats_per_bar {
            let mark = MtrTimerButtonMark::new();
            mark.set_angle(i as f32 * 360.0 / beats_per_bar as f32);
            imp.marks_overlay.get().add_overlay(&mark);
        }
    }

    fn set_beats_per_bar(&self, beats_per_bar: u32) {
        let imp = imp::MtrTimerButton::from_instance(&self);

        imp.beats_per_bar.set(beats_per_bar);
        self.pause();
        self.update_marks();
    }

    fn set_beats_per_minute(&self, beats_per_minute: u32) {
        let imp = imp::MtrTimerButton::from_instance(&self);

        imp.beats_per_minute.set(beats_per_minute);
        self.pause();
    }

    fn setup_signals(&self) {
        let imp = imp::MtrTimerButton::from_instance(&self);

        imp.start_button
            .connect_clicked(clone!(@strong self as this => move |_| {
                this.start();
            }));

        imp.pause_button
            .connect_clicked(clone!(@strong self as this => move |_| {
                this.pause();
            }));
    }

    pub fn active(&self) -> bool {
        let imp = imp::MtrTimerButton::from_instance(&self);
        match imp.stack.get().visible_child() {
            Some(child) => child == imp.pause_button.get(),
            None => false,
        }
    }

    fn start(&self) {
        let imp = imp::MtrTimerButton::from_instance(&self);

        imp.start_time.set(Instant::now());
        imp.stack.get().set_visible_child(&imp.pause_button.get());

        let source_id = self.add_tick_callback(move |this, _clock| {
            this.queue_draw();
            Continue(true)
        });

        imp.running_id.replace(Some(source_id));

        self.set_state_flags(gtk::StateFlags::CHECKED, false);

        self.notify("active");
    }

    fn pause(&self) {
        let imp = imp::MtrTimerButton::from_instance(&self);

        if let Some(id) = imp.running_id.take() {
            id.remove();
        }

        imp.stack.get().set_visible_child(&imp.start_button.get());

        self.unset_state_flags(gtk::StateFlags::CHECKED);

        self.notify("active");
    }
}
