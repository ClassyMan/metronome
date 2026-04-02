use crate::config::{APP_ID, PKGDATADIR};
use crate::gp5_parser;
use crate::gp7_parser;
use crate::tab_audio_thread::{BeatCallback, TabAudioCommand, TabAudioThread};
use crate::tab_fretboard::MtrTabFretboard;
use crate::tab_midi;
use crate::tab_models::TabScore;
use crate::tab_strip::MtrTabStrip;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::path::PathBuf;

const GUITAR_TONES: [(u32, &str); 3] = [
    (25, "Clean"),
    (29, "Crunch"),
    (30, "Lead"),
];

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/com/adrienplazas/Metronome/ui/tab-player-page.ui")]
    pub struct MtrTabPlayerPage {
        #[template_child]
        pub open_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub song_title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub time_sig_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub current_time_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub track_dropdown: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub play_pause_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub stop_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub skip_start_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub skip_end_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub prev_bar_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub next_bar_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub loop_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub metronome_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub tone_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub tempo_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub guitar_volume_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub metronome_volume_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub tab_strip_scroll: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub tab_strip: TemplateChild<MtrTabStrip>,
        #[template_child]
        pub tab_fretboard_scroll: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub tab_fretboard: TemplateChild<MtrTabFretboard>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,

        pub audio_thread: RefCell<Option<TabAudioThread>>,
        pub score: RefCell<Option<TabScore>>,
        pub file_path: RefCell<Option<PathBuf>>,
        #[template_child]
        pub status_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub dismiss_error_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub effective_bpm_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub count_in_button: TemplateChild<gtk::Button>,

        pub selected_track: Cell<usize>,
        pub tone_index: Cell<usize>,
        pub is_playing: Cell<bool>,
        pub metronome_enabled: Cell<bool>,
        pub current_beat: Cell<usize>,
        pub count_in_bars: Cell<u32>,
        pub settings: gio::Settings,
    }

    impl Default for MtrTabPlayerPage {
        fn default() -> Self {
            Self {
                open_button: Default::default(),
                song_title_label: Default::default(),
                time_sig_label: Default::default(),
                current_time_label: Default::default(),
                track_dropdown: Default::default(),
                play_pause_button: Default::default(),
                stop_button: Default::default(),
                skip_start_button: Default::default(),
                skip_end_button: Default::default(),
                prev_bar_button: Default::default(),
                next_bar_button: Default::default(),
                loop_button: Default::default(),
                metronome_button: Default::default(),
                tone_button: Default::default(),
                tempo_scale: Default::default(),
                guitar_volume_scale: Default::default(),
                metronome_volume_scale: Default::default(),
                tab_strip_scroll: Default::default(),
                tab_strip: Default::default(),
                tab_fretboard_scroll: Default::default(),
                tab_fretboard: Default::default(),
                status_label: Default::default(),
                audio_thread: RefCell::new(None),
                score: RefCell::new(None),
                file_path: RefCell::new(None),
                selected_track: Cell::new(0),
                tone_index: Cell::new(0),
                is_playing: Cell::new(false),
                status_box: Default::default(),
                dismiss_error_button: Default::default(),
                effective_bpm_label: Default::default(),
                count_in_button: Default::default(),
                metronome_enabled: Cell::new(false),
                current_beat: Cell::new(0),
                count_in_bars: Cell::new(0),
                settings: gio::Settings::new(APP_ID),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTabPlayerPage {
        const NAME: &'static str = "MtrTabPlayerPage";
        type Type = super::MtrTabPlayerPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            MtrTabStrip::ensure_type();
            MtrTabFretboard::ensure_type();
            klass.bind_template();
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MtrTabPlayerPage {
        fn constructed(&self) {
            self.parent_constructed();
            let page = self.obj().clone();
            page.setup_signals();

            // Configure ScrolledWindow viewports to use natural width
            for scrolled in [&*self.tab_strip_scroll, &*self.tab_fretboard_scroll] {
                if let Some(viewport) = scrolled.child().and_then(|child| child.downcast::<gtk::Viewport>().ok()) {
                    viewport.set_hscroll_policy(gtk::ScrollablePolicy::Natural);
                }
            }

            // Restore saved settings
            page.load_settings();

        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
            // Stop audio thread on dispose
            if let Some(ref audio_thread) = *self.audio_thread.borrow() {
                audio_thread.send(TabAudioCommand::Stop);
            }
        }
    }

    impl WidgetImpl for MtrTabPlayerPage {}
}

glib::wrapper! {
    pub struct MtrTabPlayerPage(ObjectSubclass<imp::MtrTabPlayerPage>)
        @extends gtk::Widget;
}

impl MtrTabPlayerPage {
    fn setup_signals(&self) {
        self.rebuild_open_menu();

        let page = self.clone();
        self.imp().play_pause_button.connect_clicked(move |_| {
            page.toggle_play_pause();
        });

        let page = self.clone();
        self.imp().stop_button.connect_clicked(move |_| {
            page.stop();
        });

        let page = self.clone();
        self.imp().skip_start_button.connect_clicked(move |_| {
            page.seek_to_beat(0);
        });

        let page = self.clone();
        self.imp().skip_end_button.connect_clicked(move |_| {
            if let Some(ref score) = *page.imp().score.borrow() {
                if let Some(last_bar) = score.bars.last() {
                    page.seek_to_beat(last_bar.first_beat_index);
                }
            }
        });

        let page = self.clone();
        self.imp().prev_bar_button.connect_clicked(move |_| {
            page.navigate_bar(-1);
        });

        let page = self.clone();
        self.imp().next_bar_button.connect_clicked(move |_| {
            page.navigate_bar(1);
        });

        let page = self.clone();
        self.imp().tone_button.connect_clicked(move |_| {
            page.cycle_tone();
        });

        let page = self.clone();
        self.imp().metronome_button.connect_toggled(move |button| {
            page.imp().metronome_enabled.set(button.is_active());
            page.imp().settings.set_boolean("tab-metronome-enabled", button.is_active()).ok();
            page.rebuild_timeline();
        });

        let page = self.clone();
        self.imp()
            .tempo_scale
            .connect_value_changed(move |scale| {
                page.imp().settings.set_uint("tab-tempo-percent", scale.value() as u32).ok();
                page.rebuild_timeline();
            });

        let page = self.clone();
        self.imp()
            .guitar_volume_scale
            .connect_value_changed(move |scale| {
                page.imp().settings.set_uint("tab-guitar-volume", scale.value() as u32).ok();
                if let Some(ref audio_thread) = *page.imp().audio_thread.borrow() {
                    audio_thread.send(TabAudioCommand::SetGuitarVolume(scale.value() as u8));
                }
            });

        let page = self.clone();
        self.imp()
            .metronome_volume_scale
            .connect_value_changed(move |scale| {
                page.imp().settings.set_uint("tab-metronome-volume", scale.value() as u32).ok();
                if let Some(ref audio_thread) = *page.imp().audio_thread.borrow() {
                    audio_thread.send(TabAudioCommand::SetMetronomeVolume(
                        scale.value() as u8,
                    ));
                }
            });

        // Tab strip: seek on click/drag
        let page = self.clone();
        self.imp()
            .tab_strip
            .connect_closure(
                "beat-seeked",
                false,
                glib::closure_local!(move |_strip: &MtrTabStrip, beat_index: u32| {
                    page.seek_to_beat(beat_index as usize);
                }),
            );

        // Tab strip: drag-select loop range
        let page = self.clone();
        self.imp()
            .tab_strip
            .connect_closure(
                "loop-range-changed",
                false,
                glib::closure_local!(move |_strip: &MtrTabStrip, start: u32, end: u32| {
                    page.set_loop_range(start as usize, end as usize);
                }),
            );

        // Tab strip: click to clear loop
        let page = self.clone();
        self.imp()
            .tab_strip
            .connect_closure(
                "loop-cleared",
                false,
                glib::closure_local!(move |_strip: &MtrTabStrip| {
                    page.clear_loop();
                }),
            );

        // Dismiss error button
        let page = self.clone();
        self.imp().dismiss_error_button.connect_clicked(move |_| {
            page.imp().status_label.set_label("Open a GuitarPro file to begin");
            page.imp().dismiss_error_button.set_visible(false);
        });

        // Count-in button
        let page = self.clone();
        self.imp().count_in_button.connect_clicked(move |_| {
            page.cycle_count_in();
        });

        // Loop toggle button
        let page = self.clone();
        self.imp().loop_button.connect_toggled(move |button| {
            if !button.is_active() {
                page.clear_loop();
            }
        });

        // Vertical scroll wheel → horizontal scroll on tab strip
        Self::add_vertical_to_horizontal_scroll(&self.imp().tab_strip_scroll);
        Self::add_vertical_to_horizontal_scroll(&self.imp().tab_fretboard_scroll);

        // Track dropdown: switch track on selection change
        let page = self.clone();
        self.imp()
            .track_dropdown
            .connect_notify_local(Some("selected"), move |dropdown, _| {
                let track_index = dropdown.selected() as usize;
                page.switch_track(track_index);
            });
    }

    fn add_vertical_to_horizontal_scroll(scrolled_window: &gtk::ScrolledWindow) {
        let scroll_ctrl = gtk::EventControllerScroll::new(
            gtk::EventControllerScrollFlags::VERTICAL,
        );
        let sw = scrolled_window.downgrade();
        scroll_ctrl.connect_scroll(move |_, _dx, dy| {
            if let Some(scrolled) = sw.upgrade() {
                let adj = scrolled.hadjustment();
                let step = 150.0;
                adj.set_value(adj.value() + dy * step);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        scrolled_window.add_controller(scroll_ctrl);
    }

    fn load_settings(&self) {
        let settings = &self.imp().settings;
        self.imp().tempo_scale.set_value(
            settings.uint("tab-tempo-percent") as f64,
        );
        self.imp().guitar_volume_scale.set_value(
            settings.uint("tab-guitar-volume") as f64,
        );
        self.imp().metronome_volume_scale.set_value(
            settings.uint("tab-metronome-volume") as f64,
        );
        let tone = settings.uint("tab-guitar-tone") as usize;
        if tone < GUITAR_TONES.len() {
            self.imp().tone_index.set(tone);
            self.imp().tone_button.set_label(GUITAR_TONES[tone].1);
        }
        let met = settings.boolean("tab-metronome-enabled");
        self.imp().metronome_enabled.set(met);
        self.imp().metronome_button.set_active(met);
    }

    fn rebuild_open_menu(&self) {
        let recent = crate::recent_files::load();

        if recent.is_empty() {
            // No recent files — clicking the button opens the file dialog directly
            self.imp().open_button.set_popover(gtk::Widget::NONE);
            let page = self.clone();
            // MenuButton with no popover/menu acts as a regular button via "activate"
            self.imp().open_button.connect_activate(move |_| {
                page.open_file_dialog();
            });
            return;
        }

        let list_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        // "Browse files..." item
        let browse = gtk::Button::with_label("Browse files...");
        browse.add_css_class("flat");
        let page = self.clone();
        browse.connect_clicked(move |_| {
            page.imp().open_button.popdown();
            page.open_file_dialog();
        });
        list_box.append(&browse);

        list_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // Recent file items
        for entry in &recent {
            let btn = gtk::Button::with_label(&entry.title);
            btn.add_css_class("flat");
            btn.set_halign(gtk::Align::Start);
            let page = self.clone();
            let file_path = entry.path.clone();
            btn.connect_clicked(move |_| {
                page.imp().open_button.popdown();
                page.load_file(std::path::Path::new(&file_path));
            });
            list_box.append(&btn);
        }

        let popover = gtk::Popover::new();
        popover.set_child(Some(&list_box));
        self.imp().open_button.set_popover(Some(&popover));
    }

    fn open_file_dialog(&self) {
        let filter = gtk::FileFilter::new();
        filter.add_pattern("*.gp5");
        filter.add_pattern("*.gp");
        filter.add_pattern("*.gp4");
        filter.add_pattern("*.gp3");
        filter.set_name(Some("GuitarPro Files"));

        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let dialog = gtk::FileDialog::builder()
            .title("Open GuitarPro File")
            .filters(&filters)
            .build();

        let page = self.clone();
        dialog.open(
            self.root()
                .and_then(|root| root.downcast::<gtk::Window>().ok())
                .as_ref(),
            None::<&gio::Cancellable>,
            move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        page.load_file(&path);
                    }
                }
            },
        );
    }

    pub fn load_file(&self, path: &std::path::Path) {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let parse_result: Result<(TabScore, usize), String> = match extension.as_str() {
            "gp" => gp7_parser::parse_file(path).map_err(|error| error.to_string()),
            _ => gp5_parser::parse_file(path).map_err(|error| error.to_string()),
        };

        match parse_result {
            Ok((score, default_track)) => {
                // Prefer metadata title, fall back to filename
                let title = if !score.title.is_empty() {
                    if !score.artist.is_empty() {
                        format!("{} — {}", score.title, score.artist)
                    } else {
                        score.title.clone()
                    }
                } else {
                    path.file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Untitled".into())
                };

                self.imp().song_title_label.set_label(&title);
                self.imp().selected_track.set(default_track);
                self.imp().file_path.replace(Some(path.to_path_buf()));

                // Update time signature display
                if let Some(first_bar) = score.bars.first() {
                    self.imp().time_sig_label.set_label(&format!(
                        "{}/{}",
                        first_bar.time_sig_num, first_bar.time_sig_denom
                    ));
                }

                // Populate track dropdown
                if score.tracks.len() > 1 {
                    let model = gtk::StringList::new(
                        &score
                            .tracks
                            .iter()
                            .map(|track| track.name.as_str())
                            .collect::<Vec<_>>(),
                    );
                    self.imp().track_dropdown.set_model(Some(&model));
                    self.imp()
                        .track_dropdown
                        .set_selected(default_track as u32);
                    self.imp().track_dropdown.set_visible(true);
                } else {
                    self.imp().track_dropdown.set_visible(false);
                }

                // Show tab strip and fretboard, hide status
                self.imp().tab_strip_scroll.set_visible(true);
                self.imp().tab_fretboard_scroll.set_visible(true);
                self.imp().status_box.set_visible(false);
                self.imp().dismiss_error_button.set_visible(false);
                self.imp().tab_strip.set_score(&score);

                // Set accessible labels for E2E test discovery
                let strip: &gtk::Accessible = self.imp().tab_strip.upcast_ref();
                strip.update_property(&[gtk::accessible::Property::Label("Tab Strip")]);
                let fretboard: &gtk::Accessible = self.imp().tab_fretboard.upcast_ref();
                fretboard.update_property(&[gtk::accessible::Property::Label("Tab Fretboard")]);

                self.imp().score.replace(Some(score));
                self.ensure_audio_thread();
                self.rebuild_timeline();

                log::info!("Loaded tab: {} (track {})", title, default_track);

                // Save to recent files and rebuild the open menu
                crate::recent_files::add(path, &title);
                self.rebuild_open_menu();
            }
            Err(error_message) => {
                self.imp()
                    .status_label
                    .set_label(&format!("Error: {}", error_message));
                self.imp().dismiss_error_button.set_visible(true);
                self.imp().status_box.set_visible(true);
                self.imp().tab_strip_scroll.set_visible(false);
                self.imp().tab_fretboard_scroll.set_visible(false);
                self.imp().score.replace(None);
                log::error!("Failed to load file: {}", error_message);
            }
        }
    }

    fn ensure_audio_thread(&self) {
        if self.imp().audio_thread.borrow().is_some() {
            return;
        }

        let sonivox_path = PathBuf::from(PKGDATADIR).join("soundfonts/sonivox.sf2");
        let metronome_path =
            PathBuf::from(PKGDATADIR).join("soundfonts/metronome_clicks.sf2");

        let page_weak = glib::SendWeakRef::from(self.downgrade());
        let beat_callback = BeatCallback::new(move |beat_index, notes| {
            let weak = page_weak.clone();
            let notes_owned: Vec<(u8, u8)> = notes.to_vec();
            glib::MainContext::default().invoke(move || {
                if let Some(page) = weak.upgrade() {
                    page.on_beat(beat_index, &notes_owned);
                } else {
                    log::warn!("[main] weak upgrade failed for beat {}", beat_index);
                }
            });
        });

        let audio_thread =
            TabAudioThread::new(sonivox_path, metronome_path, beat_callback);

        self.imp().audio_thread.replace(audio_thread);
    }

    fn update_effective_bpm(&self) {
        let score = self.imp().score.borrow();
        if let Some(score) = score.as_ref() {
            if let Some(bar) = score.bars.first() {
                let tempo_percent = self.imp().tempo_scale.value();
                let effective = (bar.tempo * tempo_percent / 100.0).round() as u32;
                self.imp()
                    .effective_bpm_label
                    .set_label(&format!("{}bpm", effective));
            }
        }
    }

    fn rebuild_timeline(&self) {
        let score = self.imp().score.borrow();
        let score = match score.as_ref() {
            Some(score) => score,
            None => return,
        };

        let tempo_percent = self.imp().tempo_scale.value();
        let metronome_enabled = self.imp().metronome_enabled.get();
        let track_index = self.imp().selected_track.get();

        let timeline = tab_midi::build_timeline(
            score,
            track_index,
            tempo_percent,
            metronome_enabled,
        );

        if let Some(ref audio_thread) = *self.imp().audio_thread.borrow() {
            audio_thread.send(TabAudioCommand::SetTimeline(timeline));
        }

        self.update_effective_bpm();
    }

    fn toggle_play_pause(&self) {
        if self.imp().score.borrow().is_none() {
            return;
        }

        let is_playing = self.imp().is_playing.get();
        if is_playing {
            if let Some(ref audio_thread) = *self.imp().audio_thread.borrow() {
                audio_thread.send(TabAudioCommand::Pause);
            }
            self.imp().is_playing.set(false);
            self.imp()
                .play_pause_button
                .set_icon_name("media-playback-start-symbolic");
        } else {
            if let Some(ref audio_thread) = *self.imp().audio_thread.borrow() {
                let count_in = self.imp().count_in_bars.get();
                if count_in > 0 {
                    let score = self.imp().score.borrow();
                    let score = match score.as_ref() {
                        Some(score) => score,
                        None => return,
                    };
                    let first_bar = score.bars.first();
                    let bpm = first_bar.map(|bar| bar.tempo).unwrap_or(120.0);
                    let beats = first_bar.map(|bar| bar.time_sig_num).unwrap_or(4);
                    let tempo_percent = self.imp().tempo_scale.value();
                    audio_thread.send(TabAudioCommand::PlayWithCountIn {
                        bars: count_in,
                        bpm: bpm * tempo_percent / 100.0,
                        beats_per_bar: beats,
                    });
                } else {
                    audio_thread.send(TabAudioCommand::Play);
                }
            }
            self.imp().is_playing.set(true);
            self.imp()
                .play_pause_button
                .set_icon_name("media-playback-pause-symbolic");
        }
    }

    fn stop(&self) {
        if let Some(ref audio_thread) = *self.imp().audio_thread.borrow() {
            audio_thread.send(TabAudioCommand::Pause);
            audio_thread.send(TabAudioCommand::SeekToBeat(0));
        }
        self.imp().is_playing.set(false);
        self.imp()
            .play_pause_button
            .set_icon_name("media-playback-start-symbolic");
        self.imp().current_time_label.set_label("0:00");
        self.imp().tab_strip.set_current_beat(-1);
        self.imp().tab_fretboard.clear_notes();
    }

    fn seek_to_beat(&self, beat_index: usize) {
        if let Some(ref audio_thread) = *self.imp().audio_thread.borrow() {
            audio_thread.send(TabAudioCommand::SeekToBeat(beat_index));
        }
    }

    fn switch_track(&self, track_index: usize) {
        let current = self.imp().selected_track.get();
        if track_index == current {
            return;
        }

        let score = self.imp().score.borrow();
        let score = match score.as_ref() {
            Some(score) => score,
            None => return,
        };

        if track_index >= score.tracks.len() {
            return;
        }

        let _ = score; // release borrow

        // Re-parse the file for the new track
        let file_path = self.imp().file_path.borrow().clone();
        if let Some(ref path) = file_path {
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_lowercase();

            let parse_result: Result<(TabScore, usize), String> = match extension.as_str() {
                "gp" => gp7_parser::parse_file_for_track(path, track_index)
                    .map_err(|error| error.to_string()),
                _ => gp5_parser::parse_file_for_track(path, track_index)
                    .map(|score| (score, track_index))
                    .map_err(|error| error.to_string()),
            };

            if let Ok((score, _)) = parse_result {
                // Rebuild score for the new track — the parser extracts all tracks
                // but build_tab_score uses the default. For GP7 we need to re-extract
                // beats for the new track. Since our parsers already return all tracks'
                // beats for the default track, we re-parse entirely.
                self.imp().selected_track.set(track_index);
                self.imp().current_beat.set(0);
                self.imp().tab_strip.set_current_beat(-1);
                self.imp().tab_fretboard.clear_notes();
                self.imp().tab_strip.set_score(&score);
                self.imp().score.replace(Some(score));
                self.rebuild_timeline();
            }
        }
    }

    fn navigate_bar(&self, direction: i32) {
        let score = self.imp().score.borrow();
        let score = match score.as_ref() {
            Some(score) => score,
            None => return,
        };

        let current = self.imp().current_beat.get();
        let current_bar_index = score
            .beats
            .get(current)
            .map(|beat| beat.bar_index)
            .unwrap_or(0);

        let target_bar = (current_bar_index as i32 + direction)
            .clamp(0, score.bars.len() as i32 - 1) as usize;

        let target_beat = score.bars[target_bar].first_beat_index;
        self.seek_to_beat(target_beat);
    }

    fn cycle_count_in(&self) {
        let current = self.imp().count_in_bars.get();
        let next = match current {
            0 => 1,
            1 => 2,
            2 => 4,
            _ => 0,
        };
        self.imp().count_in_bars.set(next);
        self.imp()
            .count_in_button
            .set_label(&format!("{}in", next));
    }

    pub fn set_loop_on_bar(&self, beat_index: usize) {
        let score = self.imp().score.borrow();
        let score = match score.as_ref() {
            Some(score) => score,
            None => return,
        };

        let bar = match score.beats.get(beat_index) {
            Some(beat) => &score.bars[beat.bar_index],
            None => return,
        };

        let loop_start = bar.first_beat_index;
        let loop_end = bar.first_beat_index + bar.beat_count.saturating_sub(1);
        self.set_loop_range(loop_start, loop_end);
    }

    fn set_loop_range(&self, start_beat: usize, end_beat: usize) {
        self.imp().tab_strip.set_loop_range(start_beat as i32, end_beat as i32);
        self.imp().loop_button.set_active(true);

        if let Some(ref audio_thread) = *self.imp().audio_thread.borrow() {
            audio_thread.send(TabAudioCommand::SetLoop(Some((start_beat, end_beat))));
        }
    }

    pub fn clear_loop(&self) {
        self.imp().tab_strip.set_loop_range(-1, -1);
        self.imp().loop_button.set_active(false);

        if let Some(ref audio_thread) = *self.imp().audio_thread.borrow() {
            audio_thread.send(TabAudioCommand::SetLoop(None));
        }
    }

    fn cycle_tone(&self) {
        let current = self.imp().tone_index.get();
        let next = (current + 1) % GUITAR_TONES.len();
        self.imp().tone_index.set(next);
        self.imp().settings.set_uint("tab-guitar-tone", next as u32).ok();

        let (program, label) = GUITAR_TONES[next];
        self.imp().tone_button.set_label(label);

        if let Some(ref audio_thread) = *self.imp().audio_thread.borrow() {
            audio_thread.send(TabAudioCommand::SetGuitarProgram(program));
        }
    }

    fn on_beat(&self, beat_index: usize, _notes: &[(u8, u8)]) {
        let score = self.imp().score.borrow();
        let score = match score.as_ref() {
            Some(score) => score,
            None => {
                log::warn!("[main] on_beat({}): score is None", beat_index);
                return;
            }
        };
        log::debug!("[main] on_beat({}) score.beats.len()={}", beat_index, score.beats.len());

        self.imp().current_beat.set(beat_index);

        // Update tab strip cursor
        self.imp().tab_strip.set_current_beat(beat_index as i32);

        // Update fretboard with notes from this beat
        if let Some(beat) = score.beats.get(beat_index) {
            let note_pairs: Vec<(u8, u8)> = beat
                .notes
                .iter()
                .map(|note| (note.string, note.fret))
                .collect();
            if !note_pairs.is_empty() {
                self.imp().tab_fretboard.set_active_notes(&note_pairs);
            }
        }

        // Update time display
        if let Some(beat) = score.beats.get(beat_index) {
            let tempo_percent = self.imp().tempo_scale.value();
            let bar = &score.bars[beat.bar_index];
            let effective_bpm = bar.tempo * tempo_percent / 100.0;
            let seconds = beat.tick / (TICKS_PER_QUARTER_FOR_TIME * effective_bpm / 60.0);
            let minutes = (seconds / 60.0) as u32;
            let secs = (seconds % 60.0) as u32;
            self.imp()
                .current_time_label
                .set_label(&format!("{}:{:02}", minutes, secs));

            // Update time sig
            self.imp().time_sig_label.set_label(&format!(
                "{}/{}",
                bar.time_sig_num, bar.time_sig_denom
            ));
        }

        // Check if playback reached end
        if beat_index + 1 >= score.beats.len()
            && !self.imp().loop_button.is_active()
        {
            self.imp().is_playing.set(false);
            self.imp()
                .play_pause_button
                .set_icon_name("media-playback-start-symbolic");
        }
    }
}

const TICKS_PER_QUARTER_FOR_TIME: f64 = 960.0;
