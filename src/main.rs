#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod application;
#[rustfmt::skip]
mod clicker;
mod config;
#[cfg(target_os = "windows")]
mod portable_settings;
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
#[cfg(target_os = "windows")]
use config::RESOURCES_FILE;
#[cfg(not(target_os = "windows"))]
use config::{GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};
#[cfg(not(target_os = "windows"))]
use gettextrs::*;
use gtk::{gio, glib};

fn main() -> glib::ExitCode {
    pretty_env_logger::init();

    #[cfg(not(target_os = "windows"))]
    {
        setlocale(LocaleCategory::LcAll, "");
        bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
        textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");
    }

    gtk::glib::set_application_name("Metronome");
    gtk::glib::set_prgname(Some("metronome"));

    #[cfg(target_os = "windows")]
    {
        let exe_dir = std::env::current_exe()
            .expect("Cannot find executable path")
            .parent()
            .expect("Cannot find executable directory")
            .to_path_buf();
        std::env::set_var("GST_PLUGIN_PATH", exe_dir.join("lib").join("gstreamer-1.0"));
        std::env::set_var("GST_PLUGIN_SYSTEM_PATH", "");
        std::env::set_var(
            "GDK_PIXBUF_MODULE_FILE",
            exe_dir.join("lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"),
        );
        std::env::set_var("XDG_DATA_DIRS", exe_dir.join("share"));
        std::env::set_var(
            "GSETTINGS_SCHEMA_DIR",
            exe_dir.join("share/glib-2.0/schemas"),
        );
    }

    gst::init().expect("Unable to start GStreamer");

    #[cfg(target_os = "windows")]
    let resource_path = {
        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        exe_dir.join(RESOURCES_FILE)
    };
    #[cfg(not(target_os = "windows"))]
    let resource_path = std::path::PathBuf::from(RESOURCES_FILE);

    let res = gio::Resource::load(&resource_path).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = MtrApplication::default();
    app.run()
}
