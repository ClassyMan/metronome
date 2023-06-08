use crate::timerbuttonmark::MtrTimerButtonMark;
use crate::timerbuttontrough::MtrTimerButtonTrough;
use adw::subclass::prelude::*;
use gtk::{glib, prelude::*};
use std::time::Instant;

mod imp {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        marker::PhantomData,
    };

    #[derive(Debug, glib::Properties, gtk::CompositeTemplate)]
    #[template(resource = "/com/adrienplazas/Metronome/ui/timerbutton.ui")]
    #[properties(wrapper_type = super::MtrTimerButton)]
    pub struct MtrTimerButton {
        #[template_child]
        pub trough: TemplateChild<MtrTimerButtonTrough>,
        #[template_child]
        pub start_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub marks_overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        marks_container: TemplateChild<gtk::Box>,
        #[property(get, set = Self::set_beats_per_bar, minimum = 1, maximum = 9, default = 4)]
        pub beats_per_bar: Cell<u32>,
        #[property(get, set = Self::set_beats_per_minute, minimum = 20, maximum = 260, default = 100)]
        pub beats_per_minute: Cell<u32>,
        pub snapshot_time: Cell<Instant>,
        pub running_id: RefCell<Option<gtk::TickCallbackId>>,
        #[property(get = Self::active)]
        pub active: PhantomData<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTimerButton {
        const NAME: &'static str = "MtrTimerButton";
        type Type = super::MtrTimerButton;
        type ParentType = gtk::Widget;

        fn new() -> Self {
            Self {
                trough: Default::default(),
                start_button: Default::default(),
                pause_button: Default::default(),
                marks_overlay: Default::default(),
                stack: Default::default(),
                marks_container: Default::default(),
                beats_per_bar: std::cell::Cell::new(4),
                beats_per_minute: std::cell::Cell::new(100),
                snapshot_time: std::cell::Cell::new(Instant::now()),
                running_id: Default::default(),
                active: Default::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.set_layout_manager_type::<gtk::BinLayout>();
            klass.set_css_name("timerbutton");
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            MtrTimerButtonTrough::ensure_type();
            obj.init_template();
        }
    }

    impl ObjectImpl for MtrTimerButton {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.update_marks();
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

        fn dispose(&self) {
            self.dispose_template();
        }
    }

    impl WidgetImpl for MtrTimerButton {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let s_per_beat = 60.0 / widget.beats_per_minute() as f64;
            let s_per_bar = s_per_beat * widget.beats_per_bar() as f64;

            let elapsed = self.snapshot_time.get().elapsed();
            self.snapshot_time.set(Instant::now());

            let progress = if self.running_id.borrow().is_some() {
                let progress = self.trough.progress() + elapsed.as_secs_f64() / s_per_bar;
                // Perform a kind of floating point modulus between 0 and 2.
                progress.fract() + (progress as i32 % 2) as f64
            } else {
                0.0
            };

            self.trough.set_progress(progress);

            self.parent_snapshot(snapshot);
        }
    }

    impl MtrTimerButton {
        fn set_beats_per_bar(&self, beats_per_bar: u32) {
            let obj = self.obj();

            self.beats_per_bar.set(beats_per_bar);
            obj.pause();
            obj.update_marks();
        }

        fn set_beats_per_minute(&self, beats_per_minute: u32) {
            self.beats_per_minute.set(beats_per_minute);
        }

        pub fn active(&self) -> bool {
            match self.stack.visible_child() {
                Some(child) => child == self.pause_button.get(),
                None => false,
            }
        }
    }
}

glib::wrapper! {
    pub struct MtrTimerButton(ObjectSubclass<imp::MtrTimerButton>)
        @extends gtk::Widget;
}

#[gtk::template_callbacks]
impl MtrTimerButton {
    fn update_marks(&self) {
        let imp = self.imp();

        while let Some(child) = imp.marks_overlay.first_child() {
            child.unparent();
        }

        let beats_per_bar = self.beats_per_bar();
        for i in 0..beats_per_bar {
            let mark = MtrTimerButtonMark::default();
            mark.set_angle(i as f32 * 360.0 / beats_per_bar as f32);
            imp.marks_overlay.add_overlay(&mark);
        }
    }

    #[template_callback]
    pub fn start(&self) {
        let imp = self.imp();

        imp.snapshot_time.set(Instant::now());
        imp.stack.set_visible_child(&*imp.pause_button);

        let source_id = self.add_tick_callback(move |this, _clock| {
            this.queue_draw();
            Continue(true)
        });

        imp.running_id.replace(Some(source_id));

        self.set_state_flags(gtk::StateFlags::CHECKED, false);

        self.notify_active();
    }

    #[template_callback]
    pub fn pause(&self) {
        let imp = self.imp();

        if let Some(id) = imp.running_id.take() {
            id.remove();
        }

        imp.stack.set_visible_child(&*imp.start_button);

        self.unset_state_flags(gtk::StateFlags::CHECKED);

        self.notify_active();
    }
}
