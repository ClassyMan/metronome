mod application;
mod chord_builder;
#[rustfmt::skip]
mod clicker;
mod fretboard;
mod guitar_player;
mod config;
mod scale_data;
mod scales_page;
mod theme;
mod theme_dialog;
mod theme_editor;
mod theme_manager;
mod timer;
mod timerbutton;
mod timerbuttonmark;
mod timerbuttontrough;
mod window;

use application::MtrApplication;
use config::{GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};
use gettextrs::*;
use gtk::{gio, glib};

fn main() -> glib::ExitCode {
    // Initialize logger, debug is carried out via debug!, info!, and warn!.
    pretty_env_logger::init();

    // Prepare i18n
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    gtk::glib::set_application_name(&gettext("Metronome"));
    gtk::glib::set_prgname(Some("metronome"));

    gst::init().expect("Unable to start GStreamer");

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = MtrApplication::default();
    app.run()
}
