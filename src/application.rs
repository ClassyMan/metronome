use crate::config;
use crate::window::MtrApplicationWindow;
use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;
    use glib::WeakRef;
    use std::cell::OnceCell;

    #[derive(Debug, Default)]
    pub struct MtrApplication {
        pub window: OnceCell<WeakRef<MtrApplicationWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrApplication {
        const NAME: &'static str = "MtrApplication";
        type Type = super::MtrApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for MtrApplication {}

    impl ApplicationImpl for MtrApplication {
        fn activate(&self) {
            log::debug!("GtkApplication<MtrApplication>::activate");
            self.parent_activate();

            // Set icons for shell
            gtk::Window::set_default_icon_name(config::APP_ID);

            let app = self.obj();
            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.present();
                return;
            }

            let window = MtrApplicationWindow::new(&app);
            self.window
                .set(window.downgrade())
                .expect("Window already set.");

            window.present();
        }

        fn startup(&self) {
            log::debug!("GtkApplication<MtrApplication>::startup");
            self.parent_startup();
            let app = self.obj();

            app.setup_gactions();
            app.setup_accels();
        }
    }

    impl GtkApplicationImpl for MtrApplication {}
    impl AdwApplicationImpl for MtrApplication {}
}

glib::wrapper! {
    pub struct MtrApplication(ObjectSubclass<imp::MtrApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl MtrApplication {
    fn get_main_window(&self) -> MtrApplicationWindow {
        self.imp().window.get().unwrap().upgrade().unwrap()
    }

    fn setup_gactions(&self) {
        // Quit
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(|app: &Self, _, _| {
                // This is needed to trigger the delete event
                // and saving the window state
                app.get_main_window().close();
                app.quit();
            })
            .build();

        // About
        let about_action = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| {
                app.show_about_dialog();
            })
            .build();

        //Start and stop playing
        let sound_control_action = gio::ActionEntry::builder("sound-control")
            .activate(|app: &Self, _, _| {
                let window = app.get_main_window();
                let timer = &window.imp().timer_button;
                if timer.active() {
                    timer.pause();
                } else {
                    timer.start();
                }
            })
            .build();

        self.add_action_entries([quit_action, about_action, sound_control_action]);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<primary>q"]);
        self.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
        self.set_accels_for_action("win.tap", &["t"]);
        self.set_accels_for_action("app.sound-control", &["space"]);
    }

    fn show_about_dialog(&self) {
        adw::AboutWindow::builder()
            .application_name("Metronome")
            .application_icon(config::APP_ID)
            .license_type(gtk::License::Gpl30)
            .website("https://gitlab.gnome.org/World/metronome/")
            .issue_url("https://gitlab.gnome.org/World/metronome/-/issues")
            .version(config::VERSION)
            .transient_for(&self.get_main_window())
            .modal(true)
            .developers(vec![
                "Adrien Plazas <kekun.plazas@laposte.net>",
                "Clara Hobbs <clara@clarahobbs.com>",
                "FineFindus https://gitlab.gnome.org/FineFindus",
            ])
            .artists(vec!["Tobias Bernard <tbernard@gnome.org>"])
            .build()
            .present();
    }

    pub fn run(&self) -> glib::ExitCode {
        log::info!("Metronome ({})", config::APP_ID);
        log::info!("Version: {} ({})", config::VERSION, config::PROFILE);
        log::info!("Datadir: {}", config::PKGDATADIR);

        ApplicationExtManual::run(self)
    }
}

impl Default for MtrApplication {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", config::APP_ID)
            .property("resource-base-path", "/com/adrienplazas/Metronome/")
            .build()
    }
}
