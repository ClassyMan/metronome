use adw::subclass::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gdk, glib, graphene, gsk};
use once_cell::sync::Lazy;
use std::cell::Cell;

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct MtrTimerButtonTrough {
        pub progress: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTimerButtonTrough {
        const NAME: &'static str = "MtrTimerButtonTrough";
        type Type = super::MtrTimerButtonTrough;
        type ParentType = gtk::Widget;

        fn new() -> Self {
            Self {
                progress: std::cell::Cell::<f64>::new(0.0),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
            klass.set_css_name("timerbuttontrough");
        }
    }

    impl ObjectImpl for MtrTimerButtonTrough {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecDouble::new(
                    "progress",
                    "Progress",
                    "Progress",
                    i32::MIN as f64,
                    i32::MAX as f64,
                    0.0,
                    glib::ParamFlags::READWRITE,
                )]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "progress" => self.progress.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let obj = self.obj();
            match pspec.name() {
                "progress" => obj.set_progress(value.get::<f64>().unwrap()),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for MtrTimerButtonTrough {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f64;
            let height = widget.height() as f64;
            let style_ctx = widget.style_context();

            let fg_color = style_ctx.color();
            let transparent = gdk::RGBA::new(0.0, 0.0, 0.0, 0.0);

            let progress = self.progress.get() as f32;

            let fill = gsk::ColorStop::new(progress.fract(), fg_color);
            let void = gsk::ColorStop::new(progress.fract(), transparent);
            let stops = if (progress.trunc() as i32) % 2 == 0 {
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

            self.parent_snapshot(snapshot);
        }
    }
}

glib::wrapper! {
    pub struct MtrTimerButtonTrough(ObjectSubclass<imp::MtrTimerButtonTrough>)
        @extends gtk::Widget;
}

impl MtrTimerButtonTrough {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_progress(&self, progress: f64) {
        let imp = imp::MtrTimerButtonTrough::from_instance(&self);

        imp.progress.set(progress);

        self.queue_draw();
    }
}
