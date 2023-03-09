use crate::config;
use crate::window::MtrApplicationWindow;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{
    gdk, gio,
    glib::{self, clone},
};

mod imp {
    use super::*;
    use glib::WeakRef;
    use once_cell::sync::OnceCell;

    #[derive(Debug, Default)]
    pub struct MtrApplication {
        pub window: OnceCell<WeakRef<MtrApplicationWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrApplication {
        const NAME: &'static str = "MtrApplication";
        type Type = super::MtrApplication;
        type ParentType = gtk::Application;
    }

    impl ObjectImpl for MtrApplication {}

    impl gio::subclass::prelude::ApplicationImpl for MtrApplication {
        fn activate(&self) {
            log::debug!("GtkApplication<MtrApplication>::activate");
            self.parent_activate();
            let app = self.obj();
            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.show();
                window.present();
                return;
            }

            app.set_resource_base_path(Some("/com/adrienplazas/Metronome/"));
            app.setup_css();

            let window = MtrApplicationWindow::new(&app);
            self.window
                .set(window.downgrade())
                .expect("Window already set.");

            app.get_main_window().present();
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
}

glib::wrapper! {
    pub struct MtrApplication(ObjectSubclass<imp::MtrApplication>)
        @extends gio::Application, gtk::Application, @implements gio::ActionMap, gio::ActionGroup;
}

impl MtrApplication {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", config::APP_ID)
            .build()
    }

    fn get_main_window(&self) -> MtrApplicationWindow {
        let priv_ = imp::MtrApplication::from_instance(self);
        priv_.window.get().unwrap().upgrade().unwrap()
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

        self.add_action_entries([quit_action, about_action]);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<primary>q"]);
        self.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
        self.set_accels_for_action("win.tap", &["t"]);
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/com/adrienplazas/Metronome/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk::StyleContext::add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn show_about_dialog(&self) {
        let dialog = gtk::AboutDialog::builder()
            .program_name("Metronome")
            .logo_icon_name(config::APP_ID)
            .license_type(gtk::License::Gpl30)
            .website("https://gitlab.gnome.org/aplazas/metronome/")
            .version(config::VERSION)
            .transient_for(&self.get_main_window())
            .modal(true)
            .authors(vec!["Adrien Plazas <kekun.plazas@laposte.net>"])
            .artists(vec!["Tobias Bernard <tbernard@gnome.org>"])
            .build();

        dialog.show();
    }

    pub fn run(&self) {
        log::info!("Metronome ({})", config::APP_ID);
        log::info!("Version: {} ({})", config::VERSION, config::PROFILE);
        log::info!("Datadir: {}", config::PKGDATADIR);

        ApplicationExtManual::run(self);
    }
}
