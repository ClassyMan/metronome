use crate::config;
use crate::window::MtrApplicationWindow;
use adw::prelude::AdwDialogExt;
use adw::subclass::prelude::*;
use gettextrs::gettext;
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

        // Test-friendly actions for E2E automation via D-Bus
        let set_chord = gio::SimpleAction::new("set-chord-structure", Some(glib::VariantTy::UINT32));
        let weak_app = self.downgrade();
        set_chord.connect_activate(move |_, param| {
            if let Some(app) = weak_app.upgrade() {
                let idx = param.unwrap().get::<u32>().unwrap();
                app.get_main_window().imp().scales_page.set_chord_structure(idx);
            }
        });
        self.add_action(&set_chord);

        let tap_fret = gio::SimpleAction::new("tap-fret", Some(glib::VariantTy::new("(uu)").unwrap()));
        let weak_app = self.downgrade();
        tap_fret.connect_activate(move |_, param| {
            if let Some(app) = weak_app.upgrade() {
                let (string_idx, fret) = param.unwrap().get::<(u32, u32)>().unwrap();
                app.get_main_window().imp().scales_page.tap_fret(string_idx as usize, fret as usize);
            }
        });
        self.add_action(&tap_fret);

        let load_tab = gio::SimpleAction::new("load-tab-file", Some(glib::VariantTy::STRING));
        let weak_app = self.downgrade();
        load_tab.connect_activate(move |_, param| {
            if let Some(app) = weak_app.upgrade() {
                let path_str = param.unwrap().get::<String>().unwrap();
                let path = std::path::Path::new(&path_str);
                app.get_main_window().imp().tab_player_page.load_file(path);
            }
        });
        self.add_action(&load_tab);

        let set_loop = gio::SimpleAction::new("set-tab-loop-bar", Some(glib::VariantTy::UINT32));
        let weak_app = self.downgrade();
        set_loop.connect_activate(move |_, param| {
            if let Some(app) = weak_app.upgrade() {
                let beat_index = param.unwrap().get::<u32>().unwrap();
                let window = app.get_main_window();
                window.imp().tab_player_page.set_loop_on_bar(beat_index as usize);
            }
        });
        self.add_action(&set_loop);

        let clear_loop = gio::SimpleAction::new("clear-tab-loop", None);
        let weak_app = self.downgrade();
        clear_loop.connect_activate(move |_, _| {
            if let Some(app) = weak_app.upgrade() {
                let window = app.get_main_window();
                window.imp().tab_player_page.clear_loop();
            }
        });
        self.add_action(&clear_loop);

        let get_scroll = gio::SimpleAction::new("get-tab-scroll-info", None);
        let weak_app = self.downgrade();
        get_scroll.connect_activate(move |_, _| {
            if let Some(app) = weak_app.upgrade() {
                let window = app.get_main_window();
                let page = &window.imp().tab_player_page;
                let adj = page.imp().tab_strip_scroll.hadjustment();
                let child_width = page.imp().tab_strip.width();
                let scroll_width = page.imp().tab_strip_scroll.width();
                let (min_w, nat_w, _, _) = page.imp().tab_strip.measure(gtk::Orientation::Horizontal, -1);
                let _ = min_w;
                log::info!(
                    "TAB SCROLL: value={}, upper={}, page_size={}, child_w={}, scroll_w={}, natural_w={}",
                    adj.value(), adj.upper(), adj.page_size(),
                    child_width, scroll_width, nat_w
                );
            }
        });
        self.add_action(&get_scroll);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<primary>q"]);
        self.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
        self.set_accels_for_action("win.tap", &["t"]);
        self.set_accels_for_action("app.sound-control", &["space"]);
    }

    fn show_about_dialog(&self) {
        let mut details = String::new();
        details.push_str("<b>");
        details.push_str(&gettext("Keep the tempo"));
        details.push_str("</b>\n");
        details.push_str(&gettext("Metronome beats the rhythm for you, you simply need to tell it the required time signature and beats per minutes.\n"));
        details.push_str(&gettext(
            "You can also tap to let the application guess the required beats per minute",
        ));

        adw::AboutDialog::builder()
            .application_name("Metronome")
            .application_icon(config::APP_ID)
            .license_type(gtk::License::Gpl30)
            .website("https://gitlab.gnome.org/World/metronome/")
            .issue_url("https://gitlab.gnome.org/World/metronome/-/issues")
            .version(config::VERSION)
            .comments(details)
            .developers(vec![
                "Adrien Plazas <kekun.plazas@laposte.net>",
                "Clara Hobbs <clara@clarahobbs.com>",
                "FineFindus https://gitlab.gnome.org/FineFindus",
            ])
            .artists(vec!["Tobias Bernard <tbernard@gnome.org>"])
            // Translators: Please enter your credits here (format: "Name https://example.com" or "Name <email@example.com>", no quotes)
            .translator_credits(gettext("translator-credits"))
            .build()
            .present(&self.get_main_window());
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
