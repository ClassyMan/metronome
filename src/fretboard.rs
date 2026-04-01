use crate::scale_data::{
    self, Scale, ALL_FAMILIES, DOUBLE_MARKERS, FRET_MARKERS, NUM_FRETS, NUM_STRINGS,
};
use adw::subclass::prelude::*;
use gtk::{gdk, glib, graphene, prelude::*};
use std::cell::Cell;

const FRET_WIDTH: f32 = 52.0;
const NUT_WIDTH: f32 = 6.0;
const LEFT_MARGIN: f32 = 12.0;
const TOP_MARGIN: f32 = 24.0;
const BOTTOM_MARGIN: f32 = 22.0;
const STRING_SPACING: f32 = 28.0;
const NOTE_RADIUS_RATIO: f32 = 0.30;
const NOTE_RADIUS_MIN: f32 = 8.0;
const NOTE_RADIUS_MAX: f32 = 18.0;

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug, glib::Properties)]
    #[properties(wrapper_type = super::MtrFretboard)]
    pub struct MtrFretboard {
        #[property(get, set = Self::set_root, minimum = 0, maximum = 11, default = 0)]
        pub root: Cell<u32>,
        #[property(get, set = Self::set_family_index, minimum = 0, maximum = 4, default = 0)]
        pub family_index: Cell<u32>,
        #[property(get, set = Self::set_mode_index, minimum = 0, maximum = 9, default = 0)]
        pub mode_index: Cell<u32>,
        /// Comma-separated fret assignments per string (display order 0=highE..5=lowE).
        /// Empty string = scale-only mode (no chord voicing).
        #[property(get, set = Self::set_voicing)]
        pub voicing: RefCell<String>,
        /// Comma-separated chord tone labels (e.g. "R,3,5").
        #[property(get, set = Self::set_voicing_labels)]
        pub voicing_labels: RefCell<String>,
        /// 0 = full scale, 1+ = pentatonic variant index (1-based).
        #[property(get, set = Self::set_pentatonic_variant, minimum = 0, maximum = 3, default = 0)]
        pub pentatonic_variant: Cell<u32>,
    }

    impl Default for MtrFretboard {
        fn default() -> Self {
            Self {
                root: Cell::new(0),
                family_index: Cell::new(0),
                mode_index: Cell::new(0),
                voicing: RefCell::new(String::new()),
                voicing_labels: RefCell::new(String::new()),
                pentatonic_variant: Cell::new(0),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MtrFretboard {
        const NAME: &'static str = "MtrFretboard";
        type Type = super::MtrFretboard;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("fretboard");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MtrFretboard {
        fn constructed(&self) {
            self.parent_constructed();

            let gesture = gtk::GestureClick::new();
            let widget = self.obj().downgrade();
            gesture.connect_pressed(move |_, _, x, y| {
                if let Some(fb) = widget.upgrade() {
                    fb.on_click(x, y);
                }
            });
            self.obj().add_controller(gesture);
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
                vec![glib::subclass::Signal::builder("fret-tapped")
                    .param_types([u32::static_type(), u32::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for MtrFretboard {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => {
                    let natural = (LEFT_MARGIN + NUT_WIDTH + FRET_WIDTH * NUM_FRETS as f32) as i32;
                    (200, natural, -1, -1)
                }
                _ => {
                    let natural =
                        (TOP_MARGIN + STRING_SPACING * (NUM_STRINGS - 1) as f32 + BOTTOM_MARGIN)
                            as i32;
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
            let accent = lookup_accent_color(&widget);

            let string_spacing = STRING_SPACING;
            let fretboard_height = string_spacing * (NUM_STRINGS - 1) as f32;
            let note_radius =
                (string_spacing * NOTE_RADIUS_RATIO).clamp(NOTE_RADIUS_MIN, NOTE_RADIUS_MAX);

            let root = self.root.get() as u8;
            let family_idx = self.family_index.get() as usize;
            let mode_idx = self.mode_index.get() as usize;

            let family = &ALL_FAMILIES[family_idx.min(ALL_FAMILIES.len() - 1)];
            let scale = &family.scales[mode_idx.min(family.scales.len() - 1)];

            let pent_var = self.pentatonic_variant.get() as usize;

            let voicing_str = self.voicing.borrow();
            let labels_str = self.voicing_labels.borrow();
            let voicing_frets = parse_voicing(&voicing_str);
            let voicing_labels = parse_labels(&labels_str);
            let has_voicing = !voicing_frets.is_empty();

            // Build pentatonic filter: which degree indices to show
            let pent_filter: Option<&[usize]> = if pent_var > 0 {
                scale
                    .pentatonic_variants
                    .get(pent_var - 1)
                    .copied()
            } else {
                None
            };

            // Draw fret markers (dots)
            draw_fret_markers(snapshot, &fg, string_spacing, fretboard_height);

            // Draw strings
            for string_idx in 0..NUM_STRINGS {
                let y = TOP_MARGIN + string_idx as f32 * string_spacing;
                let thickness = 1.0 + (NUM_STRINGS - 1 - string_idx) as f32 * 0.5;
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
                let x = LEFT_MARGIN + NUT_WIDTH + (fret as f32 - 0.0) * FRET_WIDTH;
                let color = with_alpha(&fg, 0.12);
                snapshot.append_color(
                    &color,
                    &graphene::Rect::new(x, TOP_MARGIN, 1.0, fretboard_height),
                );
            }

            // Draw fret numbers
            for &fret in &[1usize] {
                draw_fret_number(&widget, snapshot, &fg, fret);
            }
            for &fret in FRET_MARKERS.iter() {
                draw_fret_number(&widget, snapshot, &fg, fret);
            }

            // Draw scale notes / chord voicing
            if has_voicing {
                draw_chord_voicing(
                    &widget,
                    snapshot,
                    &fg,
                    &accent,
                    root,
                    scale,
                    &voicing_frets,
                    &voicing_labels,
                    string_spacing,
                    note_radius,
                );
            } else {
                draw_scale_notes(
                    &widget, snapshot, &fg, &accent, root, scale, pent_filter, string_spacing, note_radius,
                );
            }
        }
    }

    impl MtrFretboard {
        fn set_root(&self, val: u32) {
            self.root.set(val);
            self.obj().queue_draw();
        }
        fn set_family_index(&self, val: u32) {
            self.family_index.set(val);
            self.obj().queue_draw();
        }
        fn set_mode_index(&self, val: u32) {
            self.mode_index.set(val);
            self.obj().queue_draw();
        }
        fn set_voicing(&self, val: String) {
            *self.voicing.borrow_mut() = val;
            self.obj().queue_draw();
        }
        fn set_voicing_labels(&self, val: String) {
            *self.voicing_labels.borrow_mut() = val;
            self.obj().queue_draw();
        }
        fn set_pentatonic_variant(&self, val: u32) {
            self.pentatonic_variant.set(val);
            self.obj().queue_draw();
        }
    }
}

glib::wrapper! {
    pub struct MtrFretboard(ObjectSubclass<imp::MtrFretboard>)
        @extends gtk::Widget;
}

impl MtrFretboard {
    fn on_click(&self, x: f64, y: f64) {
        let height = self.height() as f32;
        let fretboard_height = height - TOP_MARGIN - BOTTOM_MARGIN;
        let string_spacing = fretboard_height / (NUM_STRINGS - 1) as f32;

        let string_idx =
            ((y as f32 - TOP_MARGIN) / string_spacing).round() as i32;
        let string_idx = string_idx.clamp(0, (NUM_STRINGS - 1) as i32) as u32;

        let fret_f = (x as f32 - LEFT_MARGIN - NUT_WIDTH) / FRET_WIDTH + 0.5;
        let fret = (fret_f as i32).clamp(0, NUM_FRETS as i32) as u32;

        self.emit_by_name::<()>("fret-tapped", &[&string_idx, &fret]);
    }
}

// ── Drawing helpers ──

fn contrast_color(bg: &gdk::RGBA) -> gdk::RGBA {
    let luminance = 0.299 * bg.red() + 0.587 * bg.green() + 0.114 * bg.blue();
    if luminance > 0.5 {
        gdk::RGBA::new(0.0, 0.0, 0.0, 0.9)
    } else {
        gdk::RGBA::new(1.0, 1.0, 1.0, 0.9)
    }
}

fn with_alpha(color: &gdk::RGBA, alpha: f32) -> gdk::RGBA {
    gdk::RGBA::new(color.red(), color.green(), color.blue(), alpha)
}

fn lookup_accent_color(widget: &MtrFretboard) -> gdk::RGBA {
    // Read accent color from the theme's CSS @define-color accent_bg_color.
    // style_context().lookup_color() is deprecated since GTK 4.10 but there
    // is no non-deprecated replacement for named CSS colors in GTK4's snapshot
    // rendering path. This is the standard approach used by GNOME apps.
    #[allow(deprecated)]
    if let Some(color) = widget.style_context().lookup_color("accent_bg_color") {
        return color;
    }
    gdk::RGBA::new(0.21, 0.52, 0.89, 1.0)
}

fn fret_center_x(fret: usize) -> f32 {
    if fret == 0 {
        LEFT_MARGIN - 10.0
    } else {
        LEFT_MARGIN + NUT_WIDTH + (fret as f32 - 0.5) * FRET_WIDTH
    }
}

fn draw_fret_markers(
    snapshot: &gtk::Snapshot,
    fg: &gdk::RGBA,
    string_spacing: f32,
    fretboard_height: f32,
) {
    let dot_color = with_alpha(fg, 0.06);
    let dot_radius = 5.0f32;

    for &fret in FRET_MARKERS.iter() {
        let x = fret_center_x(fret);
        if DOUBLE_MARKERS.contains(&fret) {
            // Two dots
            let y1 = TOP_MARGIN + string_spacing * 1.5;
            let y2 = TOP_MARGIN + string_spacing * 3.5;
            append_circle(snapshot, &dot_color, x, y1, dot_radius);
            append_circle(snapshot, &dot_color, x, y2, dot_radius);
        } else {
            let y = TOP_MARGIN + fretboard_height / 2.0;
            append_circle(snapshot, &dot_color, x, y, dot_radius);
        }
    }
}

fn draw_fret_number(widget: &MtrFretboard, snapshot: &gtk::Snapshot, fg: &gdk::RGBA, fret: usize) {
    let x = fret_center_x(fret);
    let label = format!("{}", fret);
    let color = with_alpha(fg, 0.35);
    draw_text_on_widget(widget, snapshot, &color, x, TOP_MARGIN - 14.0, &label, 9.0);
}

fn draw_scale_notes(
    widget: &MtrFretboard,
    snapshot: &gtk::Snapshot,
    fg: &gdk::RGBA,
    accent: &gdk::RGBA,
    root: u8,
    scale: &Scale,
    pent_filter: Option<&[usize]>,
    string_spacing: f32,
    note_radius: f32,
) {
    for string_idx in 0..NUM_STRINGS {
        for fret in 0..=NUM_FRETS {
            let note = scale_data::note_at_fret(string_idx, fret);
            if let Some(degree) = scale_data::scale_degree(note, root, scale) {
                let in_pentatonic = pent_filter
                    .map(|indices| indices.contains(&degree))
                    .unwrap_or(true);

                let x = fret_center_x(fret);
                let y = TOP_MARGIN + string_idx as f32 * string_spacing;
                let is_root = degree == 0;

                if !in_pentatonic {
                    // Passing tone: dim dot only
                    let dim = with_alpha(fg, 0.12);
                    append_circle(snapshot, &dim, x, y, note_radius * 0.5);
                } else if is_root {
                    let aura_color = with_alpha(accent, 0.20);
                    append_circle(snapshot, &aura_color, x, y, note_radius * 1.4);
                    append_circle(snapshot, accent, x, y, note_radius);
                    let label_color = contrast_color(accent);
                    draw_text_on_widget(widget,
                        snapshot,
                        &label_color,
                        x,
                        y,
                        scale.degree_labels[degree],
                        9.0,
                    );
                } else {
                    let dot_color = with_alpha(accent, 0.45);
                    append_circle(snapshot, &dot_color, x, y, note_radius * 0.85);
                    let label_color = with_alpha(fg, 0.8);
                    draw_text_on_widget(widget,
                        snapshot,
                        &label_color,
                        x,
                        y,
                        scale.degree_labels[degree],
                        8.0,
                    );
                }
            }
        }
    }
}

fn draw_chord_voicing(
    widget: &MtrFretboard,
    snapshot: &gtk::Snapshot,
    fg: &gdk::RGBA,
    accent: &gdk::RGBA,
    root: u8,
    scale: &Scale,
    voicing_frets: &[(usize, usize)],
    voicing_labels: &[String],
    string_spacing: f32,
    note_radius: f32,
) {
    // Dim all scale notes first
    for string_idx in 0..NUM_STRINGS {
        for fret in 0..=NUM_FRETS {
            let note = scale_data::note_at_fret(string_idx, fret);
            if scale_data::scale_degree(note, root, scale).is_some() {
                let x = fret_center_x(fret);
                let y = TOP_MARGIN + string_idx as f32 * string_spacing;
                let dim = with_alpha(fg, 0.08);
                append_circle(snapshot, &dim, x, y, note_radius * 0.5);
            }
        }
    }

    // Draw voiced notes
    for (idx, &(string_idx, fret)) in voicing_frets.iter().enumerate() {
        let x = fret_center_x(fret);
        let y = TOP_MARGIN + string_idx as f32 * string_spacing;
        let label = voicing_labels
            .get(idx)
            .map(|s| s.as_str())
            .unwrap_or("?");
        let is_root = label == "R";

        if is_root {
            let aura_color = with_alpha(accent, 0.20);
            append_circle(snapshot, &aura_color, x, y, note_radius * 1.4);
            append_circle(snapshot, accent, x, y, note_radius);
            let label_color = gdk::RGBA::new(0.0, 0.0, 0.0, 0.9);
            draw_text_on_widget(widget,snapshot, &label_color, x, y, label, 9.0);
        } else {
            let tone_color = with_alpha(accent, 0.7);
            append_circle(snapshot, &tone_color, x, y, note_radius);
            let label_color = with_alpha(fg, 0.9);
            draw_text_on_widget(widget,snapshot, &label_color, x, y, label, 8.0);
        }
    }
}

fn append_circle(snapshot: &gtk::Snapshot, color: &gdk::RGBA, cx: f32, cy: f32, radius: f32) {
    let rect = graphene::Rect::new(cx - radius, cy - radius, radius * 2.0, radius * 2.0);
    let rounded = gtk::gsk::RoundedRect::from_rect(rect, radius);
    snapshot.push_rounded_clip(&rounded);
    snapshot.append_color(color, &rect);
    snapshot.pop();
}

fn draw_text_on_widget(
    widget: &impl IsA<gtk::Widget>,
    snapshot: &gtk::Snapshot,
    color: &gdk::RGBA,
    cx: f32,
    cy: f32,
    text: &str,
    font_size: f32,
) {
    let pango_context = widget.as_ref().pango_context();
    let mut font_desc = pango_context.font_description().unwrap_or_default();
    font_desc.set_size((font_size * gtk::pango::SCALE as f32) as i32);

    let layout = gtk::pango::Layout::new(&pango_context);
    layout.set_font_description(Some(&font_desc));
    layout.set_text(text);
    layout.set_alignment(gtk::pango::Alignment::Center);

    let (ink_rect, _logical_rect) = layout.pixel_extents();
    let text_width = ink_rect.width() as f32;
    let text_height = ink_rect.height() as f32;

    let tx = cx - text_width / 2.0 - ink_rect.x() as f32;
    let ty = cy - text_height / 2.0 - ink_rect.y() as f32;

    snapshot.save();
    snapshot.translate(&graphene::Point::new(tx, ty));
    snapshot.append_layout(&layout, color);
    snapshot.restore();
}

fn parse_voicing(s: &str) -> Vec<(usize, usize)> {
    if s.is_empty() {
        return vec![];
    }
    s.split(';')
        .filter_map(|pair| {
            let mut parts = pair.split(',');
            let string_idx = parts.next()?.parse().ok()?;
            let fret = parts.next()?.parse().ok()?;
            Some((string_idx, fret))
        })
        .collect()
}

fn parse_labels(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }
    s.split(',').map(|l| l.to_string()).collect()
}
