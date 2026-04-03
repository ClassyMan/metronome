/// Canvas-rendered tab fretboard — shows currently-playing notes with fade animation.

use iced::mouse;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};
use std::time::Instant;

const FRET_WIDTH: f32 = 52.0;
const NUT_WIDTH: f32 = 6.0;
const LEFT_MARGIN: f32 = 12.0;
pub const TOP_MARGIN: f32 = 20.0;
const STRING_SPACING: f32 = 28.0;
const NOTE_RADIUS: f32 = 11.0;
const NUM_STRINGS: usize = 6;
const NUM_FRETS: usize = 24;
const FRET_MARKERS: [usize; 10] = [3, 5, 7, 9, 12, 15, 17, 19, 21, 24];
const DOUBLE_MARKERS: [usize; 2] = [12, 24];
const FADE_DURATION_MS: u128 = 1500;
const STRIKE_DURATION_MS: u128 = 120;

struct NoteGlow {
    string: u8,
    fret: u8,
    start_time: Instant,
}

pub struct TabFretboardCanvas {
    cache: canvas::Cache,
    active_glows: Vec<NoteGlow>,
}

impl TabFretboardCanvas {
    pub fn new() -> Self {
        Self {
            cache: canvas::Cache::new(),
            active_glows: Vec::new(),
        }
    }

    pub fn set_active_notes(&mut self, notes: &[(u8, u8)]) {
        let now = Instant::now();

        // Prune expired glows
        self.active_glows
            .retain(|glow| now.duration_since(glow.start_time).as_millis() < FADE_DURATION_MS);

        for &(string, fret) in notes {
            self.active_glows
                .retain(|glow| glow.string != string || glow.fret != fret);
            self.active_glows.push(NoteGlow {
                string,
                fret,
                start_time: now,
            });
        }
        self.cache.clear();
    }

    pub fn clear_notes(&mut self) {
        self.active_glows.clear();
        self.cache.clear();
    }

    pub fn has_active_animations(&self) -> bool {
        let now = Instant::now();
        self.active_glows
            .iter()
            .any(|glow| now.duration_since(glow.start_time).as_millis() < FADE_DURATION_MS)
    }

    pub fn tick(&mut self) {
        if self.has_active_animations() {
            self.cache.clear();
        }
    }

    pub fn content_height() -> f32 {
        TOP_MARGIN + STRING_SPACING * (NUM_STRINGS - 1) as f32 + 16.0
    }
}

impl<Message> canvas::Program<Message> for TabFretboardCanvas {
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
            let accent = Color::from_rgba(0.35, 0.65, 0.95, 1.0);
            let fretboard_height = STRING_SPACING * (NUM_STRINGS - 1) as f32;
            let now = Instant::now();

            // Fret marker dots
            let dot_color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);
            for &fret in FRET_MARKERS.iter() {
                let x = fret_center_x(fret);
                if DOUBLE_MARKERS.contains(&fret) {
                    draw_circle(frame, dot_color, x, TOP_MARGIN + STRING_SPACING * 1.5, 5.0);
                    draw_circle(frame, dot_color, x, TOP_MARGIN + STRING_SPACING * 3.5, 5.0);
                } else {
                    draw_circle(frame, dot_color, x, TOP_MARGIN + fretboard_height / 2.0, 5.0);
                }
            }

            // Strings
            for string_index in 0..NUM_STRINGS {
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                let thickness = 1.0 + (NUM_STRINGS - 1 - string_index) as f32 * 0.5;
                frame.fill_rectangle(
                    Point::new(LEFT_MARGIN, y - thickness / 2.0),
                    Size::new(bounds.width - LEFT_MARGIN, thickness),
                    Color::from_rgba(1.0, 1.0, 1.0, 0.2),
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
                    dim,
                );
            }

            // Active note glows
            for glow in &self.active_glows {
                let elapsed_ms = now.duration_since(glow.start_time).as_millis();
                if elapsed_ms >= FADE_DURATION_MS {
                    continue;
                }

                let string_index = (glow.string as usize).saturating_sub(1);
                if string_index >= NUM_STRINGS {
                    continue;
                }

                let x = fret_center_x(glow.fret as usize);
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                let intensity = 1.0 - (elapsed_ms as f32 / FADE_DURATION_MS as f32);

                // Strike flash (first 120ms)
                if elapsed_ms < STRIKE_DURATION_MS {
                    let strike_alpha =
                        0.9 * (1.0 - elapsed_ms as f32 / STRIKE_DURATION_MS as f32);
                    draw_circle(
                        frame,
                        Color::from_rgba(1.0, 1.0, 1.0, strike_alpha),
                        x,
                        y,
                        NOTE_RADIUS * 1.8,
                    );
                }

                // 3-layer glow
                draw_circle(
                    frame,
                    Color::from_rgba(accent.r, accent.g, accent.b, intensity * 0.25),
                    x,
                    y,
                    NOTE_RADIUS * 1.6,
                );
                draw_circle(
                    frame,
                    Color::from_rgba(accent.r, accent.g, accent.b, intensity * 0.5),
                    x,
                    y,
                    NOTE_RADIUS * 1.2,
                );
                draw_circle(
                    frame,
                    Color::from_rgba(accent.r, accent.g, accent.b, intensity),
                    x,
                    y,
                    NOTE_RADIUS,
                );

                // Fret number
                frame.fill_text(canvas::Text {
                    content: format!("{}", glow.fret),
                    position: Point::new(x - 4.0, y - 5.0),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.9 * intensity),
                    size: 9.0.into(),
                    ..canvas::Text::default()
                });
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
