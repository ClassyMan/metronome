use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{glib, graphene};

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::MtrTimerButtonMark)]
    pub struct MtrTimerButtonMark {
        #[property(get, set)]
        pub angle: Cell<f32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTimerButtonMark {
        const NAME: &'static str = "MtrTimerButtonMark";
        type Type = super::MtrTimerButtonMark;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("timerbuttonmark");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MtrTimerButtonMark {}

    impl WidgetImpl for MtrTimerButtonMark {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;
            let fg_color = widget.color();
            snapshot.rotate(widget.angle() + 180.0);
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

impl Default for MtrTimerButtonMark {
    fn default() -> Self {
        glib::Object::new()
    }
}
