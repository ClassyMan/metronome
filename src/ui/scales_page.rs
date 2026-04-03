use super::fretboard_canvas::{FretboardCanvas, TOP_MARGIN};
use crate::scale_data::{ALL_FAMILIES, NOTE_NAMES};
use iced::widget::{button, canvas, column, container, pick_list, row, text, Space};
use iced::{Element, Length};

const STRING_SPACING: f32 = 26.0;
const NUM_STRINGS: usize = 6;

#[derive(Debug, Clone)]
pub enum Message {
    SetRoot(usize),
    SetFamily(usize),
    SetMode(usize),
    SetChordStructure(usize),
    CycleInversion,
    TogglePentatonic,
}

pub struct ScalesPage {
    root: usize,
    family: usize,
    mode: usize,
    chord_structure: usize,
    inversion: usize,
    pentatonic_variant: usize,
    fretboard: FretboardCanvas,
}

impl ScalesPage {
    pub fn new() -> Self {
        Self {
            root: 0,
            family: 0,
            mode: 0,
            chord_structure: 0,
            inversion: 0,
            pentatonic_variant: 0,
            fretboard: FretboardCanvas::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::SetRoot(root) => {
                self.root = root.min(11);
            }
            Message::SetFamily(family) => {
                self.family = family.min(ALL_FAMILIES.len() - 1);
                self.mode = 0;
                self.chord_structure = 0;
            }
            Message::SetMode(mode) => {
                let family = &ALL_FAMILIES[self.family];
                self.mode = mode.min(family.scales.len().saturating_sub(1));
                self.chord_structure = 0;
            }
            Message::SetChordStructure(structure) => {
                self.chord_structure = structure;
                self.inversion = 0;
            }
            Message::CycleInversion => {
                self.inversion = (self.inversion + 1) % 3;
            }
            Message::TogglePentatonic => {
                let scale = self.current_scale();
                if !scale.pentatonic_variants.is_empty() {
                    self.pentatonic_variant =
                        (self.pentatonic_variant + 1) % (scale.pentatonic_variants.len() + 1);
                }
            }
        }
        self.fretboard.set_scale(
            self.root,
            self.family,
            self.mode,
            self.pentatonic_variant,
        );
        iced::Task::none()
    }

    pub fn restore(&mut self, settings: &super::settings::Settings) {
        self.root = settings.scale_root;
        self.family = settings.scale_family;
        self.mode = settings.scale_mode;
        self.fretboard.set_scale(self.root, self.family, self.mode, 0);
    }

    pub fn save(&self, settings: &mut super::settings::Settings) {
        settings.scale_root = self.root;
        settings.scale_family = self.family;
        settings.scale_mode = self.mode;
    }

    fn current_scale(&self) -> &'static crate::scale_data::Scale {
        &ALL_FAMILIES[self.family].scales[self.mode]
    }

    pub fn view(&self) -> Element<Message> {
        let scale = self.current_scale();

        let root_names: Vec<String> = NOTE_NAMES.iter().map(|s| s.to_string()).collect();
        let family_names: Vec<String> = ALL_FAMILIES.iter().map(|f| f.name.to_string()).collect();
        let mode_names: Vec<String> = ALL_FAMILIES[self.family]
            .scales
            .iter()
            .map(|s| s.name.to_string())
            .collect();

        let root_pick = pick_list(
            root_names.clone(),
            Some(root_names[self.root].clone()),
            move |selected| {
                let index = NOTE_NAMES.iter().position(|&n| n == selected).unwrap_or(0);
                Message::SetRoot(index)
            },
        )
        .width(80);

        let family_pick = pick_list(
            family_names.clone(),
            Some(family_names[self.family].clone()),
            move |selected| {
                let index = ALL_FAMILIES
                    .iter()
                    .position(|f| f.name == selected)
                    .unwrap_or(0);
                Message::SetFamily(index)
            },
        )
        .width(150);

        let mode_pick = pick_list(
            mode_names.clone(),
            Some(mode_names[self.mode].clone()),
            {
                let family = self.family;
                move |selected| {
                    let fam = &ALL_FAMILIES[family];
                    let index = fam
                        .scales
                        .iter()
                        .position(|s| s.name == selected)
                        .unwrap_or(0);
                    Message::SetMode(index)
                }
            },
        )
        .width(150);

        let scale_label = text(format!("{} {}", NOTE_NAMES[self.root], scale.name)).size(20);

        let chord_structures = [
            "None", "Triad", "7th", "9th", "11th", "13th", "add9", "sus2", "sus4", "6th", "6/9",
        ];
        let chord_pick = pick_list(
            chord_structures.map(String::from).to_vec(),
            Some(
                chord_structures[self.chord_structure.min(chord_structures.len() - 1)].to_string(),
            ),
            move |selected| {
                let index = chord_structures
                    .iter()
                    .position(|&s| s == selected)
                    .unwrap_or(0);
                Message::SetChordStructure(index)
            },
        )
        .width(100);

        let inversion_label = match self.inversion {
            0 => "Root",
            1 => "1st inv",
            _ => "2nd inv",
        };
        let inversion_btn = button(text(inversion_label).size(12))
            .on_press(Message::CycleInversion)
            .padding([4, 8]);

        let has_pentatonic = !scale.pentatonic_variants.is_empty();
        let penta_label = if self.pentatonic_variant == 0 {
            "Penta: off"
        } else {
            "Penta: on"
        };

        let controls = column![
            row![root_pick, family_pick, mode_pick].spacing(8),
            Space::new().height(8),
            scale_label,
            Space::new().height(8),
            row![
                chord_pick,
                inversion_btn,
                if has_pentatonic {
                    Element::from(
                        button(text(penta_label).size(12))
                            .on_press(Message::TogglePentatonic)
                            .padding([4, 8]),
                    )
                } else {
                    Element::from(Space::new())
                },
            ]
            .spacing(8),
        ]
        .padding(12);

        let fretboard = canvas(&self.fretboard)
            .width(Length::Fill)
            .height(TOP_MARGIN + STRING_SPACING * (NUM_STRINGS - 1) as f32 + 32.0);

        let content = column![controls, fretboard].spacing(0);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_page() -> ScalesPage {
        ScalesPage::new()
    }

    #[test]
    fn test_initial_state() {
        let page = make_page();
        assert_eq!(page.root, 0);
        assert_eq!(page.family, 0);
        assert_eq!(page.mode, 0);
        assert_eq!(page.chord_structure, 0);
    }

    #[test]
    fn test_set_root() {
        let mut page = make_page();
        page.update(Message::SetRoot(7));
        assert_eq!(page.root, 7);
    }

    #[test]
    fn test_set_root_clamped() {
        let mut page = make_page();
        page.update(Message::SetRoot(99));
        assert_eq!(page.root, 11);
    }

    #[test]
    fn test_set_family_resets_mode() {
        let mut page = make_page();
        page.update(Message::SetMode(3));
        assert_eq!(page.mode, 3);
        page.update(Message::SetFamily(1));
        assert_eq!(page.mode, 0);
        assert_eq!(page.family, 1);
    }

    #[test]
    fn test_set_mode_clamped() {
        let mut page = make_page();
        page.update(Message::SetMode(999));
        let max_mode = ALL_FAMILIES[0].scales.len() - 1;
        assert_eq!(page.mode, max_mode);
    }

    #[test]
    fn test_chord_structure_resets_inversion() {
        let mut page = make_page();
        page.inversion = 2;
        page.update(Message::SetChordStructure(1));
        assert_eq!(page.chord_structure, 1);
        assert_eq!(page.inversion, 0);
    }

    #[test]
    fn test_cycle_inversion() {
        let mut page = make_page();
        page.update(Message::CycleInversion);
        assert_eq!(page.inversion, 1);
        page.update(Message::CycleInversion);
        assert_eq!(page.inversion, 2);
        page.update(Message::CycleInversion);
        assert_eq!(page.inversion, 0);
    }

    #[test]
    fn test_current_scale() {
        let page = make_page();
        let scale = page.current_scale();
        assert_eq!(scale.name, "Ionian");
    }

    #[test]
    fn test_pentatonic_toggle() {
        let mut page = make_page();
        assert!(!page.current_scale().pentatonic_variants.is_empty());
        page.update(Message::TogglePentatonic);
        assert_eq!(page.pentatonic_variant, 1);
    }

    // ── Boundary and interaction tests ─────────────────────────────

    #[test]
    fn test_pentatonic_no_variants_is_noop() {
        let mut page = make_page();
        // Find a family with scales that have no pentatonic variants
        // Messiaen scales typically have none
        page.update(Message::SetFamily(3)); // Messiaen
        let scale = page.current_scale();
        assert!(scale.pentatonic_variants.is_empty());
        page.update(Message::TogglePentatonic);
        assert_eq!(page.pentatonic_variant, 0);
    }

    #[test]
    fn test_pentatonic_cycles_back_to_zero() {
        let mut page = make_page();
        let variant_count = page.current_scale().pentatonic_variants.len();
        for _ in 0..=variant_count {
            page.update(Message::TogglePentatonic);
        }
        assert_eq!(page.pentatonic_variant, 0);
    }

    #[test]
    fn test_set_mode_after_family_change_clamps() {
        let mut page = make_page();
        page.update(Message::SetFamily(3)); // Messiaen — may have fewer scales
        let max_mode = ALL_FAMILIES[3].scales.len() - 1;
        page.update(Message::SetMode(999));
        assert_eq!(page.mode, max_mode);
    }

    #[test]
    fn test_chord_structure_out_of_bounds_stores_value() {
        let mut page = make_page();
        page.update(Message::SetChordStructure(999));
        // Value stored as-is (view clamps for display)
        assert_eq!(page.chord_structure, 999);
    }

    #[test]
    fn test_family_change_resets_chord_structure() {
        let mut page = make_page();
        page.update(Message::SetChordStructure(5));
        assert_eq!(page.chord_structure, 5);
        page.update(Message::SetFamily(1));
        assert_eq!(page.chord_structure, 0);
    }

    #[test]
    fn test_restore_then_family_change_resets_mode() {
        let mut page = make_page();
        let mut settings = super::super::settings::Settings::default();
        settings.scale_root = 5;
        settings.scale_family = 0;
        settings.scale_mode = 3;
        page.restore(&settings);
        assert_eq!(page.mode, 3);

        page.update(Message::SetFamily(1));
        assert_eq!(page.mode, 0);
        assert_eq!(page.root, 5); // root should survive
    }

    #[test]
    fn test_save_captures_scale_state() {
        let mut page = make_page();
        page.update(Message::SetRoot(7));
        page.update(Message::SetFamily(2));
        let mut settings = super::super::settings::Settings::default();
        page.save(&mut settings);
        assert_eq!(settings.scale_root, 7);
        assert_eq!(settings.scale_family, 2);
        assert_eq!(settings.scale_mode, 0);
    }
}
