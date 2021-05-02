use adw::subclass::prelude::*;
use gtk::{glib, self};

static AUDIO_CLICKER_HIGH_URI: &str = "resource:///com/adrienplazas/Metronome/audio/clicker-high.ogg";
static AUDIO_CLICKER_LOW_URI: &str = "resource:///com/adrienplazas/Metronome/audio/clicker-low.ogg";

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct MtrClicker {
        pub player: gst_player::Player,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrClicker {
        const NAME: &'static str = "MtrClicker";
        type Type = super::MtrClicker;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                player: gst_player::Player::new(None, None),
            }
        }
    }

    impl ObjectImpl for MtrClicker {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }
}

glib::wrapper! {
    pub struct MtrClicker(ObjectSubclass<imp::MtrClicker>);
}

impl MtrClicker {
    pub fn new() -> Self {
        let this: Self =
            glib::Object::new(&[]).expect("Failed to create MtrClicker");

        this
    }

    pub fn high(&self) {
        let imp = imp::MtrClicker::from_instance(&self);
        imp.player.set_uri(AUDIO_CLICKER_HIGH_URI);
        imp.player.play();
    }

    pub fn low(&self) {
        let imp = imp::MtrClicker::from_instance(&self);
        imp.player.set_uri(AUDIO_CLICKER_LOW_URI);
        imp.player.play();
    }
}
