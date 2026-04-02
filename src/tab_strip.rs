/// Scrolling tablature notation widget.
///
/// Renders 6-line guitar tab with fret numbers, bar lines, beat cursor,
/// and loop range visualization. Auto-scrolls to keep the cursor visible.

use crate::tab_models::TabScore;
use adw::subclass::prelude::*;
use gtk::{gdk, glib, graphene, prelude::*};
use std::cell::{Cell, RefCell};

const BEAT_WIDTH: f32 = 48.0;
const LEFT_MARGIN: f32 = 40.0;
const TOP_MARGIN: f32 = 16.0;
const BOTTOM_MARGIN: f32 = 12.0;
const STRING_COUNT: usize = 6;
const STRING_SPACING: f32 = 18.0;
const STRING_LABELS: [&str; 6] = ["e", "B", "G", "D", "A", "E"];

mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct MtrTabStrip {
        pub current_beat: Cell<i32>,
        pub loop_start_beat: Cell<i32>,
        pub loop_end_beat: Cell<i32>,
        pub beats: RefCell<Vec<StripBeat>>,
        pub bar_boundaries: RefCell<Vec<BarBoundary>>,
        pub total_beats: Cell<usize>,
        pub drag_start_beat: Cell<i32>,
        pub drag_selecting: Cell<bool>,
        /// The beat where selection was anchored (for shift+click/arrow extending)
        pub selection_anchor: Cell<i32>,
        /// 0=not dragging handle, 1=dragging start handle, 2=dragging end handle
        pub dragging_handle: Cell<u8>,
    }

    impl Default for MtrTabStrip {
        fn default() -> Self {
            Self {
                current_beat: Cell::new(-1),
                loop_start_beat: Cell::new(-1),
                loop_end_beat: Cell::new(-1),
                beats: RefCell::new(Vec::new()),
                bar_boundaries: RefCell::new(Vec::new()),
                total_beats: Cell::new(0),
                drag_start_beat: Cell::new(-1),
                drag_selecting: Cell::new(false),
                selection_anchor: Cell::new(-1),
                dragging_handle: Cell::new(0),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrTabStrip {
        const NAME: &'static str = "MtrTabStrip";
        type Type = super::MtrTabStrip;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("tab-strip");
        }
    }

    impl ObjectImpl for MtrTabStrip {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_focusable(true);

            // Drag: click-to-seek, drag-to-select, shift+click to extend.
            // Claims on press to prevent GtkWindow's CSD window-move gesture.
            let drag = gtk::GestureDrag::new();
            drag.set_button(1);
            let widget = self.obj().downgrade();
            drag.connect_drag_begin(move |gesture, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                if let Some(strip) = widget.upgrade() {
                    strip.grab_focus();
                    strip.imp().dragging_handle.set(0);

                    // Check if clicking near a loop handle (in the handle zone above stave)
                    let handle_zone_bottom = TOP_MARGIN;
                    if y < handle_zone_bottom as f64 {
                        let hit = strip.hit_test_handle(x);
                        if hit > 0 {
                            strip.imp().dragging_handle.set(hit);
                            strip.imp().drag_selecting.set(true);
                            return;
                        }
                    }

                    let beat = strip.x_to_beat(x);
                    let shift = gesture
                        .current_event()
                        .map(|ev| ev.modifier_state().contains(gdk::ModifierType::SHIFT_MASK))
                        .unwrap_or(false);

                    if shift && strip.imp().selection_anchor.get() >= 0 {
                        let anchor = strip.imp().selection_anchor.get();
                        let lo = anchor.min(beat);
                        let hi = anchor.max(beat);
                        strip.set_loop_range(lo, hi);
                        strip.emit_by_name::<()>("loop-range-changed",
                            &[&(lo as u32), &(hi as u32)]);
                        strip.imp().drag_selecting.set(true);
                    } else {
                        strip.imp().drag_start_beat.set(beat);
                        strip.imp().selection_anchor.set(beat);
                        strip.imp().drag_selecting.set(false);
                    }
                }
            });
            let widget = self.obj().downgrade();
            drag.connect_drag_update(move |gesture, offset_x, _| {
                if let Some(strip) = widget.upgrade() {
                    if let Some((start_x, _)) = gesture.start_point() {
                        let handle = strip.imp().dragging_handle.get();
                        if handle > 0 {
                            // Dragging a handle — move that boundary
                            let new_beat = strip.x_to_beat(start_x + offset_x);
                            let other = if handle == 1 {
                                strip.imp().loop_end_beat.get()
                            } else {
                                strip.imp().loop_start_beat.get()
                            };
                            let lo = new_beat.min(other);
                            let hi = new_beat.max(other);
                            strip.set_loop_range(lo, hi);
                            strip.emit_by_name::<()>("loop-range-changed",
                                &[&(lo as u32), &(hi as u32)]);
                            return;
                        }

                        if offset_x.abs() < 5.0 {
                            return;
                        }
                        strip.imp().drag_selecting.set(true);
                        let anchor = strip.imp().selection_anchor.get();
                        let end_beat = strip.x_to_beat(start_x + offset_x);
                        let lo = anchor.min(end_beat);
                        let hi = anchor.max(end_beat);
                        strip.set_loop_range(lo, hi);
                        strip.emit_by_name::<()>("loop-range-changed",
                            &[&(lo as u32), &(hi as u32)]);
                    }
                }
            });
            let widget = self.obj().downgrade();
            drag.connect_drag_end(move |_, _, _| {
                if let Some(strip) = widget.upgrade() {
                    if !strip.imp().drag_selecting.get() {
                        // No drag — treat as a click: move cursor to beat
                        let beat = strip.imp().drag_start_beat.get();
                        if beat >= 0 {
                            // Clear any active loop
                            if strip.imp().loop_start_beat.get() >= 0 {
                                strip.emit_by_name::<()>("loop-cleared", &[]);
                            }
                            strip.imp().selection_anchor.set(beat);
                            strip.set_current_beat(beat);
                            strip.emit_by_name::<()>("beat-seeked", &[&(beat as u32)]);
                        }
                    }
                    strip.imp().drag_selecting.set(false);
                }
            });
            self.obj().add_controller(drag);

            // Keyboard: arrow keys to navigate, shift+arrow to extend selection
            let key_ctrl = gtk::EventControllerKey::new();
            let widget = self.obj().downgrade();
            key_ctrl.connect_key_pressed(move |_, keyval, _, modifier| {
                let Some(strip) = widget.upgrade() else {
                    return glib::Propagation::Proceed;
                };
                let shift = modifier.contains(gdk::ModifierType::SHIFT_MASK);
                let ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);
                let max_beat = strip.imp().total_beats.get() as i32 - 1;
                if max_beat < 0 {
                    return glib::Propagation::Proceed;
                }

                match keyval {
                    gdk::Key::Left | gdk::Key::Right => {
                        let direction = if keyval == gdk::Key::Left { -1 } else { 1 };

                        if shift {
                            let moving_edge = strip.get_moving_edge();
                            let target = if ctrl {
                                strip.snap_to_bar_boundary(moving_edge, direction, max_beat)
                            } else {
                                (moving_edge + direction).clamp(0, max_beat)
                            };
                            strip.set_selection_edge(target, max_beat);
                        } else {
                            let current = strip.imp().current_beat.get().max(0);
                            let target = if ctrl {
                                strip.snap_to_bar_boundary(current, direction, max_beat)
                            } else {
                                (current + direction).clamp(0, max_beat)
                            };
                            strip.move_cursor(target - current, max_beat);
                        }
                        glib::Propagation::Stop
                    }
                    gdk::Key::Escape => {
                        if strip.imp().loop_start_beat.get() >= 0 {
                            strip.emit_by_name::<()>("loop-cleared", &[]);
                            strip.imp().selection_anchor.set(-1);
                        }
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            });
            self.obj().add_controller(key_ctrl);
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            use std::sync::OnceLock;
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("beat-seeked")
                        .param_types([u32::static_type()])
                        .build(),
                    glib::subclass::Signal::builder("loop-range-changed")
                        .param_types([u32::static_type(), u32::static_type()])
                        .build(),
                    glib::subclass::Signal::builder("loop-cleared")
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for MtrTabStrip {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => {
                    let natural =
                        (LEFT_MARGIN + self.total_beats.get() as f32 * BEAT_WIDTH + LEFT_MARGIN)
                            as i32;
                    (200, natural.max(200), -1, -1)
                }
                _ => {
                    let natural = (TOP_MARGIN
                        + STRING_SPACING * (STRING_COUNT - 1) as f32
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
            let beats = self.beats.borrow();
            let bar_boundaries = self.bar_boundaries.borrow();
            let current_beat = self.current_beat.get();
            let loop_start = self.loop_start_beat.get();
            let loop_end = self.loop_end_beat.get();

            // Draw string lines
            for string_index in 0..STRING_COUNT {
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                let color = with_alpha(&fg, 0.15);
                snapshot.append_color(
                    &color,
                    &graphene::Rect::new(LEFT_MARGIN, y - 0.5, width - LEFT_MARGIN, 1.0),
                );
            }

            // Draw string labels
            for (string_index, label) in STRING_LABELS.iter().enumerate() {
                let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                draw_text(&*widget, snapshot, &with_alpha(&fg, 0.4), 12.0, y, label, 10.0);
            }

            // Draw loop range shading
            if loop_start >= 0 && loop_end >= 0 {
                let start_x = LEFT_MARGIN + loop_start as f32 * BEAT_WIDTH;
                let end_x = LEFT_MARGIN + (loop_end + 1) as f32 * BEAT_WIDTH;
                let loop_color = with_alpha(&accent, 0.1);
                let fretboard_height = STRING_SPACING * (STRING_COUNT - 1) as f32;
                snapshot.append_color(
                    &loop_color,
                    &graphene::Rect::new(
                        start_x,
                        TOP_MARGIN - 4.0,
                        end_x - start_x,
                        fretboard_height + 8.0,
                    ),
                );
            }

            // Draw bar lines
            for boundary in bar_boundaries.iter() {
                let x = LEFT_MARGIN + boundary.beat_index as f32 * BEAT_WIDTH;
                let bar_color = with_alpha(&fg, 0.25);
                let fretboard_height = STRING_SPACING * (STRING_COUNT - 1) as f32;
                snapshot.append_color(
                    &bar_color,
                    &graphene::Rect::new(x - 0.5, TOP_MARGIN - 4.0, 1.0, fretboard_height + 8.0),
                );

                // Time signature label (only if it changed)
                if boundary.show_time_sig {
                    let ts_label = format!("{}/{}", boundary.time_sig_num, boundary.time_sig_denom);
                    draw_text(
                        &widget,
                        snapshot,
                        &with_alpha(&fg, 0.35),
                        x + 2.0,
                        TOP_MARGIN - 14.0,
                        &ts_label,
                        8.0,
                    );
                }
            }

            // Draw beat data (fret numbers)
            for (beat_offset, strip_beat) in beats.iter().enumerate() {
                let x = LEFT_MARGIN + beat_offset as f32 * BEAT_WIDTH + BEAT_WIDTH / 2.0;
                let is_current = beat_offset as i32 == current_beat;

                if strip_beat.is_rest {
                    // Draw rest symbol
                    let rest_color = if is_current {
                        with_alpha(&accent, 0.8)
                    } else {
                        with_alpha(&fg, 0.3)
                    };
                    for string_index in 0..STRING_COUNT {
                        let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                        draw_text(&*widget, snapshot, &rest_color, x, y, "–", 10.0);
                    }
                    continue;
                }

                for note in &strip_beat.notes {
                    let string_index = (note.string as usize).saturating_sub(1);
                    if string_index >= STRING_COUNT {
                        continue;
                    }
                    let y = TOP_MARGIN + string_index as f32 * STRING_SPACING;
                    let fret_text = format!("{}", note.fret);

                    if is_current {
                        // Highlight with accent color background
                        let bg_size = 14.0;
                        let bg_rect = graphene::Rect::new(
                            x - bg_size / 2.0,
                            y - bg_size / 2.0,
                            bg_size,
                            bg_size,
                        );
                        let rounded = gtk::gsk::RoundedRect::from_rect(bg_rect, 3.0);
                        snapshot.push_rounded_clip(&rounded);
                        snapshot.append_color(&with_alpha(&accent, 0.8), &bg_rect);
                        snapshot.pop();

                        let text_color = contrast_color(&accent);
                        draw_text(&*widget, snapshot, &text_color, x, y, &fret_text, 10.0);
                    } else {
                        draw_text(&*widget, snapshot, &fg, x, y, &fret_text, 10.0);
                    }
                }
            }

            // Draw cursor line
            if current_beat >= 0 {
                let cursor_x =
                    LEFT_MARGIN + current_beat as f32 * BEAT_WIDTH + BEAT_WIDTH / 2.0;
                let fretboard_height = STRING_SPACING * (STRING_COUNT - 1) as f32;
                let cursor_color = with_alpha(&accent, 0.4);
                snapshot.append_color(
                    &cursor_color,
                    &graphene::Rect::new(
                        cursor_x - 0.5,
                        TOP_MARGIN - 4.0,
                        1.0,
                        fretboard_height + 8.0,
                    ),
                );
            }

            // Draw loop boundary triangle handles above the stave
            if loop_start >= 0 && loop_end >= 0 {
                let handle_y = TOP_MARGIN - 10.0;
                let handle_size = 8.0f32;
                let handle_color = with_alpha(&accent, 0.85);

                for &handle_beat in &[loop_start, loop_end] {
                    let hx = LEFT_MARGIN + handle_beat as f32 * BEAT_WIDTH + BEAT_WIDTH / 2.0;
                    // Downward-pointing triangle (approximated with stacked rects)
                    for row in 0..6 {
                        let fraction = row as f32 / 5.0;
                        let half_width = handle_size * (1.0 - fraction);
                        let ry = handle_y + fraction * handle_size * 1.2;
                        snapshot.append_color(
                            &handle_color,
                            &graphene::Rect::new(hx - half_width, ry, half_width * 2.0, 2.0),
                        );
                    }
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct MtrTabStrip(ObjectSubclass<imp::MtrTabStrip>)
        @extends gtk::Widget,
        @implements gtk::Accessible;
}

#[derive(Debug, Clone)]
struct StripBeat {
    notes: Vec<StripNote>,
    is_rest: bool,
}

#[derive(Debug, Clone)]
struct StripNote {
    string: u8,
    fret: u8,
}

#[derive(Debug, Clone)]
struct BarBoundary {
    beat_index: usize,
    time_sig_num: u8,
    time_sig_denom: u8,
    show_time_sig: bool,
}

impl MtrTabStrip {
    pub fn set_score(&self, score: &TabScore) {
        let mut strip_beats = Vec::with_capacity(score.beats.len());
        for beat in &score.beats {
            strip_beats.push(StripBeat {
                notes: beat
                    .notes
                    .iter()
                    .map(|note| StripNote {
                        string: note.string,
                        fret: note.fret,
                    })
                    .collect(),
                is_rest: beat.is_rest,
            });
        }

        let mut bar_boundaries = Vec::with_capacity(score.bars.len());
        let mut prev_num: u8 = 0;
        let mut prev_denom: u8 = 0;
        for bar in &score.bars {
            let show_time_sig =
                bar.time_sig_num != prev_num || bar.time_sig_denom != prev_denom;
            bar_boundaries.push(BarBoundary {
                beat_index: bar.first_beat_index,
                time_sig_num: bar.time_sig_num,
                time_sig_denom: bar.time_sig_denom,
                show_time_sig,
            });
            prev_num = bar.time_sig_num;
            prev_denom = bar.time_sig_denom;
        }

        self.imp().total_beats.set(score.beats.len());
        self.imp().beats.replace(strip_beats);
        self.imp().bar_boundaries.replace(bar_boundaries);
        self.queue_resize();
        self.queue_draw();
    }

    pub fn set_current_beat(&self, beat_index: i32) {
        if self.imp().current_beat.get() != beat_index {
            self.imp().current_beat.set(beat_index);
            self.queue_draw();
            self.auto_scroll(beat_index);
        }
    }

    pub fn set_loop_range(&self, start_beat: i32, end_beat: i32) {
        self.imp().loop_start_beat.set(start_beat);
        self.imp().loop_end_beat.set(end_beat);
        self.queue_draw();
    }

    /// Convert a widget-relative X coordinate to a beat index.
    /// Accounts for the parent ScrolledWindow scroll offset.
    pub fn x_to_beat(&self, widget_x: f64) -> i32 {
        let scroll_offset = self
            .parent()
            .and_then(|parent| parent.downcast::<gtk::ScrolledWindow>().ok())
            .map(|scrolled| scrolled.hadjustment().value())
            .unwrap_or(0.0);

        let content_x = widget_x + scroll_offset;
        let beat = ((content_x - LEFT_MARGIN as f64) / BEAT_WIDTH as f64) as i32;
        beat.clamp(0, self.imp().total_beats.get() as i32 - 1)
    }

    /// Hit-test loop handles. Returns 1 for start handle, 2 for end, 0 for miss.
    fn hit_test_handle(&self, x: f64) -> u8 {
        let ls = self.imp().loop_start_beat.get();
        let le = self.imp().loop_end_beat.get();
        if ls < 0 || le < 0 {
            return 0;
        }
        let hit_radius = BEAT_WIDTH as f64 * 0.8;
        let scroll_offset = self
            .parent()
            .and_then(|p| p.downcast::<gtk::ScrolledWindow>().ok())
            .map(|s| s.hadjustment().value())
            .unwrap_or(0.0);
        let content_x = x + scroll_offset;

        let start_x = LEFT_MARGIN as f64 + ls as f64 * BEAT_WIDTH as f64 + BEAT_WIDTH as f64 / 2.0;
        let end_x = LEFT_MARGIN as f64 + le as f64 * BEAT_WIDTH as f64 + BEAT_WIDTH as f64 / 2.0;

        let dist_start = (content_x - start_x).abs();
        let dist_end = (content_x - end_x).abs();

        if dist_start < hit_radius && dist_start <= dist_end {
            1
        } else if dist_end < hit_radius {
            2
        } else {
            0
        }
    }

    /// Move cursor by `delta` beats (no selection). Clears any active loop.
    fn move_cursor(&self, delta: i32, max_beat: i32) {
        let current = self.imp().current_beat.get().max(0);
        let target = (current + delta).clamp(0, max_beat);

        if self.imp().loop_start_beat.get() >= 0 {
            self.emit_by_name::<()>("loop-cleared", &[]);
        }
        self.imp().selection_anchor.set(target);
        self.set_current_beat(target);
        self.emit_by_name::<()>("beat-seeked", &[&(target as u32)]);
    }

    /// Get the moving edge of the selection (the edge furthest from anchor).
    fn get_moving_edge(&self) -> i32 {
        let anchor = self.imp().selection_anchor.get();
        let ls = self.imp().loop_start_beat.get();
        let le = self.imp().loop_end_beat.get();
        if ls < 0 {
            anchor.max(self.imp().current_beat.get().max(0))
        } else if (le - anchor).abs() > (ls - anchor).abs() {
            le
        } else {
            ls
        }
    }

    /// Set one edge of the selection to `target`, keeping the anchor fixed.
    fn set_selection_edge(&self, target: i32, max_beat: i32) {
        let mut anchor = self.imp().selection_anchor.get();
        if anchor < 0 {
            anchor = self.imp().current_beat.get().max(0);
            self.imp().selection_anchor.set(anchor);
        }
        let target = target.clamp(0, max_beat);
        let lo = anchor.min(target);
        let hi = anchor.max(target);
        self.set_loop_range(lo, hi);
        self.emit_by_name::<()>("loop-range-changed", &[&(lo as u32), &(hi as u32)]);
        self.auto_scroll(target);
    }

    /// Return the absolute beat position of the nearest bar boundary in `direction`.
    fn snap_to_bar_boundary(&self, from_beat: i32, direction: i32, max_beat: i32) -> i32 {
        let current = from_beat.max(0) as usize;
        let boundaries = self.imp().bar_boundaries.borrow();

        if direction > 0 {
            // Find the next bar boundary that's meaningfully ahead.
            // Skip any boundary at current+1 (we're already at its preceding beat).
            for boundary in boundaries.iter() {
                if boundary.beat_index > current + 1 {
                    return (boundary.beat_index as i32 - 1).min(max_beat);
                }
            }
            max_beat
        } else {
            // Find the bar boundary before the current position.
            // If we're at a bar start, go to the previous bar's start.
            let mut prev_prev = 0i32;
            let mut prev = 0i32;
            for boundary in boundaries.iter() {
                if boundary.beat_index as i32 >= from_beat {
                    break;
                }
                prev_prev = prev;
                prev = boundary.beat_index as i32;
            }
            // If current is already at prev, go to prev_prev
            if prev == from_beat {
                prev_prev
            } else {
                prev
            }
        }
    }

    fn auto_scroll(&self, beat_index: i32) {
        if beat_index < 0 {
            return;
        }
        // Find the parent ScrolledWindow and adjust its hadjustment
        let mut parent = self.parent();
        while let Some(ref widget) = parent {
            if let Some(scrolled) = widget.downcast_ref::<gtk::ScrolledWindow>() {
                let cursor_x = LEFT_MARGIN as f64 + beat_index as f64 * BEAT_WIDTH as f64;
                let viewport_width = scrolled.width() as f64;
                let target_offset = cursor_x - viewport_width * 0.25;

                let adjustment = scrolled.hadjustment();
                let current = adjustment.value();
                if cursor_x < current || cursor_x > current + viewport_width * 0.75 {
                    adjustment.set_value(target_offset.max(0.0));
                }
                break;
            }
            parent = widget.parent();
        }
    }
}

fn with_alpha(color: &gdk::RGBA, alpha: f32) -> gdk::RGBA {
    gdk::RGBA::new(color.red(), color.green(), color.blue(), alpha)
}

fn contrast_color(bg: &gdk::RGBA) -> gdk::RGBA {
    let luminance = 0.299 * bg.red() + 0.587 * bg.green() + 0.114 * bg.blue();
    if luminance > 0.5 {
        gdk::RGBA::new(0.0, 0.0, 0.0, 0.9)
    } else {
        gdk::RGBA::new(1.0, 1.0, 1.0, 0.9)
    }
}

fn lookup_accent_color(widget: &MtrTabStrip) -> gdk::RGBA {
    #[allow(deprecated)]
    if let Some(color) = widget.style_context().lookup_color("accent_bg_color") {
        return color;
    }
    gdk::RGBA::new(0.21, 0.52, 0.89, 1.0)
}

fn draw_text(
    widget: &MtrTabStrip,
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
