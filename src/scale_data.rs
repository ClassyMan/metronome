/// Pure music-theory data for a guitar scales/chords reference tool.
/// No GTK dependencies — just constants, structs, and helper functions.

pub struct Scale {
    pub name: &'static str,
    pub intervals: &'static [u8],
    pub degree_labels: &'static [&'static str],
    pub pentatonic_variants: &'static [&'static [usize]],
}

pub struct ScaleFamily {
    pub name: &'static str,
    pub scales: &'static [Scale],
}

pub const NOTE_NAMES: [&str; 12] = [
    "C", "C\u{266F}", "D", "E\u{266D}", "E", "F",
    "F\u{266F}", "G", "A\u{266D}", "A", "B\u{266D}", "B",
];

/// Semitones from C0 for each open string in standard tuning (low E to high e).
pub const STANDARD_TUNING: [u8; 6] = [4, 9, 14, 19, 23, 28];

pub const NUM_FRETS: usize = 24;
pub const NUM_STRINGS: usize = 6;

pub const FRET_MARKERS: [usize; 10] = [3, 5, 7, 9, 12, 15, 17, 19, 21, 24];
pub const DOUBLE_MARKERS: [usize; 2] = [12, 24];

// ---------------------------------------------------------------------------
// Major family (7 modes)
// ---------------------------------------------------------------------------

static MAJOR_IONIAN: Scale = Scale {
    name: "Ionian",
    intervals: &[0, 2, 4, 5, 7, 9, 11],
    degree_labels: &["1", "2", "3", "4", "5", "6", "7"],
    pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 4, 5, 6]],
};

static MAJOR_DORIAN: Scale = Scale {
    name: "Dorian",
    intervals: &[0, 2, 3, 5, 7, 9, 10],
    degree_labels: &["1", "2", "\u{266D}3", "4", "5", "6", "\u{266D}7"],
    pentatonic_variants: &[&[0, 2, 3, 5, 6], &[0, 2, 3, 4, 5]],
};

static MAJOR_PHRYGIAN: Scale = Scale {
    name: "Phrygian",
    intervals: &[0, 1, 3, 5, 7, 8, 10],
    degree_labels: &["1", "\u{266D}2", "\u{266D}3", "4", "5", "\u{266D}6", "\u{266D}7"],
    pentatonic_variants: &[&[0, 1, 2, 4, 6], &[0, 1, 2, 3, 4]],
};

static MAJOR_LYDIAN: Scale = Scale {
    name: "Lydian",
    intervals: &[0, 2, 4, 6, 7, 9, 11],
    degree_labels: &["1", "2", "3", "\u{266F}4", "5", "6", "7"],
    pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 3, 5, 6]],
};

static MAJOR_MIXOLYDIAN: Scale = Scale {
    name: "Mixolydian",
    intervals: &[0, 2, 4, 5, 7, 9, 10],
    degree_labels: &["1", "2", "3", "4", "5", "6", "\u{266D}7"],
    pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 4, 5, 6]],
};

static MAJOR_AEOLIAN: Scale = Scale {
    name: "Aeolian",
    intervals: &[0, 2, 3, 5, 7, 8, 10],
    degree_labels: &["1", "2", "\u{266D}3", "4", "5", "\u{266D}6", "\u{266D}7"],
    pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 3, 5, 6]],
};

static MAJOR_LOCRIAN: Scale = Scale {
    name: "Locrian",
    intervals: &[0, 1, 3, 5, 6, 8, 10],
    degree_labels: &["1", "\u{266D}2", "\u{266D}3", "4", "\u{266D}5", "\u{266D}6", "\u{266D}7"],
    pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 1, 2, 4, 6]],
};

static MAJOR_SCALES: [Scale; 7] = [
    Scale { name: MAJOR_IONIAN.name, intervals: MAJOR_IONIAN.intervals, degree_labels: MAJOR_IONIAN.degree_labels, pentatonic_variants: MAJOR_IONIAN.pentatonic_variants },
    Scale { name: MAJOR_DORIAN.name, intervals: MAJOR_DORIAN.intervals, degree_labels: MAJOR_DORIAN.degree_labels, pentatonic_variants: MAJOR_DORIAN.pentatonic_variants },
    Scale { name: MAJOR_PHRYGIAN.name, intervals: MAJOR_PHRYGIAN.intervals, degree_labels: MAJOR_PHRYGIAN.degree_labels, pentatonic_variants: MAJOR_PHRYGIAN.pentatonic_variants },
    Scale { name: MAJOR_LYDIAN.name, intervals: MAJOR_LYDIAN.intervals, degree_labels: MAJOR_LYDIAN.degree_labels, pentatonic_variants: MAJOR_LYDIAN.pentatonic_variants },
    Scale { name: MAJOR_MIXOLYDIAN.name, intervals: MAJOR_MIXOLYDIAN.intervals, degree_labels: MAJOR_MIXOLYDIAN.degree_labels, pentatonic_variants: MAJOR_MIXOLYDIAN.pentatonic_variants },
    Scale { name: MAJOR_AEOLIAN.name, intervals: MAJOR_AEOLIAN.intervals, degree_labels: MAJOR_AEOLIAN.degree_labels, pentatonic_variants: MAJOR_AEOLIAN.pentatonic_variants },
    Scale { name: MAJOR_LOCRIAN.name, intervals: MAJOR_LOCRIAN.intervals, degree_labels: MAJOR_LOCRIAN.degree_labels, pentatonic_variants: MAJOR_LOCRIAN.pentatonic_variants },
];

// ---------------------------------------------------------------------------
// Melodic minor family (7 modes)
// ---------------------------------------------------------------------------

static MELODIC_MINOR_SCALES: [Scale; 7] = [
    Scale {
        name: "Melodic Minor",
        intervals: &[0, 2, 3, 5, 7, 9, 11],
        degree_labels: &["1", "2", "\u{266D}3", "4", "5", "6", "7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 4, 5, 6]],
    },
    Scale {
        name: "Dorian \u{266D}2",
        intervals: &[0, 1, 3, 5, 7, 9, 10],
        degree_labels: &["1", "\u{266D}2", "\u{266D}3", "4", "5", "6", "\u{266D}7"],
        pentatonic_variants: &[&[0, 1, 2, 4, 6], &[0, 1, 2, 5, 6]],
    },
    Scale {
        name: "Lydian \u{266F}5",
        intervals: &[0, 2, 4, 6, 8, 9, 11],
        degree_labels: &["1", "2", "3", "\u{266F}4", "\u{266F}5", "6", "7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 4, 5, 6]],
    },
    Scale {
        name: "Lydian \u{266D}7",
        intervals: &[0, 2, 4, 6, 7, 9, 10],
        degree_labels: &["1", "2", "3", "\u{266F}4", "5", "6", "\u{266D}7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 3, 5, 6]],
    },
    Scale {
        name: "Mixolydian \u{266D}6",
        intervals: &[0, 2, 4, 5, 7, 8, 10],
        degree_labels: &["1", "2", "3", "4", "5", "\u{266D}6", "\u{266D}7"],
        pentatonic_variants: &[&[0, 2, 3, 5, 6], &[0, 2, 4, 5, 6]],
    },
    Scale {
        name: "Locrian \u{266E}2",
        intervals: &[0, 2, 3, 5, 6, 8, 10],
        degree_labels: &["1", "2", "\u{266D}3", "4", "\u{266D}5", "\u{266D}6", "\u{266D}7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 1, 2, 4, 6]],
    },
    Scale {
        name: "Altered",
        intervals: &[0, 1, 3, 4, 6, 8, 10],
        degree_labels: &["1", "\u{266D}2", "\u{266D}3", "\u{266D}4", "\u{266D}5", "\u{266D}6", "\u{266D}7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 1, 2, 4, 6]],
    },
];

// ---------------------------------------------------------------------------
// Harmonic minor family (7 modes)
// ---------------------------------------------------------------------------

static HARMONIC_MINOR_SCALES: [Scale; 7] = [
    Scale {
        name: "Harmonic Minor",
        intervals: &[0, 2, 3, 5, 7, 8, 11],
        degree_labels: &["1", "2", "\u{266D}3", "4", "5", "\u{266D}6", "7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 4, 5, 6]],
    },
    Scale {
        name: "Locrian \u{266E}6",
        intervals: &[0, 1, 3, 5, 6, 9, 10],
        degree_labels: &["1", "\u{266D}2", "\u{266D}3", "4", "\u{266D}5", "6", "\u{266D}7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 4, 5, 6]],
    },
    Scale {
        name: "Ionian \u{266F}5",
        intervals: &[0, 2, 4, 5, 8, 9, 11],
        degree_labels: &["1", "2", "3", "4", "\u{266F}5", "6", "7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 4, 5, 6]],
    },
    Scale {
        name: "Dorian \u{266F}4",
        intervals: &[0, 2, 3, 6, 7, 9, 10],
        degree_labels: &["1", "2", "\u{266D}3", "\u{266F}4", "5", "6", "\u{266D}7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 2, 3, 5, 6]],
    },
    Scale {
        name: "Phrygian Dominant",
        intervals: &[0, 1, 4, 5, 7, 8, 10],
        degree_labels: &["1", "\u{266D}2", "3", "4", "5", "\u{266D}6", "\u{266D}7"],
        pentatonic_variants: &[&[0, 1, 2, 4, 6], &[0, 1, 2, 5, 6]],
    },
    Scale {
        name: "Lydian \u{266F}2",
        intervals: &[0, 3, 4, 6, 7, 9, 11],
        degree_labels: &["1", "\u{266F}2", "3", "\u{266F}4", "5", "6", "7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 1, 2, 4, 6]],
    },
    Scale {
        name: "Altered \u{266D}\u{266D}7",
        intervals: &[0, 1, 3, 4, 6, 8, 9],
        degree_labels: &["1", "\u{266D}2", "\u{266D}3", "\u{266D}4", "\u{266D}5", "\u{266D}6", "\u{266D}\u{266D}7"],
        pentatonic_variants: &[&[0, 2, 3, 4, 6], &[0, 1, 2, 4, 6]],
    },
];

// ---------------------------------------------------------------------------
// Messiaen modes of limited transposition (no pentatonic variants)
// ---------------------------------------------------------------------------

static MESSIAEN_SCALES: [Scale; 8] = [
    Scale {
        name: "Mode 1",
        intervals: &[0, 2, 4, 6, 8, 10],
        degree_labels: &["1", "2", "3", "\u{266F}4", "\u{266F}5", "\u{266D}7"],
        pentatonic_variants: &[],
    },
    Scale {
        name: "Mode 2 (HW)",
        intervals: &[0, 1, 3, 4, 6, 7, 9, 10],
        degree_labels: &["1", "\u{266D}2", "\u{266D}3", "3", "\u{266D}5", "5", "6", "\u{266D}7"],
        pentatonic_variants: &[],
    },
    Scale {
        name: "Mode 2 (WH)",
        intervals: &[0, 2, 3, 5, 6, 8, 9, 11],
        degree_labels: &["1", "2", "\u{266D}3", "4", "\u{266D}5", "\u{266D}6", "6", "7"],
        pentatonic_variants: &[],
    },
    Scale {
        name: "Mode 3",
        intervals: &[0, 2, 3, 4, 6, 7, 8, 10, 11],
        degree_labels: &["1", "2", "\u{266D}3", "3", "\u{266F}4", "5", "\u{266D}6", "\u{266D}7", "7"],
        pentatonic_variants: &[],
    },
    Scale {
        name: "Mode 4",
        intervals: &[0, 1, 2, 5, 6, 7, 8, 11],
        degree_labels: &["1", "\u{266D}2", "2", "4", "\u{266F}4", "5", "\u{266D}6", "7"],
        pentatonic_variants: &[],
    },
    Scale {
        name: "Mode 5",
        intervals: &[0, 1, 5, 6, 7, 11],
        degree_labels: &["1", "\u{266D}2", "4", "\u{266F}4", "5", "7"],
        pentatonic_variants: &[],
    },
    Scale {
        name: "Mode 6",
        intervals: &[0, 2, 4, 5, 6, 8, 10, 11],
        degree_labels: &["1", "2", "3", "4", "\u{266F}4", "\u{266D}6", "\u{266D}7", "7"],
        pentatonic_variants: &[],
    },
    Scale {
        name: "Mode 7",
        intervals: &[0, 1, 2, 3, 5, 6, 7, 8, 9, 11],
        degree_labels: &["1", "\u{266D}2", "2", "\u{266D}3", "4", "\u{266F}4", "5", "\u{266D}6", "6", "7"],
        pentatonic_variants: &[],
    },
];

// ---------------------------------------------------------------------------
// Other
// ---------------------------------------------------------------------------

static OTHER_SCALES: [Scale; 1] = [
    Scale {
        name: "Blues",
        intervals: &[0, 3, 5, 6, 7, 10],
        degree_labels: &["1", "\u{266D}3", "4", "\u{266D}5", "5", "\u{266D}7"],
        pentatonic_variants: &[],
    },
];

// ---------------------------------------------------------------------------
// Top-level export
// ---------------------------------------------------------------------------

static FAMILIES: [ScaleFamily; 5] = [
    ScaleFamily { name: "Major", scales: &MAJOR_SCALES },
    ScaleFamily { name: "Melodic Minor", scales: &MELODIC_MINOR_SCALES },
    ScaleFamily { name: "Harmonic Minor", scales: &HARMONIC_MINOR_SCALES },
    ScaleFamily { name: "Messiaen", scales: &MESSIAEN_SCALES },
    ScaleFamily { name: "Other", scales: &OTHER_SCALES },
];

pub const ALL_FAMILIES: &[ScaleFamily] = &FAMILIES;

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Returns the scale degree index (0-based) for a given MIDI-style note
/// within a scale rooted at `root`, or `None` if the note is not in the scale.
/// Both `note` and `root` are absolute pitch values; only their pitch class
/// (mod 12) matters.
pub fn scale_degree(note: u8, root: u8, scale: &Scale) -> Option<usize> {
    let semitones_from_root = ((note as i16 - root as i16).rem_euclid(12)) as u8;
    scale
        .intervals
        .iter()
        .position(|&interval| interval == semitones_from_root)
}

/// A scale supports diatonic chord construction when it has exactly 7 notes
/// (so thirds can be stacked in the usual way).
pub fn has_diatonic_chords(scale: &Scale) -> bool {
    scale.intervals.len() == 7
}

/// Returns the absolute pitch (semitones from C0) at a given fret on a given
/// string, using standard tuning.  The result is **not** reduced mod 12 so
/// the caller can derive both the pitch class and the octave if needed.
pub fn note_at_fret(string_index: usize, fret: usize) -> u8 {
    STANDARD_TUNING[string_index] + fret as u8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_degree_c_ionian() {
        let ionian = &ALL_FAMILIES[0].scales[0];
        assert_eq!(ionian.name, "Ionian");

        // C root = 0 (pitch class of C)
        let root: u8 = 0;

        // Every interval of Ionian should map to its own index
        for (expected_degree, &interval) in ionian.intervals.iter().enumerate() {
            let note = root + interval;
            assert_eq!(
                scale_degree(note, root, ionian),
                Some(expected_degree),
                "interval {} should be degree {}",
                interval,
                expected_degree,
            );
        }

        // C# (1 semitone) is not in C Ionian
        assert_eq!(scale_degree(1, root, ionian), None);
        // Eb (3 semitones) is not in C Ionian
        assert_eq!(scale_degree(3, root, ionian), None);
    }

    #[test]
    fn test_scale_degree_wraps_octaves() {
        let ionian = &ALL_FAMILIES[0].scales[0];
        let root: u8 = 0;

        // Note 14 = C0 + 14 semitones = D1, should still be degree 1 (the "2")
        assert_eq!(scale_degree(14, root, ionian), Some(1));
        // High root in a different octave
        assert_eq!(scale_degree(24, root, ionian), Some(0));
    }

    #[test]
    fn test_note_at_fret_open_strings() {
        // Open strings should equal the STANDARD_TUNING values
        for (string_idx, &expected) in STANDARD_TUNING.iter().enumerate() {
            assert_eq!(
                note_at_fret(string_idx, 0),
                expected,
                "open string {} should be {}",
                string_idx,
                expected,
            );
        }
    }

    #[test]
    fn test_note_at_fret_known_positions() {
        // Low E string (index 0), open = 4 (E). Fret 5 = A = 9
        assert_eq!(note_at_fret(0, 5), 9);
        // A string (index 1), fret 7 = E = 16
        assert_eq!(note_at_fret(1, 7), 16);
        // High e string (index 5), fret 12 = e one octave up = 40
        assert_eq!(note_at_fret(5, 12), 40);
    }

    #[test]
    fn test_has_diatonic_chords_seven_note_scales() {
        // All modes in Major, Melodic Minor, and Harmonic Minor have 7 notes
        for family in &[
            &ALL_FAMILIES[0],
            &ALL_FAMILIES[1],
            &ALL_FAMILIES[2],
        ] {
            for scale in family.scales {
                assert!(
                    has_diatonic_chords(scale),
                    "{} / {} should support diatonic chords (has {} notes)",
                    family.name,
                    scale.name,
                    scale.intervals.len(),
                );
            }
        }
    }

    #[test]
    fn test_has_diatonic_chords_non_seven_note_scales() {
        // Messiaen modes and Blues do NOT all have 7 notes
        let messiaen = &ALL_FAMILIES[3];
        for scale in messiaen.scales {
            // Messiaen modes have 6, 8, 9, or 10 notes -- never exactly 7
            assert!(
                !has_diatonic_chords(scale),
                "Messiaen {} should not support diatonic chords (has {} notes)",
                scale.name,
                scale.intervals.len(),
            );
        }

        let blues = &ALL_FAMILIES[4].scales[0];
        assert!(
            !has_diatonic_chords(blues),
            "Blues should not support diatonic chords (has {} notes)",
            blues.intervals.len(),
        );
    }

    #[test]
    fn test_all_scales_intervals_match_labels() {
        for family in ALL_FAMILIES {
            for scale in family.scales {
                assert_eq!(
                    scale.intervals.len(),
                    scale.degree_labels.len(),
                    "{} / {} has {} intervals but {} labels",
                    family.name,
                    scale.name,
                    scale.intervals.len(),
                    scale.degree_labels.len(),
                );
            }
        }
    }

    #[test]
    fn test_all_pentatonic_indices_in_bounds() {
        for family in ALL_FAMILIES {
            for scale in family.scales {
                for (variant_idx, variant) in scale.pentatonic_variants.iter().enumerate() {
                    assert_eq!(
                        variant.len(),
                        5,
                        "{} / {} pentatonic variant {} has {} notes, expected 5",
                        family.name,
                        scale.name,
                        variant_idx,
                        variant.len(),
                    );
                    for &index in *variant {
                        assert!(
                            index < scale.intervals.len(),
                            "{} / {} pentatonic variant {} has out-of-bounds index {} (scale has {} notes)",
                            family.name,
                            scale.name,
                            variant_idx,
                            index,
                            scale.intervals.len(),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_total_scale_count() {
        let total: usize = ALL_FAMILIES.iter().map(|family| family.scales.len()).sum();
        // 7 major + 7 melodic minor + 7 harmonic minor + 8 messiaen + 1 blues = 30
        assert_eq!(total, 30);
    }

    #[test]
    fn test_note_names_count() {
        assert_eq!(NOTE_NAMES.len(), 12);
    }

    #[test]
    fn test_intervals_start_at_zero_and_stay_in_range() {
        for family in ALL_FAMILIES {
            for scale in family.scales {
                assert_eq!(
                    scale.intervals[0], 0,
                    "{} / {} should start on the root (0)",
                    family.name, scale.name,
                );
                for &interval in scale.intervals {
                    assert!(
                        interval < 12,
                        "{} / {} has interval {} >= 12",
                        family.name,
                        scale.name,
                        interval,
                    );
                }
            }
        }
    }
}
