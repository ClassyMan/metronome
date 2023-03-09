use adw::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk, prelude::*};

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::MtrTimerButtonTrough)]
    pub struct MtrTimerButtonTrough {
        #[property(get, set = Self::set_progress)]
        pub progress: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTimerButtonTrough {
        const NAME: &'static str = "MtrTimerButtonTrough";
        type Type = super::MtrTimerButtonTrough;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
            klass.set_css_name("timerbuttontrough");
        }
    }

    impl ObjectImpl for MtrTimerButtonTrough {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }
    }

    impl WidgetImpl for MtrTimerButtonTrough {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f64;
            let height = widget.height() as f64;
            let fg_color = widget.color();
            let transparent = gdk::RGBA::new(0.0, 0.0, 0.0, 0.0);

            let progress = widget.progress() as f32;

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

    impl MtrTimerButtonTrough {
        fn set_progress(&self, progress: f64) {
            self.progress.set(progress);
            self.obj().queue_draw();
        }
    }
}

glib::wrapper! {
    pub struct MtrTimerButtonTrough(ObjectSubclass<imp::MtrTimerButtonTrough>)
        @extends gtk::Widget;
}
