use crate::chord_builder;
use crate::scale_data::{self, ALL_FAMILIES, NOTE_NAMES};
use iced::mouse;
use iced::widget::{button, canvas, column, container, pick_list, row, text, Space};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

const FRET_WIDTH: f32 = 48.0;
const NUT_WIDTH: f32 = 6.0;
const LEFT_MARGIN: f32 = 40.0;
const TOP_MARGIN: f32 = 24.0;
const STRING_SPACING: f32 = 26.0;
const NOTE_RADIUS: f32 = 10.0;
const NUM_STRINGS: usize = 6;
const NUM_FRETS: usize = 24;
const STRING_LABELS: [&str; 6] = ["e", "B", "G", "D", "A", "E"];
const FRET_MARKERS: [usize; 10] = [3, 5, 7, 9, 12, 15, 17, 19, 21, 24];
const DOUBLE_MARKERS: [usize; 2] = [12, 24];

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
    fretboard_cache: canvas::Cache,
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
            fretboard_cache: canvas::Cache::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::SetRoot(root) => {
                self.root = root.min(11);
                self.fretboard_cache.clear();
            }
            Message::SetFamily(family) => {
                self.family = family.min(ALL_FAMILIES.len() - 1);
                self.mode = 0;
                self.chord_structure = 0;
                self.fretboard_cache.clear();
            }
            Message::SetMode(mode) => {
                let family = &ALL_FAMILIES[self.family];
                self.mode = mode.min(family.scales.len().saturating_sub(1));
                self.chord_structure = 0;
                self.fretboard_cache.clear();
            }
            Message::SetChordStructure(structure) => {
                self.chord_structure = structure;
                self.inversion = 0;
                self.fretboard_cache.clear();
            }
            Message::CycleInversion => {
                self.inversion = (self.inversion + 1) % 3;
                self.fretboard_cache.clear();
            }
            Message::TogglePentatonic => {
                let scale = &ALL_FAMILIES[self.family].scales[self.mode];
                if !scale.pentatonic_variants.is_empty() {
                    self.pentatonic_variant =
                        (self.pentatonic_variant + 1) % (scale.pentatonic_variants.len() + 1);
                    self.fretboard_cache.clear();
                }
            }
        }
        iced::Task::none()
    }

    fn current_scale(&self) -> &'static scale_data::Scale {
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
            move |selected| {
                let family = &ALL_FAMILIES[self.family];
                let index = family
                    .scales
                    .iter()
                    .position(|s| s.name == selected)
                    .unwrap_or(0);
                Message::SetMode(index)
            },
        )
        .width(150);

        let scale_label = text(format!(
            "{} {}",
            NOTE_NAMES[self.root],
            scale.name
        ))
        .size(20);

        let chord_structures = ["None", "Triad", "7th", "9th", "11th", "13th",
            "add9", "sus2", "sus4", "6th", "6/9"];
        let chord_pick = pick_list(
            chord_structures.map(String::from).to_vec(),
            Some(chord_structures[self.chord_structure.min(chord_structures.len() - 1)].to_string()),
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

        let fretboard = canvas(&self.fretboard_cache)
            .width(Length::Fill)
            .height(TOP_MARGIN + STRING_SPACING * (NUM_STRINGS - 1) as f32 + 32.0);

        let content = column![controls, fretboard].spacing(0);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl canvas::Program<Message> for canvas::Cache {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        // For now, draw the empty fretboard structure.
        // Scale notes will be drawn once we wire up the scale state.
        let geometry = self.draw(renderer, bounds.size(), |frame| {
            let fg = Color::from_rgba(1.0, 1.0, 1.0, 0.8);
            let dim = Color::from_rgba(1.0, 1.0, 1.0, 0.15);
            let fretboard_height = STRING_SPACING * (NUM_STRINGS - 1) as f32;

            // Fret marker dots
            let dot_color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);
            for &fret in FRET_MARKERS.iter() {
                let x = fret_center_x(fret);
                if DOUBLE_MARKERS.contains(&fret) {
                    draw_circle(frame, dot_color, x, TOP_MARGIN + STRING_SPACING * 1.5, 4.0);
                    draw_circle(frame, dot_color, x, TOP_MARGIN + STRING_SPACING * 3.5, 4.0);
                } else {
                    draw_circle(
                        frame,
                        dot_color,
                        x,
                        TOP_MARGIN + fretboard_height / 2.0,
                        4.0,
                    );
                }
            }

            // Strings
            for string_index in 0..NUM_STRINGS {
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                let thickness = 1.0 + (NUM_STRINGS - 1 - string_index) as f32 * 0.4;
                frame.fill_rectangle(
                    Point::new(LEFT_MARGIN, y - thickness / 2.0),
                    Size::new(bounds.width - LEFT_MARGIN, thickness),
                    dim,
                );
            }

            // Nut
            frame.fill_rectangle(
                Point::new(LEFT_MARGIN, TOP_MARGIN),
                Size::new(NUT_WIDTH, fretboard_height),
                Color::from_rgba(1.0, 1.0, 1.0, 0.5),
            );

            // Fret lines
            for fret in 1..=NUM_FRETS {
                let x = LEFT_MARGIN + NUT_WIDTH + fret as f32 * FRET_WIDTH;
                frame.fill_rectangle(
                    Point::new(x, TOP_MARGIN),
                    Size::new(1.0, fretboard_height),
                    Color::from_rgba(1.0, 1.0, 1.0, 0.12),
                );
            }

            // String labels
            for (string_index, label) in STRING_LABELS.iter().enumerate() {
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                let label_text = canvas::Text {
                    content: label.to_string(),
                    position: Point::new(12.0, y - 6.0),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.4),
                    size: 12.0.into(),
                    ..canvas::Text::default()
                };
                frame.fill_text(label_text);
            }

            // Fret numbers at markers
            for &fret in FRET_MARKERS.iter() {
                let x = fret_center_x(fret);
                let label_text = canvas::Text {
                    content: format!("{}", fret),
                    position: Point::new(x - 6.0, TOP_MARGIN - 16.0),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.35),
                    size: 10.0.into(),
                    ..canvas::Text::default()
                };
                frame.fill_text(label_text);
            }
        });

        vec![geometry]
    }
}

fn fret_center_x(fret: usize) -> f32 {
    if fret == 0 {
        LEFT_MARGIN - 10.0
    } else {
        LEFT_MARGIN + NUT_WIDTH + (fret as f32 - 0.5) * FRET_WIDTH
    }
}

fn draw_circle(frame: &mut canvas::Frame, color: Color, cx: f32, cy: f32, radius: f32) {
    frame.fill(
        &canvas::Path::circle(Point::new(cx, cy), radius),
        color,
    );
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
        page.update(Message::SetRoot(7)); // G
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
        page.update(Message::SetFamily(1)); // Melodic Minor
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
        // Major Ionian has pentatonic variants
        let has_penta = !page.current_scale().pentatonic_variants.is_empty();
        assert!(has_penta);
        page.update(Message::TogglePentatonic);
        assert_eq!(page.pentatonic_variant, 1);
        page.update(Message::TogglePentatonic);
        // Should cycle back to 0 after all variants
    }
}
