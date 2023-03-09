use adw::subclass::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{glib, graphene};
use std::cell::Cell;

mod imp {
    use super::*;

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

    impl ObjectImpl for MtrTimerButtonMark {
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

    impl WidgetImpl for MtrTimerButtonMark {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;
            let style_ctx = widget.style_context();
            let fg_color = style_ctx.color();
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

impl MtrTimerButtonMark {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
