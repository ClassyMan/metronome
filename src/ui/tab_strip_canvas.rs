/// Canvas-rendered tab strip — shows tablature notation with beat cursor and loop range.

use super::fluent;
use crate::tab_models::TabScore;
use iced::mouse;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size};

const BEAT_WIDTH: f32 = 48.0;
const LEFT_MARGIN: f32 = 40.0;
const TOP_MARGIN: f32 = 16.0;
const STRING_SPACING: f32 = 18.0;
const STRING_COUNT: usize = 6;
const STRING_LABELS: [&str; 6] = ["e", "B", "G", "D", "A", "E"];

pub struct TabStripCanvas {
    cache: canvas::Cache,
    beats: Vec<StripBeat>,
    bar_boundaries: Vec<usize>,
    current_beat: i32,
    loop_start: i32,
    loop_end: i32,
}

struct StripBeat {
    notes: Vec<(u8, u8)>,
    is_rest: bool,
}

impl TabStripCanvas {
    pub fn new() -> Self {
        Self {
            cache: canvas::Cache::new(),
            beats: Vec::new(),
            bar_boundaries: Vec::new(),
            current_beat: -1,
            loop_start: -1,
            loop_end: -1,
        }
    }

    pub fn set_score(&mut self, score: &TabScore) {
        self.beats = score
            .beats
            .iter()
            .map(|beat| StripBeat {
                notes: beat.notes.iter().map(|n| (n.string, n.fret)).collect(),
                is_rest: beat.is_rest,
            })
            .collect();
        self.bar_boundaries = score.bars.iter().map(|bar| bar.first_beat_index).collect();
        self.current_beat = -1;
        self.cache.clear();
    }

    pub fn set_current_beat(&mut self, beat: i32) {
        if self.current_beat != beat {
            self.current_beat = beat;
            self.cache.clear();
        }
    }

    pub fn set_loop_range(&mut self, start: i32, end: i32) {
        self.loop_start = start;
        self.loop_end = end;
        self.cache.clear();
    }

    pub fn total_width(&self) -> f32 {
        LEFT_MARGIN + self.beats.len() as f32 * BEAT_WIDTH + LEFT_MARGIN
    }

    pub fn content_height(&self) -> f32 {
        TOP_MARGIN + STRING_SPACING * (STRING_COUNT - 1) as f32 + 16.0
    }
}

impl<Message> canvas::Program<Message, fluent::Theme> for TabStripCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &fluent::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            let fg = Color::from_rgba(1.0, 1.0, 1.0, 0.8);
            let dim = Color::from_rgba(1.0, 1.0, 1.0, 0.15);
            let accent = Color::from_rgba(0.35, 0.65, 0.95, 1.0);
            let fretboard_height = STRING_SPACING * (STRING_COUNT - 1) as f32;

            // String lines
            for string_index in 0..STRING_COUNT {
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                frame.fill_rectangle(
                    Point::new(LEFT_MARGIN, y - 0.5),
                    Size::new(bounds.width - LEFT_MARGIN, 1.0),
                    dim,
                );
            }

            // String labels
            for (string_index, label) in STRING_LABELS.iter().enumerate() {
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                frame.fill_text(canvas::Text {
                    content: label.to_string(),
                    position: Point::new(12.0, y - 6.0),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.4),
                    size: 10.0.into(),
                    ..canvas::Text::default()
                });
            }

            // Loop range shading
            if self.loop_start >= 0 && self.loop_end >= 0 {
                let start_x = LEFT_MARGIN + self.loop_start as f32 * BEAT_WIDTH;
                let end_x = LEFT_MARGIN + (self.loop_end + 1) as f32 * BEAT_WIDTH;
                frame.fill_rectangle(
                    Point::new(start_x, TOP_MARGIN - 4.0),
                    Size::new(end_x - start_x, fretboard_height + 8.0),
                    Color::from_rgba(accent.r, accent.g, accent.b, 0.1),
                );
            }

            // Bar lines
            for &beat_index in &self.bar_boundaries {
                let x = LEFT_MARGIN + beat_index as f32 * BEAT_WIDTH;
                frame.fill_rectangle(
                    Point::new(x - 0.5, TOP_MARGIN - 4.0),
                    Size::new(1.0, fretboard_height + 8.0),
                    Color::from_rgba(1.0, 1.0, 1.0, 0.25),
                );
            }

            // Beat data (fret numbers)
            for (beat_offset, strip_beat) in self.beats.iter().enumerate() {
                let x = LEFT_MARGIN + beat_offset as f32 * BEAT_WIDTH + BEAT_WIDTH / 2.0;
                let is_current = beat_offset as i32 == self.current_beat;

                if strip_beat.is_rest {
                    let rest_color = if is_current {
                        Color::from_rgba(accent.r, accent.g, accent.b, 0.8)
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.3)
                    };
                    for string_index in 0..STRING_COUNT {
                        let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                        frame.fill_text(canvas::Text {
                            content: "-".to_string(),
                            position: Point::new(x - 3.0, y - 6.0),
                            color: rest_color,
                            size: 10.0.into(),
                            ..canvas::Text::default()
                        });
                    }
                    continue;
                }

                for &(string, fret) in &strip_beat.notes {
                    let string_index = (string as usize).saturating_sub(1);
                    if string_index >= STRING_COUNT {
                        continue;
                    }
                    let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                    let fret_text = format!("{}", fret);

                    if is_current {
                        // Highlighted background
                        let bg_size = 14.0;
                        frame.fill(
                            &canvas::Path::rectangle(
                                Point::new(x - bg_size / 2.0, y - bg_size / 2.0),
                                Size::new(bg_size, bg_size),
                            ),
                            Color::from_rgba(accent.r, accent.g, accent.b, 0.8),
                        );
                        frame.fill_text(canvas::Text {
                            content: fret_text,
                            position: Point::new(x - 4.0, y - 5.0),
                            color: Color::WHITE,
                            size: 10.0.into(),
                            ..canvas::Text::default()
                        });
                    } else {
                        frame.fill_text(canvas::Text {
                            content: fret_text,
                            position: Point::new(x - 4.0, y - 5.0),
                            color: fg,
                            size: 10.0.into(),
                            ..canvas::Text::default()
                        });
                    }
                }
            }

            // Cursor line
            if self.current_beat >= 0 {
                let cursor_x =
                    LEFT_MARGIN + self.current_beat as f32 * BEAT_WIDTH + BEAT_WIDTH / 2.0;
                frame.fill_rectangle(
                    Point::new(cursor_x - 0.5, TOP_MARGIN - 4.0),
                    Size::new(1.0, fretboard_height + 8.0),
                    Color::from_rgba(accent.r, accent.g, accent.b, 0.4),
                );
            }
        });

        vec![geometry]
    }
}
