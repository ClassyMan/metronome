use crate::chord_builder::{self, CHORD_STRUCTURES};
use crate::fretboard::MtrFretboard;
use crate::guitar_player;
use crate::scale_data::{self, ALL_FAMILIES, NOTE_NAMES};
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/com/adrienplazas/Metronome/ui/scales-page.ui")]
    pub struct MtrScalesPage {
        #[template_child]
        pub fretboard: TemplateChild<MtrFretboard>,
        #[template_child]
        pub root_dropdown: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub family_dropdown: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub mode_dropdown: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub chord_dropdown: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub chord_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub scale_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub degree_labels_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub pentatonic_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub inversion_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub mute_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub tap_hint_label: TemplateChild<gtk::Label>,
        pub inversion: Cell<usize>,
        pub chord_degree: Cell<i32>,
        pub chord_fret: Cell<i32>,
        pub pentatonic_variant: Cell<usize>,
    }

    impl Default for MtrScalesPage {
        fn default() -> Self {
            Self {
                fretboard: Default::default(),
                root_dropdown: Default::default(),
                family_dropdown: Default::default(),
                mode_dropdown: Default::default(),
                chord_dropdown: Default::default(),
                chord_label: Default::default(),
                scale_name_label: Default::default(),
                degree_labels_label: Default::default(),
                pentatonic_button: Default::default(),
                inversion_button: Default::default(),
                mute_button: Default::default(),
                tap_hint_label: Default::default(),
                inversion: Cell::new(0),
                chord_degree: Cell::new(-1),
                chord_fret: Cell::new(-1),
                pentatonic_variant: Cell::new(0),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrScalesPage {
        const NAME: &'static str = "MtrScalesPage";
        type Type = super::MtrScalesPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            MtrFretboard::ensure_type();
            klass.bind_template();
            klass.set_layout_manager_type::<gtk::BinLayout>();
            klass.set_css_name("scales-page");

        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MtrScalesPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_dropdowns();
            obj.connect_signals();
            obj.update_controls_visibility();
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for MtrScalesPage {}
}

glib::wrapper! {
    pub struct MtrScalesPage(ObjectSubclass<imp::MtrScalesPage>)
        @extends gtk::Widget;
}

impl MtrScalesPage {
    pub fn set_chord_structure(&self, index: u32) {
        self.imp().chord_dropdown.set_selected(index);
    }

    pub fn tap_fret(&self, string_idx: usize, fret: usize) {
        self.on_fret_tapped(string_idx, fret);
    }

    fn setup_dropdowns(&self) {
        let imp = self.imp();

        // Root note dropdown (C, C#, D, ... B)
        let root_model = gtk::StringList::new(&NOTE_NAMES);
        imp.root_dropdown.set_model(Some(&root_model));
        imp.root_dropdown.set_selected(0);

        // Family dropdown
        let family_names: Vec<&str> = ALL_FAMILIES.iter().map(|f| f.name).collect();
        let family_model = gtk::StringList::new(&family_names);
        imp.family_dropdown.set_model(Some(&family_model));
        imp.family_dropdown.set_selected(0);

        // Mode dropdown (populated from selected family)
        self.update_mode_dropdown();

        // Chord structure dropdown (None + 10 structures)
        let mut chord_names: Vec<&str> = vec!["None"];
        chord_names.extend(CHORD_STRUCTURES.iter().map(|s| s.label));
        let chord_model = gtk::StringList::new(&chord_names);
        imp.chord_dropdown.set_model(Some(&chord_model));
        imp.chord_dropdown.set_selected(0);
    }

    fn connect_signals(&self) {
        let imp = self.imp();

        let page = self.downgrade();
        imp.root_dropdown.connect_selected_notify(move |_| {
            if let Some(page) = page.upgrade() {
                page.on_selection_changed();
            }
        });

        let page = self.downgrade();
        imp.family_dropdown.connect_selected_notify(move |_| {
            if let Some(page) = page.upgrade() {
                page.update_mode_dropdown();
                page.on_selection_changed();
            }
        });

        let page = self.downgrade();
        imp.mode_dropdown.connect_selected_notify(move |_| {
            if let Some(page) = page.upgrade() {
                page.on_selection_changed();
            }
        });

        let page = self.downgrade();
        imp.chord_dropdown.connect_selected_notify(move |_| {
            if let Some(page) = page.upgrade() {
                // Reset chord degree when structure changes
                page.imp().chord_degree.set(-1);
                page.imp().chord_fret.set(-1);
                page.imp().inversion.set(0);
                page.imp().inversion_button.set_label("Root");
                page.on_selection_changed();
            }
        });

        // Pentatonic toggle
        let page = self.downgrade();
        imp.pentatonic_button.connect_clicked(move |_| {
            if let Some(page) = page.upgrade() {
                page.cycle_pentatonic();
            }
        });

        // Inversion cycling
        let page = self.downgrade();
        imp.inversion_button.connect_clicked(move |_| {
            if let Some(page) = page.upgrade() {
                page.cycle_inversion();
            }
        });

        // Mute toggle — update label and replay chord when unmuting
        let page = self.downgrade();
        imp.mute_button.connect_toggled(move |btn| {
            let label = if btn.is_active() { "\u{1F50A}" } else { "\u{1F507}" };
            btn.set_label(label);
            if let Some(page) = page.upgrade() {
                if btn.is_active() {
                    page.update_voicing();
                }
            }
        });

        // Fretboard tap handler
        let page = self.downgrade();
        imp.fretboard.connect_closure(
            "fret-tapped",
            false,
            glib::closure_local!(move |_fb: MtrFretboard, string_idx: u32, fret: u32| {
                if let Some(page) = page.upgrade() {
                    page.on_fret_tapped(string_idx as usize, fret as usize);
                }
            }),
        );
    }

    fn update_mode_dropdown(&self) {
        let imp = self.imp();
        let family_idx = imp.family_dropdown.selected() as usize;
        let family = &ALL_FAMILIES[family_idx.min(ALL_FAMILIES.len() - 1)];

        let mode_names: Vec<&str> = family.scales.iter().map(|s| s.name).collect();
        let mode_model = gtk::StringList::new(&mode_names);
        imp.mode_dropdown.set_model(Some(&mode_model));
        imp.mode_dropdown.set_selected(0);
    }

    fn on_selection_changed(&self) {
        let imp = self.imp();
        let root = imp.root_dropdown.selected() as u32;
        let family_idx = imp.family_dropdown.selected() as u32;
        let mode_idx = imp.mode_dropdown.selected() as u32;

        imp.fretboard.set_root(root);
        imp.fretboard.set_family_index(family_idx);
        imp.fretboard.set_mode_index(mode_idx);

        // Clamp pentatonic variant if the new scale has fewer variants
        let family = &ALL_FAMILIES[family_idx as usize];
        let scale = &family.scales[mode_idx as usize];
        let num_variants = scale.pentatonic_variants.len();
        let current_pent = imp.pentatonic_variant.get();
        if current_pent > num_variants {
            imp.pentatonic_variant.set(0);
        }
        imp.fretboard.set_pentatonic_variant(imp.pentatonic_variant.get() as u32);

        // Update scale name and degree labels
        let root_name = NOTE_NAMES[root as usize];
        let scale_name = format!("{} {}", root_name, scale.name);
        imp.scale_name_label.set_label(&scale_name);

        let degrees: String = scale
            .degree_labels
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .join("  ");
        imp.degree_labels_label.set_label(&degrees);

        self.update_controls_visibility();
        self.update_voicing();
    }

    fn cycle_pentatonic(&self) {
        let imp = self.imp();
        let family_idx = imp.family_dropdown.selected() as usize;
        let mode_idx = imp.mode_dropdown.selected() as usize;
        let family = &ALL_FAMILIES[family_idx.min(ALL_FAMILIES.len() - 1)];
        let scale = &family.scales[mode_idx.min(family.scales.len() - 1)];
        let num_variants = scale.pentatonic_variants.len();

        if num_variants == 0 {
            return;
        }

        let next = (imp.pentatonic_variant.get() + 1) % (num_variants + 1);
        imp.pentatonic_variant.set(next);
        imp.fretboard.set_pentatonic_variant(next as u32);

        // Update button label
        let label = if num_variants > 1 && next > 0 {
            format!("Pent {next}")
        } else {
            "Pent".to_string()
        };
        imp.pentatonic_button.set_label(&label);

        // Add/remove suggested-action style for active state
        let style = imp.pentatonic_button.css_classes();
        if next > 0 {
            if !style.iter().any(|c| c == "suggested-action") {
                imp.pentatonic_button.add_css_class("suggested-action");
            }
        } else {
            imp.pentatonic_button.remove_css_class("suggested-action");
        }
    }

    fn cycle_inversion(&self) {
        let imp = self.imp();
        let chord_selected = imp.chord_dropdown.selected() as usize;
        if chord_selected == 0 || imp.chord_degree.get() < 0 {
            return;
        }
        let structure = &CHORD_STRUCTURES[chord_selected - 1];
        let max_inversions = structure.offsets.len();
        let next = (imp.inversion.get() + 1) % max_inversions;
        imp.inversion.set(next);

        let label = if next == 0 {
            "Root".to_string()
        } else {
            format!("Inv {next}")
        };
        imp.inversion_button.set_label(&label);

        self.update_voicing();
    }

    fn update_controls_visibility(&self) {
        let imp = self.imp();
        let chord_selected = imp.chord_dropdown.selected() as usize;
        let degree = imp.chord_degree.get();
        let family_idx = imp.family_dropdown.selected() as usize;
        let mode_idx = imp.mode_dropdown.selected() as usize;
        let family = &ALL_FAMILIES[family_idx.min(ALL_FAMILIES.len() - 1)];
        let scale = &family.scales[mode_idx.min(family.scales.len() - 1)];

        let has_chord = chord_selected > 0 && degree >= 0;
        let has_pentatonics = !scale.pentatonic_variants.is_empty();
        let in_chord_mode = chord_selected > 0;

        // Pentatonic: show when NOT in chord mode and scale has variants
        imp.pentatonic_button.set_visible(!in_chord_mode && has_pentatonics);

        // Inversion: show when chord is active
        imp.inversion_button.set_visible(has_chord);

        // Mute: show when in chord mode
        imp.mute_button.set_visible(in_chord_mode);

        // Tap hint: show when chord structure selected but no note tapped
        imp.tap_hint_label.set_visible(in_chord_mode && degree < 0);
    }

    fn on_fret_tapped(&self, string_idx: usize, fret: usize) {
        let imp = self.imp();
        let chord_selected = imp.chord_dropdown.selected();
        if chord_selected == 0 {
            return; // No chord structure selected
        }

        let root = imp.root_dropdown.selected() as u8;
        let family_idx = imp.family_dropdown.selected() as usize;
        let mode_idx = imp.mode_dropdown.selected() as usize;
        let family = &ALL_FAMILIES[family_idx.min(ALL_FAMILIES.len() - 1)];
        let scale = &family.scales[mode_idx.min(family.scales.len() - 1)];

        if !scale_data::has_diatonic_chords(scale) {
            return;
        }

        // Convert display string_idx to note, find scale degree
        let note = scale_data::note_at_fret(string_idx, fret);
        let degree = match scale_data::scale_degree(note, root, scale) {
            Some(d) => d as i32,
            None => return, // Tapped a non-scale note
        };

        // Toggle: tap same degree+fret = deselect
        if imp.chord_degree.get() == degree && imp.chord_fret.get() == fret as i32 {
            imp.chord_degree.set(-1);
            imp.chord_fret.set(-1);
            imp.inversion.set(0);
        } else {
            imp.chord_degree.set(degree);
            imp.chord_fret.set(fret as i32);
            imp.inversion.set(0);
        }
        imp.inversion_button.set_label("Root");

        self.update_controls_visibility();
        self.update_voicing();
    }

    fn update_voicing(&self) {
        let imp = self.imp();
        let chord_selected = imp.chord_dropdown.selected() as usize;
        let degree = imp.chord_degree.get();

        if chord_selected == 0 || degree < 0 {
            imp.fretboard.set_voicing(String::new());
            imp.fretboard.set_voicing_labels(String::new());
            imp.chord_label.set_label("");
            return;
        }

        let structure = &CHORD_STRUCTURES[chord_selected - 1];
        let root = imp.root_dropdown.selected() as u8;
        let family_idx = imp.family_dropdown.selected() as usize;
        let mode_idx = imp.mode_dropdown.selected() as usize;
        let family = &ALL_FAMILIES[family_idx.min(ALL_FAMILIES.len() - 1)];
        let scale = &family.scales[mode_idx.min(family.scales.len() - 1)];
        let center_fret = imp.chord_fret.get().max(0) as usize;
        let inversion = imp.inversion.get();

        let voicing = chord_builder::generate_voicing(
            root,
            scale,
            degree as usize,
            structure,
            center_fret,
            inversion,
        );

        // Serialize voicing for fretboard: "string,fret;string,fret;..."
        let voicing_str: String = voicing
            .iter()
            .map(|v| format!("{},{}", v.string_index, v.fret))
            .collect::<Vec<_>>()
            .join(";");

        let labels_str: String = voicing
            .iter()
            .map(|v| v.label.as_str())
            .collect::<Vec<_>>()
            .join(",");

        imp.fretboard.set_voicing(voicing_str);
        imp.fretboard.set_voicing_labels(labels_str);

        // Update chord symbol
        let symbol = chord_builder::chord_symbol(root, scale, degree as usize, structure);
        imp.chord_label.set_label(&symbol);

        // Play the chord (if not muted)
        if imp.mute_button.is_active() {
            guitar_player::play_chord(&voicing);
        }
    }
}
