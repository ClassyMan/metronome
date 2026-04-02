/// Canvas-rendered guitar fretboard for the scales page.
/// Draws strings, frets, and scale/chord note markers.

use crate::scale_data::{self, Scale};
use iced::mouse;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

const FRET_WIDTH: f32 = 48.0;
const NUT_WIDTH: f32 = 6.0;
const LEFT_MARGIN: f32 = 40.0;
pub const TOP_MARGIN: f32 = 24.0;
const STRING_SPACING: f32 = 26.0;
const NOTE_RADIUS: f32 = 10.0;
const NUM_STRINGS: usize = 6;
const NUM_FRETS: usize = 24;
const STRING_LABELS: [&str; 6] = ["e", "B", "G", "D", "A", "E"];
const FRET_MARKERS: [usize; 10] = [3, 5, 7, 9, 12, 15, 17, 19, 21, 24];
const DOUBLE_MARKERS: [usize; 2] = [12, 24];

pub struct FretboardCanvas {
    cache: canvas::Cache,
    root: usize,
    family: usize,
    mode: usize,
    pentatonic_variant: usize,
}

impl FretboardCanvas {
    pub fn new() -> Self {
        Self {
            cache: canvas::Cache::new(),
            root: 0,
            family: 0,
            mode: 0,
            pentatonic_variant: 0,
        }
    }

    pub fn set_scale(&mut self, root: usize, family: usize, mode: usize, pentatonic_variant: usize) {
        self.root = root;
        self.family = family;
        self.mode = mode;
        self.pentatonic_variant = pentatonic_variant;
        self.cache.clear();
    }

    fn current_scale(&self) -> &'static Scale {
        &scale_data::ALL_FAMILIES[self.family].scales[self.mode]
    }
}

impl<Message> canvas::Program<Message> for FretboardCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let dim = Color::from_rgba(1.0, 1.0, 1.0, 0.15);
            let fretboard_height = STRING_SPACING * (NUM_STRINGS - 1) as f32;
            let accent = Color::from_rgba(0.35, 0.65, 0.95, 1.0);

            // Fret marker dots
            let dot_color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);
            for &fret in FRET_MARKERS.iter() {
                let x = fret_center_x(fret);
                if DOUBLE_MARKERS.contains(&fret) {
                    draw_circle(frame, dot_color, x, TOP_MARGIN + STRING_SPACING * 1.5, 4.0);
                    draw_circle(frame, dot_color, x, TOP_MARGIN + STRING_SPACING * 3.5, 4.0);
                } else {
                    draw_circle(frame, dot_color, x, TOP_MARGIN + fretboard_height / 2.0, 4.0);
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
                frame.fill_text(canvas::Text {
                    content: label.to_string(),
                    position: Point::new(12.0, y - 6.0),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.4),
                    size: 12.0.into(),
                    ..canvas::Text::default()
                });
            }

            // Fret numbers at markers
            for &fret in FRET_MARKERS.iter() {
                let x = fret_center_x(fret);
                frame.fill_text(canvas::Text {
                    content: format!("{}", fret),
                    position: Point::new(x - 6.0, TOP_MARGIN - 16.0),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.35),
                    size: 10.0.into(),
                    ..canvas::Text::default()
                });
            }

            // Scale notes
            let scale = self.current_scale();
            let root = self.root as u8;
            let pentatonic_indices: Option<&[usize]> = if self.pentatonic_variant > 0 {
                scale
                    .pentatonic_variants
                    .get(self.pentatonic_variant - 1)
                    .copied()
            } else {
                None
            };

            for string_index in 0..NUM_STRINGS {
                // scale_data uses low-to-high (0=low E), canvas uses high-to-low (0=high e)
                let data_string = NUM_STRINGS - 1 - string_index;
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;

                for fret in 0..=NUM_FRETS {
                    let note = scale_data::note_at_fret(data_string, fret);
                    if let Some(degree) = scale_data::scale_degree(note, root, scale) {
                        let is_root = degree == 0;

                        // If pentatonic mode, dim non-pentatonic notes
                        let in_pentatonic = pentatonic_indices
                            .map(|indices| indices.contains(&degree))
                            .unwrap_or(true);

                        let alpha = if !in_pentatonic { 0.2 } else { 0.85 };
                        let color = if is_root {
                            Color::from_rgba(accent.r, accent.g, accent.b, alpha)
                        } else {
                            Color::from_rgba(1.0, 1.0, 1.0, alpha * 0.7)
                        };

                        let x = fret_center_x(fret);
                        let radius = if is_root {
                            NOTE_RADIUS
                        } else {
                            NOTE_RADIUS * 0.85
                        };
                        draw_circle(frame, color, x, y, radius);

                        // Degree label
                        let label = scale.degree_labels[degree];
                        let text_color = if is_root {
                            Color::from_rgba(0.0, 0.0, 0.0, alpha)
                        } else {
                            Color::from_rgba(1.0, 1.0, 1.0, alpha)
                        };
                        frame.fill_text(canvas::Text {
                            content: label.to_string(),
                            position: Point::new(x - 4.0, y - 6.0),
                            color: text_color,
                            size: 10.0.into(),
                            ..canvas::Text::default()
                        });
                    }
                }
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
    frame.fill(&canvas::Path::circle(Point::new(cx, cy), radius), color);
}
