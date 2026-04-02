/// Tab player fretboard widget — shows active notes with glow/fade animation.
///
/// Unlike the scales fretboard (MtrFretboard), this widget has no scale/mode
/// context. It displays currently playing notes with a 3-layer glow effect
/// that fades over 1500ms, plus a 120ms strike flash on note onset.

use adw::subclass::prelude::*;
use gtk::{gdk, glib, graphene, prelude::*};
use std::cell::RefCell;
use std::time::Instant;

const FRET_WIDTH: f32 = 52.0;
const NUT_WIDTH: f32 = 6.0;
const LEFT_MARGIN: f32 = 12.0;
const TOP_MARGIN: f32 = 20.0;
const BOTTOM_MARGIN: f32 = 16.0;
const NUM_STRINGS: usize = 6;
const NUM_FRETS: usize = 24;
const STRING_SPACING: f32 = 28.0;
const NOTE_RADIUS: f32 = 11.0;

const FADE_DURATION_MS: u128 = 1500;
const STRIKE_DURATION_MS: u128 = 120;

const FRET_MARKERS: [usize; 10] = [3, 5, 7, 9, 12, 15, 17, 19, 21, 24];
const DOUBLE_MARKERS: [usize; 2] = [12, 24];

#[derive(Debug, Clone)]
struct NoteGlow {
    string: u8,
    fret: u8,
    start_time: Instant,
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct MtrTabFretboard {
        pub active_glows: RefCell<Vec<NoteGlow>>,
        pub tick_callback_id: RefCell<Option<gtk::TickCallbackId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTabFretboard {
        const NAME: &'static str = "MtrTabFretboard";
        type Type = super::MtrTabFretboard;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("tab-fretboard");
        }
    }

    impl ObjectImpl for MtrTabFretboard {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn dispose(&self) {
            // Remove tick callback
            if let Some(callback_id) = self.tick_callback_id.borrow_mut().take() {
                callback_id.remove();
            }
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for MtrTabFretboard {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => {
                    let natural =
                        (LEFT_MARGIN + NUT_WIDTH + FRET_WIDTH * NUM_FRETS as f32) as i32;
                    (200, natural, -1, -1)
                }
                _ => {
                    let natural = (TOP_MARGIN
                        + STRING_SPACING * (NUM_STRINGS - 1) as f32
                        + BOTTOM_MARGIN) as i32;
                    (natural, natural, -1, -1)
                }
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;
            if width <= 0.0 || height <= 0.0 {
                return;
            }

            let fg = widget.color();
            let accent = lookup_accent_color(&*widget);
            let fretboard_height = STRING_SPACING * (NUM_STRINGS - 1) as f32;
            let now = Instant::now();

            // Draw fret marker dots
            let dot_color = with_alpha(&fg, 0.06);
            for &fret in FRET_MARKERS.iter() {
                let x = fret_center_x(fret);
                if DOUBLE_MARKERS.contains(&fret) {
                    append_circle(snapshot, &dot_color, x, TOP_MARGIN + STRING_SPACING * 1.5, 5.0);
                    append_circle(snapshot, &dot_color, x, TOP_MARGIN + STRING_SPACING * 3.5, 5.0);
                } else {
                    append_circle(
                        snapshot,
                        &dot_color,
                        x,
                        TOP_MARGIN + fretboard_height / 2.0,
                        5.0,
                    );
                }
            }

            // Draw strings
            for string_index in 0..NUM_STRINGS {
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                let thickness = 1.0 + (NUM_STRINGS - 1 - string_index) as f32 * 0.5;
                let color = with_alpha(&fg, 0.2);
                snapshot.append_color(
                    &color,
                    &graphene::Rect::new(LEFT_MARGIN, y - thickness / 2.0, width, thickness),
                );
            }

            // Draw nut
            let nut_color = with_alpha(&fg, 0.5);
            snapshot.append_color(
                &nut_color,
                &graphene::Rect::new(LEFT_MARGIN, TOP_MARGIN, NUT_WIDTH, fretboard_height),
            );

            // Draw fret lines
            for fret in 1..=NUM_FRETS {
                let x = LEFT_MARGIN + NUT_WIDTH + fret as f32 * FRET_WIDTH;
                let color = with_alpha(&fg, 0.12);
                snapshot.append_color(
                    &color,
                    &graphene::Rect::new(x, TOP_MARGIN, 1.0, fretboard_height),
                );
            }

            // Draw fret numbers
            for &fret in &[1usize] {
                draw_fret_number(&*widget, snapshot, &fg, fret);
            }
            for &fret in FRET_MARKERS.iter() {
                draw_fret_number(&*widget, snapshot, &fg, fret);
            }

            // Draw active note glows
            let glows = self.active_glows.borrow();
            for glow in glows.iter() {
                let elapsed_ms = now.duration_since(glow.start_time).as_millis();
                let string_index = (glow.string as usize).saturating_sub(1);
                if string_index >= NUM_STRINGS {
                    continue;
                }

                let x = fret_center_x(glow.fret as usize);
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                let fret_text = format!("{}", glow.fret);

                if elapsed_ms > FADE_DURATION_MS {
                    // Ghost note — faded circle remains at position
                    let ghost_color = with_alpha(&fg, 0.15);
                    append_circle(snapshot, &ghost_color, x, y, NOTE_RADIUS * 0.85);
                    let ghost_text = with_alpha(&fg, 0.3);
                    draw_text(&*widget, snapshot, &ghost_text, x, y, &fret_text, 8.0);
                    continue;
                }

                let intensity = 1.0 - (elapsed_ms as f32 / FADE_DURATION_MS as f32);

                // Strike flash with hitmarker (first 120ms)
                if elapsed_ms < STRIKE_DURATION_MS {
                    let strike_alpha = 0.9 * (1.0 - elapsed_ms as f32 / STRIKE_DURATION_MS as f32);
                    let strike_color = gdk::RGBA::new(1.0, 1.0, 1.0, strike_alpha);
                    append_circle(snapshot, &strike_color, x, y, NOTE_RADIUS * 1.8);

                    // Diagonal hitmarker lines
                    let outer_dist = NOTE_RADIUS * 1.6;
                    let inner_dist = NOTE_RADIUS * 1.0;
                    let line_color = gdk::RGBA::new(1.0, 1.0, 1.0, 0.85 * strike_alpha);
                    let line_width = 2.5;
                    for &(dx, dy_sign) in &[(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                        let x1 = x + dx * outer_dist;
                        let y1 = y + dy_sign * outer_dist;
                        let x2 = x + dx * inner_dist;
                        let y2 = y + dy_sign * inner_dist;
                        let rect = graphene::Rect::new(
                            x1.min(x2) - line_width / 2.0,
                            y1.min(y2) - line_width / 2.0,
                            (x1 - x2).abs() + line_width,
                            (y1 - y2).abs() + line_width,
                        );
                        snapshot.append_color(&line_color, &rect);
                    }
                }

                // 3-layer glow
                let outer = with_alpha(&accent, intensity * 0.25);
                append_circle(snapshot, &outer, x, y, NOTE_RADIUS * 1.6);

                let mid = with_alpha(&accent, intensity * 0.5);
                append_circle(snapshot, &mid, x, y, NOTE_RADIUS * 1.2);

                let core = with_alpha(&accent, intensity);
                append_circle(snapshot, &core, x, y, NOTE_RADIUS);

                // Fret number label
                let label_color = contrast_color(&accent, intensity);
                draw_text(&*widget, snapshot, &label_color, x, y, &fret_text, 9.0);
            }
        }
    }
}

glib::wrapper! {
    pub struct MtrTabFretboard(ObjectSubclass<imp::MtrTabFretboard>)
        @extends gtk::Widget,
        @implements gtk::Accessible;
}

impl MtrTabFretboard {
    /// Set active notes and start the fade animation.
    /// `notes` are (string, fret) pairs where string is 1-indexed (1=high E).
    pub fn set_active_notes(&self, notes: &[(u8, u8)]) {
        let now = Instant::now();
        let mut glows = self.imp().active_glows.borrow_mut();

        for &(string, fret) in notes {
            // Cancel any existing glow for this position
            glows.retain(|glow| glow.string != string || glow.fret != fret);

            glows.push(NoteGlow {
                string,
                fret,
                start_time: now,
            });
        }

        self.ensure_tick_callback();
        self.auto_scroll_to_notes(notes);
        self.queue_draw();
    }

    pub fn clear_notes(&self) {
        self.imp().active_glows.borrow_mut().clear();
        self.queue_draw();
    }

    fn auto_scroll_to_notes(&self, notes: &[(u8, u8)]) {
        if notes.is_empty() {
            return;
        }
        let min_fret = notes.iter().map(|(_, fret)| *fret).min().unwrap_or(0);
        if min_fret == 0 {
            return;
        }
        // Scroll so the lowest active fret is visible with some margin
        let target_x =
            (fret_center_x(min_fret.saturating_sub(2) as usize) - LEFT_MARGIN) as f64;

        let mut parent = self.parent();
        while let Some(ref widget) = parent {
            if let Some(scrolled) = widget.downcast_ref::<gtk::ScrolledWindow>() {
                let adjustment = scrolled.hadjustment();
                let current = adjustment.value();
                let viewport = scrolled.width() as f64;
                let note_x = fret_center_x(min_fret as usize) as f64;

                if note_x < current || note_x > current + viewport * 0.8 {
                    adjustment.set_value(target_x.max(0.0));
                }
                break;
            }
            parent = widget.parent();
        }
    }

    fn ensure_tick_callback(&self) {
        if self.imp().tick_callback_id.borrow().is_some() {
            return;
        }

        let callback_id = self.add_tick_callback(|widget, _clock| {
            let tab_fb = widget.downcast_ref::<MtrTabFretboard>().unwrap();
            let now = Instant::now();

            // Check if any glows are still animating (not yet ghost)
            let has_animating = tab_fb
                .imp()
                .active_glows
                .borrow()
                .iter()
                .any(|glow| now.duration_since(glow.start_time).as_millis() < FADE_DURATION_MS);

            tab_fb.queue_draw();

            if !has_animating {
                // All glows are ghosts now — stop the tick callback
                // (ghosts are static, no animation needed)
                tab_fb.imp().tick_callback_id.borrow_mut().take();
                return glib::ControlFlow::Break;
            }

            glib::ControlFlow::Continue
        });

        self.imp().tick_callback_id.replace(Some(callback_id));
    }
}

fn fret_center_x(fret: usize) -> f32 {
    if fret == 0 {
        LEFT_MARGIN - 10.0
    } else {
        LEFT_MARGIN + NUT_WIDTH + (fret as f32 - 0.5) * FRET_WIDTH
    }
}

fn with_alpha(color: &gdk::RGBA, alpha: f32) -> gdk::RGBA {
    gdk::RGBA::new(color.red(), color.green(), color.blue(), alpha)
}

fn contrast_color(bg: &gdk::RGBA, intensity: f32) -> gdk::RGBA {
    let luminance = 0.299 * bg.red() + 0.587 * bg.green() + 0.114 * bg.blue();
    if luminance > 0.5 {
        gdk::RGBA::new(0.0, 0.0, 0.0, 0.9 * intensity)
    } else {
        gdk::RGBA::new(1.0, 1.0, 1.0, 0.9 * intensity)
    }
}

fn lookup_accent_color(widget: &MtrTabFretboard) -> gdk::RGBA {
    #[allow(deprecated)]
    if let Some(color) = widget.style_context().lookup_color("accent_bg_color") {
        return color;
    }
    gdk::RGBA::new(0.21, 0.52, 0.89, 1.0)
}

fn append_circle(snapshot: &gtk::Snapshot, color: &gdk::RGBA, cx: f32, cy: f32, radius: f32) {
    let rect = graphene::Rect::new(cx - radius, cy - radius, radius * 2.0, radius * 2.0);
    let rounded = gtk::gsk::RoundedRect::from_rect(rect, radius);
    snapshot.push_rounded_clip(&rounded);
    snapshot.append_color(color, &rect);
    snapshot.pop();
}

fn draw_fret_number(
    widget: &MtrTabFretboard,
    snapshot: &gtk::Snapshot,
    fg: &gdk::RGBA,
    fret: usize,
) {
    let x = fret_center_x(fret);
    let label = format!("{}", fret);
    let color = with_alpha(fg, 0.35);
    draw_text(widget, snapshot, &color, x, TOP_MARGIN - 12.0, &label, 9.0);
}

fn draw_text(
    widget: &MtrTabFretboard,
    snapshot: &gtk::Snapshot,
    color: &gdk::RGBA,
    cx: f32,
    cy: f32,
    text: &str,
    font_size: f32,
) {
    let pango_context = widget.pango_context();
    let mut font_desc = pango_context.font_description().unwrap_or_default();
    font_desc.set_size((font_size * gtk::pango::SCALE as f32) as i32);

    let layout = gtk::pango::Layout::new(&pango_context);
    layout.set_font_description(Some(&font_desc));
    layout.set_text(text);

    let (ink_rect, _) = layout.pixel_extents();
    let tx = cx - ink_rect.width() as f32 / 2.0 - ink_rect.x() as f32;
    let ty = cy - ink_rect.height() as f32 / 2.0 - ink_rect.y() as f32;

    snapshot.save();
    snapshot.translate(&graphene::Point::new(tx, ty));
    snapshot.append_layout(&layout, color);
    snapshot.restore();
}
