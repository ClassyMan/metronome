use adw::subclass::prelude::*;
use gtk::glib;

static AUDIO_CLICKER_HIGH_URI: &str = "resource:///com/adrienplazas/Metronome/audio/clicker-high.ogg";
static AUDIO_CLICKER_LOW_URI: &str = "resource:///com/adrienplazas/Metronome/audio/clicker-low.ogg";

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct MtrClicker {
        pub player: gstreamer_play::Play,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrClicker {
        const NAME: &'static str = "MtrClicker";
        type Type = super::MtrClicker;

        fn new() -> Self {
            Self {
                player: gstreamer_play::Play::new(
                    None::<gstreamer_play::PlayVideoRenderer>,
                ),
            }
        }
    }

    impl ObjectImpl for MtrClicker {}
}

glib::wrapper! {
    pub struct MtrClicker(ObjectSubclass<imp::MtrClicker>);
}

impl MtrClicker {
    pub fn high(&self) {
        let imp = self.imp();
        imp.player.set_uri(Some(AUDIO_CLICKER_HIGH_URI));
        imp.player.play();
    }

    pub fn low(&self) {
        let imp = self.imp();
        imp.player.set_uri(Some(AUDIO_CLICKER_LOW_URI));
        imp.player.play();
    }

    pub fn set_volume(&self, volume: f64) {
        self.imp().player.set_volume(volume);
    }
}

impl Default for MtrClicker {
    fn default() -> Self {
        glib::Object::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glib::subclass::prelude::ObjectSubclassIsExt;

    fn ensure_gstreamer() {
        gst::init().expect("GStreamer must initialize for audio tests");
    }

    #[test]
    fn test_set_volume_updates_gstreamer_player() {
        ensure_gstreamer();
        let clicker = MtrClicker::default();
        clicker.set_volume(0.5);
        let player_volume = clicker.imp().player.volume();
        assert!(
            (player_volume - 0.5).abs() < f64::EPSILON,
            "expected 0.5, got {player_volume}"
        );
    }

    #[test]
    fn test_volume_zero_mutes_player() {
        ensure_gstreamer();
        let clicker = MtrClicker::default();
        clicker.set_volume(0.0);
        let player_volume = clicker.imp().player.volume();
        assert!(
            player_volume.abs() < f64::EPSILON,
            "expected 0.0, got {player_volume}"
        );
    }

    #[test]
    fn test_volume_full_is_max() {
        ensure_gstreamer();
        let clicker = MtrClicker::default();
        clicker.set_volume(1.0);
        let player_volume = clicker.imp().player.volume();
        assert!(
            (player_volume - 1.0).abs() < f64::EPSILON,
            "expected 1.0, got {player_volume}"
        );
    }
}
