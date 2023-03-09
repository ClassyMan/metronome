use adw::subclass::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{glib, graphene};
use std::cell::Cell;

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct MtrTimerButtonMark {
        pub angle: Cell<f32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTimerButtonMark {
        const NAME: &'static str = "MtrTimerButtonMark";
        type Type = super::MtrTimerButtonMark;
        type ParentType = gtk::Widget;

        fn new() -> Self {
            Self {
                angle: std::cell::Cell::<f32>::new(0.0),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("timerbuttonmark");
        }
    }

    impl ObjectImpl for MtrTimerButtonMark {}

    impl WidgetImpl for MtrTimerButtonMark {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;
            let style_ctx = widget.style_context();
            let fg_color = style_ctx.color();
            snapshot.rotate(self.angle.get() + 180.0);
            snapshot.append_color(
                &fg_color,
                &graphene::Rect::new(0.0, 0.0, width, height - 1.0),
            );
            self.parent_snapshot(snapshot);
        }
    }
}

glib::wrapper! {
    pub struct MtrTimerButtonMark(ObjectSubclass<imp::MtrTimerButtonMark>)
        @extends gtk::Widget;
}

impl MtrTimerButtonMark {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_angle(&self, angle: f32) {
        let imp = imp::MtrTimerButtonMark::from_instance(&self);
        imp.angle.set(angle);
    }
}
