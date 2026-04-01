use crate::scale_data::{Scale, NOTE_NAMES, NUM_FRETS, STANDARD_TUNING};

pub struct ChordStructure {
    pub label: &'static str,
    pub offsets: &'static [usize],
    pub tone_labels: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoicingNote {
    pub string_index: usize,
    pub fret: usize,
    pub label: String,
}

pub const CHORD_STRUCTURES: &[ChordStructure] = &[
    ChordStructure { label: "Triad",  offsets: &[0, 2, 4],        tone_labels: &["R", "3", "5"] },
    ChordStructure { label: "7th",    offsets: &[0, 2, 4, 6],     tone_labels: &["R", "3", "5", "7"] },
    ChordStructure { label: "9th",    offsets: &[0, 2, 4, 6, 1],  tone_labels: &["R", "3", "5", "7", "9"] },
    ChordStructure { label: "11th",   offsets: &[0, 4, 6, 1, 3],  tone_labels: &["R", "5", "7", "9", "11"] },
    ChordStructure { label: "13th",   offsets: &[0, 2, 6, 5],     tone_labels: &["R", "3", "7", "13"] },
    ChordStructure { label: "6th",    offsets: &[0, 2, 4, 5],     tone_labels: &["R", "3", "5", "6"] },
    ChordStructure { label: "add9",   offsets: &[0, 2, 4, 1],     tone_labels: &["R", "3", "5", "9"] },
    ChordStructure { label: "add11",  offsets: &[0, 2, 4, 3],     tone_labels: &["R", "3", "5", "11"] },
    ChordStructure { label: "sus4",   offsets: &[0, 3, 4],        tone_labels: &["R", "4", "5"] },
    ChordStructure { label: "sus2",   offsets: &[0, 1, 4],        tone_labels: &["R", "2", "5"] },
];

/// Generates a playable chord voicing near `center_fret`.
///
/// Assigns one chord tone per string within a 5-fret span, trying to include
/// all chord tones at least once. `inversion` controls which tone is the bass note.
pub fn generate_voicing(
    root: u8,
    scale: &Scale,
    degree: usize,
    structure: &ChordStructure,
    center_fret: usize,
    inversion: usize,
) -> Vec<VoicingNote> {
    let scale_len = scale.intervals.len();
    let tone_count = structure.offsets.len();

    let chord_tones: Vec<u8> = structure
        .offsets
        .iter()
        .map(|&offset| {
            let scale_index = (degree + offset) % scale_len;
            (root + scale.intervals[scale_index]) % 12
        })
        .collect();

    let fret_min = center_fret.saturating_sub(2);
    let fret_max = (center_fret + 3).min(NUM_FRETS);

    #[derive(Clone)]
    struct Candidate {
        fret: usize,
        tone_index: usize,
    }

    let candidates_per_string: Vec<Vec<Candidate>> = (0..6)
        .map(|phys_string| {
            let open_note = STANDARD_TUNING[phys_string] as u8;
            let mut candidates: Vec<Candidate> = (fret_min..=fret_max)
                .filter_map(|fret| {
                    let note = (open_note + fret as u8) % 12;
                    chord_tones
                        .iter()
                        .position(|&tone| tone == note)
                        .map(|tone_index| Candidate { fret, tone_index })
                })
                .collect();
            candidates.sort_by_key(|candidate| {
                (candidate.fret as isize - center_fret as isize).unsigned_abs()
            });
            candidates
        })
        .collect();

    let mut assigned: Vec<Option<Candidate>> = vec![None; 6];
    let mut used_tones: Vec<bool> = vec![false; tone_count];
    let bass_target = inversion % tone_count;

    // Step 1: assign the inversion's bass note to the lowest string that has it
    for phys_string in 0..6 {
        let found = candidates_per_string[phys_string]
            .iter()
            .find(|candidate| candidate.tone_index == bass_target);
        if let Some(candidate) = found {
            used_tones[candidate.tone_index] = true;
            assigned[phys_string] = Some(candidate.clone());
            break;
        }
    }

    // Step 2: fill remaining strings -- prefer unused chord tones, then closest to center
    for phys_string in 0..6 {
        if assigned[phys_string].is_some() {
            continue;
        }
        let candidates = &candidates_per_string[phys_string];
        if candidates.is_empty() {
            continue;
        }

        let unused_candidate = candidates
            .iter()
            .find(|candidate| !used_tones[candidate.tone_index]);
        let choice = unused_candidate.unwrap_or(&candidates[0]);
        used_tones[choice.tone_index] = true;
        assigned[phys_string] = Some(choice.clone());
    }

    assigned
        .into_iter()
        .enumerate()
        .filter_map(|(phys_string, slot)| {
            slot.map(|candidate| VoicingNote {
                string_index: 5 - phys_string,
                fret: candidate.fret,
                label: structure.tone_labels[candidate.tone_index].to_string(),
            })
        })
        .collect()
}

/// Computes the chord symbol (e.g. "Em7", "Bdim") for a chord built on `degree`
/// with the given `structure` in the context of `scale` rooted at `root`.
pub fn chord_symbol(
    root: u8,
    scale: &Scale,
    degree: usize,
    structure: &ChordStructure,
) -> String {
    let scale_len = scale.intervals.len();
    let root_interval = scale.intervals[degree % scale_len];
    let chord_root = (root + root_interval) % 12;

    let interval_for = |offset: usize| -> Option<u8> {
        if structure.offsets.contains(&offset) {
            let raw = scale.intervals[(degree + offset) % scale_len] as i16
                - root_interval as i16
                + 12;
            Some((raw % 12) as u8)
        } else {
            None
        }
    };

    let third = interval_for(2);
    let fifth = interval_for(4);
    let seventh = interval_for(6);

    let is_maj_third = third == Some(4);
    let is_min_third = third == Some(3);
    let is_dim_fifth = fifth == Some(6);
    let is_aug_fifth = fifth == Some(8);
    let is_perf_fifth = fifth == Some(7);
    let is_maj7 = seventh == Some(11);
    let is_min7 = seventh == Some(10);
    let is_dim7 = seventh == Some(9);

    let suffix = match structure.label {
        "Triad" => match () {
            _ if is_maj_third && is_perf_fifth => "",
            _ if is_min_third && is_perf_fifth => "m",
            _ if is_min_third && is_dim_fifth => "dim",
            _ if is_maj_third && is_aug_fifth => "aug",
            _ => "?",
        }
        .to_string(),

        "7th" => match () {
            _ if is_maj_third && !is_dim_fifth && !is_aug_fifth && is_maj7 => "maj7",
            _ if is_min_third && !is_dim_fifth && is_min7 => "m7",
            _ if is_maj_third && !is_dim_fifth && !is_aug_fifth && is_min7 => "7",
            _ if is_min_third && is_dim_fifth && is_min7 => "m7\u{266D}5",
            _ if is_min_third && is_dim_fifth && is_dim7 => "dim7",
            _ if is_maj_third && is_aug_fifth && is_maj7 => "aug(\u{0394})",
            _ if is_min_third && !is_dim_fifth && is_maj7 => "m(\u{0394})",
            _ if is_maj_third && is_aug_fifth && is_min7 => "aug7",
            _ => "7?",
        }
        .to_string(),

        "9th" => {
            let ninth = interval_for(1).unwrap_or(0);
            let _ninth_mod = match ninth {
                2 => "",
                1 => "\u{266D}9",
                3 => "\u{266F}9",
                _ => "?",
            };
            match () {
                _ if is_maj_third && is_maj7 => "maj9".to_string(),
                _ if is_min_third && is_min7 && !is_dim_fifth => "m9".to_string(),
                _ if is_maj_third && is_min7 => {
                    if ninth == 2 {
                        "9".to_string()
                    } else {
                        format!("7({_ninth_mod})")
                    }
                }
                _ if is_min_third && is_maj7 => "m(\u{0394})9".to_string(),
                _ if is_min_third && is_dim_fifth && is_min7 => "m9\u{266D}5".to_string(),
                _ => "9?".to_string(),
            }
        }

        "11th" => match () {
            _ if is_maj7 => "maj11",
            _ if is_min7 => "11",
            _ => "11?",
        }
        .to_string(),

        "13th" => match () {
            _ if is_maj_third && is_min7 => "13",
            _ if is_maj_third && is_maj7 => "maj13",
            _ if is_min_third && is_min7 => "m13",
            _ if is_min_third && is_maj7 => "m(\u{0394})13",
            _ => "13?",
        }
        .to_string(),

        "6th" => {
            let triad_quality = match () {
                _ if is_maj_third && is_perf_fifth => "",
                _ if is_min_third && is_perf_fifth => "m",
                _ => "?",
            };
            format!("{triad_quality}6")
        }

        "add9" => match () {
            _ if is_maj_third => "add9",
            _ if is_min_third => "m(add9)",
            _ => "add9?",
        }
        .to_string(),

        "add11" => match () {
            _ if is_maj_third => "add11",
            _ if is_min_third => "m(add11)",
            _ => "add11?",
        }
        .to_string(),

        "sus4" => {
            let fourth = interval_for(3).unwrap_or(0);
            match () {
                _ if fourth == 5 && is_perf_fifth => "sus4",
                _ if fourth == 6 && is_perf_fifth => "sus\u{266F}4",
                _ => "sus4?",
            }
            .to_string()
        }

        "sus2" => {
            let second = interval_for(1).unwrap_or(0);
            match () {
                _ if second == 2 && is_perf_fifth => "sus2",
                _ if second == 1 && is_perf_fifth => "sus\u{266D}2",
                _ => "sus2?",
            }
            .to_string()
        }

        _ => "?".to_string(),
    };

    format!("{}{}", NOTE_NAMES[chord_root as usize], suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c_ionian() -> &'static Scale {
        &crate::scale_data::ALL_FAMILIES[0].scales[0]  // Major family, Ionian mode
    }

    const ROOT_C: u8 = 0;
    const DEGREE_0: usize = 0;
    const DEGREE_1: usize = 1;
    const DEGREE_4: usize = 4;
    const DEGREE_6: usize = 6;
    const CENTER_FRET_5: usize = 5;
    const CENTER_FRET_12: usize = 12;
    const INVERSION_0: usize = 0;
    const INVERSION_1: usize = 1;

    fn triad() -> &'static ChordStructure {
        &CHORD_STRUCTURES[0]
    }

    fn seventh() -> &'static ChordStructure {
        &CHORD_STRUCTURES[1]
    }

    // ── chord_symbol tests ──

    #[test]
    fn test_c_ionian_degree0_triad_gives_c() {
        let scale = c_ionian();
        assert_eq!(chord_symbol(ROOT_C, &scale, DEGREE_0, triad()), "C");
    }

    #[test]
    fn test_c_ionian_degree1_triad_gives_dm() {
        let scale = c_ionian();
        assert_eq!(chord_symbol(ROOT_C, &scale, DEGREE_1, triad()), "Dm");
    }

    #[test]
    fn test_c_ionian_degree6_triad_gives_bdim() {
        let scale = c_ionian();
        assert_eq!(chord_symbol(ROOT_C, &scale, DEGREE_6, triad()), "Bdim");
    }

    #[test]
    fn test_c_ionian_degree0_7th_gives_cmaj7() {
        let scale = c_ionian();
        assert_eq!(chord_symbol(ROOT_C, &scale, DEGREE_0, seventh()), "Cmaj7");
    }

    #[test]
    fn test_c_ionian_degree4_7th_gives_g7() {
        let scale = c_ionian();
        assert_eq!(chord_symbol(ROOT_C, &scale, DEGREE_4, seventh()), "G7");
    }

    #[test]
    fn test_c_ionian_degree1_7th_gives_dm7() {
        let scale = c_ionian();
        assert_eq!(chord_symbol(ROOT_C, &scale, DEGREE_1, seventh()), "Dm7");
    }

    #[test]
    fn test_c_ionian_degree6_7th_gives_bm7b5() {
        let scale = c_ionian();
        assert_eq!(
            chord_symbol(ROOT_C, &scale, DEGREE_6, seventh()),
            "Bm7\u{266D}5"
        );
    }

    // ── voicing generation tests ──

    #[test]
    fn test_voicing_has_one_note_per_string_within_window() {
        let scale = c_ionian();
        let voicing =
            generate_voicing(ROOT_C, &scale, DEGREE_0, triad(), CENTER_FRET_5, INVERSION_0);

        assert!(
            !voicing.is_empty(),
            "voicing should have at least one note"
        );
        assert!(voicing.len() <= 6, "voicing should have at most 6 notes");

        let fret_min = CENTER_FRET_5.saturating_sub(2);
        let fret_max = CENTER_FRET_5 + 3;

        let mut seen_strings = std::collections::HashSet::new();
        for note in &voicing {
            assert!(
                note.fret >= fret_min && note.fret <= fret_max,
                "fret {} outside [{}, {}]",
                note.fret,
                fret_min,
                fret_max,
            );
            assert!(
                seen_strings.insert(note.string_index),
                "duplicate string_index {}",
                note.string_index,
            );
        }
    }

    #[test]
    fn test_voicing_contains_correct_chord_tones() {
        let scale = c_ionian();
        let voicing =
            generate_voicing(ROOT_C, &scale, DEGREE_0, triad(), CENTER_FRET_5, INVERSION_0);

        let labels: std::collections::HashSet<&str> =
            voicing.iter().map(|note| note.label.as_str()).collect();
        assert!(labels.contains("R"), "voicing should contain root");
        assert!(labels.contains("3"), "voicing should contain 3rd");
        assert!(labels.contains("5"), "voicing should contain 5th");
    }

    #[test]
    fn test_inversion0_puts_root_in_bass() {
        let scale = c_ionian();
        let voicing =
            generate_voicing(ROOT_C, &scale, DEGREE_0, triad(), CENTER_FRET_5, INVERSION_0);

        let bass_note = voicing
            .iter()
            .min_by_key(|note| std::cmp::Reverse(note.string_index))
            .expect("voicing should not be empty");
        assert_eq!(
            bass_note.label, "R",
            "inversion 0 should have root in bass"
        );
    }

    #[test]
    fn test_inversion1_puts_third_in_bass() {
        let scale = c_ionian();
        // Use center_fret=12 where the low E string can play E (fret 12)
        let voicing =
            generate_voicing(ROOT_C, &scale, DEGREE_0, triad(), CENTER_FRET_12, INVERSION_1);

        let bass_note = voicing
            .iter()
            .min_by_key(|note| std::cmp::Reverse(note.string_index))
            .expect("voicing should not be empty");
        assert_eq!(
            bass_note.label, "3",
            "inversion 1 should have 3rd in bass"
        );
    }

    #[test]
    fn test_all_10_structures_work_for_c_ionian_degree0() {
        let scale = c_ionian();
        for (structure_index, structure) in CHORD_STRUCTURES.iter().enumerate() {
            let voicing =
                generate_voicing(ROOT_C, &scale, DEGREE_0, structure, CENTER_FRET_5, INVERSION_0);
            assert!(
                !voicing.is_empty(),
                "structure {} ({}) produced empty voicing",
                structure_index,
                structure.label,
            );

            let symbol = chord_symbol(ROOT_C, &scale, DEGREE_0, structure);
            assert!(
                !symbol.is_empty(),
                "structure {} ({}) produced empty symbol",
                structure_index,
                structure.label,
            );
        }
    }

    #[test]
    fn test_all_10_structures_symbols_for_c_ionian_degree0() {
        let scale = c_ionian();
        let expected = [
            "C", "Cmaj7", "Cmaj9", "Cmaj11", "Cmaj13", "C6", "Cadd9", "Cadd11", "Csus4", "Csus2",
        ];
        for (structure, expected_symbol) in CHORD_STRUCTURES.iter().zip(expected.iter()) {
            let symbol = chord_symbol(ROOT_C, &scale, DEGREE_0, structure);
            assert_eq!(
                &symbol, expected_symbol,
                "structure {} produced '{}', expected '{}'",
                structure.label, symbol, expected_symbol,
            );
        }
    }
}
