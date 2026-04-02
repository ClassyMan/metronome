/// Guitar Pro 7/8 (.gp) file parser.
///
/// GP7+ files are ZIP archives containing XML in `Content/score.gpif`.
/// The XML uses a flat ID-based structure: MasterBars → Bars → Voices → Beats → Notes,
/// with Rhythms as a separate lookup table.

use crate::tab_models::*;
use std::collections::HashMap;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug)]
pub enum Gp7Error {
    Io(io::Error),
    Zip(String),
    Xml(String),
}

impl From<io::Error> for Gp7Error {
    fn from(error: io::Error) -> Self {
        Gp7Error::Io(error)
    }
}

impl std::fmt::Display for Gp7Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gp7Error::Io(error) => write!(formatter, "IO error: {}", error),
            Gp7Error::Zip(message) => write!(formatter, "ZIP error: {}", message),
            Gp7Error::Xml(message) => write!(formatter, "XML error: {}", message),
        }
    }
}

type Result<T> = std::result::Result<T, Gp7Error>;

pub fn parse_file(path: &Path) -> Result<(TabScore, usize)> {
    let xml_content = read_gpif_xml(path)?;
    parse_gpif(&xml_content, None)
}

pub fn parse_file_for_track(path: &Path, track_index: usize) -> Result<(TabScore, usize)> {
    let xml_content = read_gpif_xml(path)?;
    parse_gpif(&xml_content, Some(track_index))
}

fn read_gpif_xml(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| Gp7Error::Zip(error.to_string()))?;

    let mut score_file = archive
        .by_name("Content/score.gpif")
        .map_err(|error| Gp7Error::Zip(format!("score.gpif not found: {}", error)))?;
    let mut content = String::new();
    score_file.read_to_string(&mut content)?;
    Ok(content)
}

fn parse_gpif(xml: &str, override_track: Option<usize>) -> Result<(TabScore, usize)> {
    let title = extract_score_field(xml, "Title");
    let artist = extract_score_field(xml, "Artist");
    let tempo = extract_global_tempo(xml);
    let rhythms = extract_rhythms(xml);
    let gp_notes = extract_notes(xml);
    let gp_beats = extract_beats(xml);
    let gp_voices = extract_voices(xml);
    let gp_bars = extract_bars(xml);
    let master_bars = extract_master_bars(xml);
    let tracks = extract_tracks(xml);

    let default_track = override_track.unwrap_or_else(|| {
        tracks
            .iter()
            .position(|track| track.midi_channel != 9)
            .unwrap_or(0)
    });

    let mut score = build_score(
        tempo,
        &rhythms,
        &gp_notes,
        &gp_beats,
        &gp_voices,
        &gp_bars,
        &master_bars,
        &tracks,
        default_track,
    )?;
    score.title = title;
    score.artist = artist;

    Ok((score, default_track))
}

fn extract_global_tempo(xml: &str) -> f64 {
    // Find first Automation with Type=Tempo
    if let Some(tempo_pos) = xml.find("<Type>Tempo</Type>") {
        if let Some(value_start) = xml[tempo_pos..].find("<Value>") {
            let after_value = tempo_pos + value_start + 7;
            if let Some(value_end) = xml[after_value..].find("</Value>") {
                let value_str = &xml[after_value..after_value + value_end];
                // Value format: "120 2" (tempo + denominator)
                if let Some(space_pos) = value_str.find(' ') {
                    return value_str[..space_pos].parse().unwrap_or(120.0);
                }
                return value_str.parse().unwrap_or(120.0);
            }
        }
    }
    120.0
}

#[derive(Debug)]
struct GpRhythm {
    duration_value: i8,
    is_dotted: bool,
    tuplet_enters: u8,
    tuplet_times: u8,
}

fn note_value_to_duration(note_value: &str) -> i8 {
    match note_value {
        "Whole" => -2,
        "Half" => -1,
        "Quarter" => 0,
        "Eighth" => 1,
        "16th" => 2,
        "32nd" => 3,
        "64th" => 4,
        _ => 0,
    }
}

fn extract_rhythms(xml: &str) -> HashMap<usize, GpRhythm> {
    let mut rhythms = HashMap::new();
    let mut search_pos = 0;

    while let Some(rhythm_start) = xml[search_pos..].find("<Rhythm id=\"") {
        let abs_pos = search_pos + rhythm_start;
        let id_start = abs_pos + 12;
        let id_end = match xml[id_start..].find('"') {
            Some(pos) => id_start + pos,
            None => break,
        };
        let rhythm_id: usize = match xml[id_start..id_end].parse() {
            Ok(id) => id,
            Err(_) => {
                search_pos = id_end;
                continue;
            }
        };

        let rhythm_end = match xml[abs_pos..].find("</Rhythm>") {
            Some(pos) => abs_pos + pos,
            None => break,
        };
        let rhythm_xml = &xml[abs_pos..rhythm_end];

        let duration_value = extract_tag_text(rhythm_xml, "NoteValue")
            .map(|value| note_value_to_duration(&value))
            .unwrap_or(0);

        let is_dotted = rhythm_xml.contains("<AugmentationDot ");
        let (tuplet_enters, tuplet_times) = extract_tuplet(rhythm_xml);

        rhythms.insert(
            rhythm_id,
            GpRhythm {
                duration_value,
                is_dotted,
                tuplet_enters,
                tuplet_times,
            },
        );

        search_pos = rhythm_end;
    }

    rhythms
}

fn extract_tuplet(xml: &str) -> (u8, u8) {
    if let Some(pq_start) = xml.find("<PrimaryTuplet>") {
        if let Some(pq_end) = xml[pq_start..].find("</PrimaryTuplet>") {
            let tuplet_xml = &xml[pq_start..pq_start + pq_end];
            let enters: u8 = extract_tag_text(tuplet_xml, "Num")
                .and_then(|value| value.parse().ok())
                .unwrap_or(1);
            let times: u8 = extract_tag_text(tuplet_xml, "Den")
                .and_then(|value| value.parse().ok())
                .unwrap_or(1);
            return (enters, times);
        }
    }
    (1, 1)
}

#[derive(Debug)]
struct GpNote {
    string: u8,
    fret: u8,
}

fn extract_notes(xml: &str) -> HashMap<usize, GpNote> {
    let mut notes = HashMap::new();

    // Find the <Notes> section with <Note id= children
    let section_start = match xml.find("<Notes>\n<Note id=") {
        Some(pos) => pos,
        None => return notes,
    };
    let section_end = match xml[section_start..].find("</Notes>") {
        Some(pos) => section_start + pos,
        None => return notes,
    };
    let section = &xml[section_start..section_end];
    let mut search_pos = 0;

    while let Some(note_start) = section[search_pos..].find("<Note id=\"") {
        let abs_pos = search_pos + note_start;
        let id_start = abs_pos + 10;
        let id_end = match section[id_start..].find('"') {
            Some(pos) => id_start + pos,
            None => break,
        };
        let note_id: usize = match section[id_start..id_end].parse() {
            Ok(id) => id,
            Err(_) => {
                search_pos = id_end;
                continue;
            }
        };

        let note_end = match section[abs_pos..].find("</Note>") {
            Some(pos) => abs_pos + pos,
            None => break,
        };
        let note_xml = &section[abs_pos..note_end];

        let fret = extract_property_fret(note_xml).unwrap_or(0);
        let string = extract_property_string(note_xml).unwrap_or(0);

        notes.insert(note_id, GpNote { string, fret });
        search_pos = note_end;
    }

    notes
}

fn extract_property_fret(xml: &str) -> Option<u8> {
    let prop_start = xml.find("<Property name=\"Fret\">")?;
    let after = &xml[prop_start..];
    let fret_start = after.find("<Fret>")? + 6;
    let fret_end = after[fret_start..].find("</Fret>")?;
    after[fret_start..fret_start + fret_end].parse().ok()
}

fn extract_property_string(xml: &str) -> Option<u8> {
    let prop_start = xml.find("<Property name=\"String\">")?;
    let after = &xml[prop_start..];
    let str_start = after.find("<String>")? + 8;
    let str_end = after[str_start..].find("</String>")?;
    after[str_start..str_start + str_end].parse().ok()
}

#[derive(Debug)]
struct GpBeat {
    rhythm_id: usize,
    note_ids: Vec<usize>,
    is_rest: bool,
}

fn extract_beats(xml: &str) -> HashMap<usize, GpBeat> {
    let mut beats = HashMap::new();
    let mut search_pos = 0;

    // Find the <Beats> section that contains <Beat id= elements
    // (not the <Beats>0 1 2...</Beats> inside Voice elements)
    let beats_section_start = match xml.find("<Beats>\n<Beat id=") {
        Some(pos) => pos,
        None => return beats,
    };
    let beats_section_end = match xml[beats_section_start..].find("</Beats>") {
        Some(pos) => beats_section_start + pos,
        None => return beats,
    };
    let beats_xml = &xml[beats_section_start..beats_section_end];

    while let Some(beat_start) = beats_xml[search_pos..].find("<Beat id=\"") {
        let abs_pos = search_pos + beat_start;
        let id_start = abs_pos + 10;
        let id_end = match beats_xml[id_start..].find('"') {
            Some(pos) => id_start + pos,
            None => break,
        };
        let beat_id: usize = match beats_xml[id_start..id_end].parse() {
            Ok(id) => id,
            Err(_) => {
                search_pos = id_end;
                continue;
            }
        };

        let beat_end = match beats_xml[abs_pos..].find("</Beat>") {
            Some(pos) => abs_pos + pos,
            None => break,
        };
        let beat_xml = &beats_xml[abs_pos..beat_end];

        let rhythm_id = extract_rhythm_ref(beat_xml).unwrap_or(0);
        let note_ids = extract_note_ids(beat_xml);
        let is_rest = beat_xml.contains("<GraceNotes/>")
            || (note_ids.is_empty() && !beat_xml.contains("<Notes>"));

        beats.insert(beat_id, GpBeat { rhythm_id, note_ids, is_rest });
        search_pos = beat_end;
    }

    beats
}

fn extract_rhythm_ref(xml: &str) -> Option<usize> {
    let ref_start = xml.find("<Rhythm ref=\"")? + 13;
    let ref_end = xml[ref_start..].find('"')?;
    xml[ref_start..ref_start + ref_end].parse().ok()
}

fn extract_note_ids(xml: &str) -> Vec<usize> {
    if let Some(notes_text) = extract_tag_text(xml, "Notes") {
        notes_text
            .split_whitespace()
            .filter_map(|id_str| id_str.parse().ok())
            .collect()
    } else {
        Vec::new()
    }
}

#[derive(Debug)]
struct GpVoice {
    beat_ids: Vec<usize>,
}

fn extract_voices(xml: &str) -> HashMap<usize, GpVoice> {
    let mut voices = HashMap::new();

    // Find the <Voices> section with <Voice id= children
    let section_start = match xml.find("<Voices>\n<Voice id=") {
        Some(pos) => pos,
        None => return voices,
    };
    let section_end = match xml[section_start..].find("</Voices>") {
        Some(pos) => section_start + pos,
        None => return voices,
    };
    let section = &xml[section_start..section_end];
    let mut search_pos = 0;

    while let Some(voice_start) = section[search_pos..].find("<Voice id=\"") {
        let abs_pos = search_pos + voice_start;
        let id_start = abs_pos + 11;
        let id_end = match section[id_start..].find('"') {
            Some(pos) => id_start + pos,
            None => break,
        };
        let voice_id: usize = match section[id_start..id_end].parse() {
            Ok(id) => id,
            Err(_) => {
                search_pos = id_end;
                continue;
            }
        };

        let voice_end = match section[abs_pos..].find("</Voice>") {
            Some(pos) => abs_pos + pos,
            None => break,
        };
        let voice_xml = &section[abs_pos..voice_end];

        let beat_ids = extract_tag_text(voice_xml, "Beats")
            .map(|text| {
                text.split_whitespace()
                    .filter_map(|id_str| id_str.parse().ok())
                    .collect()
            })
            .unwrap_or_default();

        voices.insert(voice_id, GpVoice { beat_ids });
        search_pos = voice_end;
    }

    voices
}

#[derive(Debug)]
struct GpBar {
    voice_ids: Vec<usize>,
}

fn extract_bars(xml: &str) -> HashMap<usize, GpBar> {
    let mut bars = HashMap::new();
    let mut search_pos = 0;

    // Find the <Bars> section with <Bar id= children
    let bars_section_start = match xml.find("<Bars>\n<Bar id=") {
        Some(pos) => pos,
        None => return bars,
    };
    let bars_section_end = match xml[bars_section_start..].find("</Bars>") {
        Some(pos) => bars_section_start + pos,
        None => return bars,
    };
    let bars_section = &xml[bars_section_start..bars_section_end];

    while let Some(bar_start) = bars_section[search_pos..].find("<Bar id=\"") {
        let abs_pos = search_pos + bar_start;
        let id_start = abs_pos + 9;
        let id_end = match bars_section[id_start..].find('"') {
            Some(pos) => id_start + pos,
            None => break,
        };
        let bar_id: usize = match bars_section[id_start..id_end].parse() {
            Ok(id) => id,
            Err(_) => {
                search_pos = id_end;
                continue;
            }
        };

        let bar_end = match bars_section[abs_pos..].find("</Bar>") {
            Some(pos) => abs_pos + pos,
            None => break,
        };
        let bar_xml = &bars_section[abs_pos..bar_end];

        let voice_ids = extract_tag_text(bar_xml, "Voices")
            .map(|text| {
                text.split_whitespace()
                    .filter_map(|id_str| id_str.parse().ok())
                    .collect()
            })
            .unwrap_or_default();

        bars.insert(bar_id, GpBar { voice_ids });
        search_pos = bar_end;
    }

    bars
}

#[derive(Debug)]
struct GpMasterBar {
    time_sig_num: u8,
    time_sig_denom: u8,
    bar_ids: Vec<usize>,
}

fn extract_master_bars(xml: &str) -> Vec<GpMasterBar> {
    let mut master_bars = Vec::new();
    let mut search_pos = 0;

    // Find <MasterBars> section
    let section_start = match xml.find("<MasterBars>") {
        Some(pos) => pos,
        None => return master_bars,
    };
    let section_end = match xml[section_start..].find("</MasterBars>") {
        Some(pos) => section_start + pos,
        None => return master_bars,
    };
    let section = &xml[section_start..section_end];

    while let Some(mb_start) = section[search_pos..].find("<MasterBar>") {
        let abs_pos = search_pos + mb_start;
        let mb_end = match section[abs_pos..].find("</MasterBar>") {
            Some(pos) => abs_pos + pos,
            None => break,
        };
        let mb_xml = &section[abs_pos..mb_end];

        let (time_sig_num, time_sig_denom) = extract_time_sig(mb_xml);
        let bar_ids = extract_tag_text(mb_xml, "Bars")
            .map(|text| {
                text.split_whitespace()
                    .filter_map(|id_str| id_str.parse().ok())
                    .collect()
            })
            .unwrap_or_default();

        master_bars.push(GpMasterBar {
            time_sig_num,
            time_sig_denom,
            bar_ids,
        });

        search_pos = mb_end;
    }

    master_bars
}

fn extract_time_sig(xml: &str) -> (u8, u8) {
    if let Some(time_text) = extract_tag_text(xml, "Time") {
        let parts: Vec<&str> = time_text.split('/').collect();
        if parts.len() == 2 {
            let num: u8 = parts[0].parse().unwrap_or(4);
            let denom: u8 = parts[1].parse().unwrap_or(4);
            return (num, denom);
        }
    }
    (4, 4)
}

fn extract_tracks(xml: &str) -> Vec<TrackInfo> {
    let mut tracks = Vec::new();
    let mut search_pos = 0;

    // Find the <Tracks> section that contains <Track id= children
    // (not the <Tracks>0</Tracks> inside MasterTrack)
    let section_start = match xml.find("<Tracks>\n<Track id=") {
        Some(pos) => pos,
        None => {
            // Fallback: find <Tracks> followed by newline
            match xml.find("<Tracks>\n") {
                Some(pos) => pos,
                None => return tracks,
            }
        }
    };
    let section_end = match xml[section_start..].find("</Tracks>") {
        Some(pos) => section_start + pos,
        None => return tracks,
    };
    let section = &xml[section_start..section_end];

    while let Some(track_start) = section[search_pos..].find("<Track id=\"") {
        let abs_pos = search_pos + track_start;
        let track_end = match section[abs_pos..].find("</Track>") {
            Some(pos) => abs_pos + pos,
            None => break,
        };
        let track_xml = &section[abs_pos..track_end];

        let name = extract_cdata_text(track_xml, "Name").unwrap_or_default();

        // Tuning: <Pitches>40 45 50 55 59 64</Pitches>
        let tuning: Vec<u8> = extract_tag_text(track_xml, "Pitches")
            .map(|text| {
                text.split_whitespace()
                    .filter_map(|pitch_str| pitch_str.parse().ok())
                    .collect()
            })
            .unwrap_or_else(|| vec![40, 45, 50, 55, 59, 64]); // standard tuning default

        let capo = extract_property_value(track_xml, "CapoFret", "Fret")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);

        let string_count = tuning.len() as u8;

        tracks.push(TrackInfo {
            name,
            tuning,
            capo,
            string_count,
            midi_channel: 0, // GP7 doesn't use MIDI channels the same way
            midi_port: 0,
        });

        search_pos = track_end;
    }

    tracks
}

fn extract_property_value<'a>(xml: &'a str, prop_name: &str, tag_name: &str) -> Option<String> {
    let search = format!("<Property name=\"{}\">", prop_name);
    let prop_start = xml.find(&search)?;
    let after = &xml[prop_start..];
    let tag_open = format!("<{}>", tag_name);
    let tag_close = format!("</{}>", tag_name);
    let start = after.find(&tag_open)? + tag_open.len();
    let end = after[start..].find(&tag_close)?;
    Some(after[start..start + end].to_string())
}

fn build_score(
    global_tempo: f64,
    rhythms: &HashMap<usize, GpRhythm>,
    gp_notes: &HashMap<usize, GpNote>,
    gp_beats: &HashMap<usize, GpBeat>,
    gp_voices: &HashMap<usize, GpVoice>,
    gp_bars: &HashMap<usize, GpBar>,
    master_bars: &[GpMasterBar],
    tracks: &[TrackInfo],
    track_index: usize,
) -> Result<TabScore> {
    let mut tab_beats = Vec::new();
    let mut tab_bars = Vec::new();
    let mut current_tick = 0.0;
    let mut beat_index = 0;

    for (bar_index, master_bar) in master_bars.iter().enumerate() {
        let first_beat_index = beat_index;

        // Get the bar for our target track
        let bar_id = master_bar
            .bar_ids
            .get(track_index)
            .copied()
            .unwrap_or(0);

        let bar = match gp_bars.get(&bar_id) {
            Some(bar) => bar,
            None => continue,
        };

        // Use the first voice (primary voice)
        if let Some(&voice_id) = bar.voice_ids.first() {
            if let Some(voice) = gp_voices.get(&voice_id) {
                for &gp_beat_id in &voice.beat_ids {
                    let gp_beat = match gp_beats.get(&gp_beat_id) {
                        Some(gp_beat) => gp_beat,
                        None => continue,
                    };

                    let rhythm = rhythms.get(&gp_beat.rhythm_id);
                    let tick_duration = rhythm
                        .map(|rhythm| {
                            duration_ticks(
                                rhythm.duration_value,
                                rhythm.is_dotted,
                                rhythm.tuplet_enters,
                                rhythm.tuplet_times,
                            )
                        })
                        .unwrap_or(TICKS_PER_QUARTER);

                    let string_count = tracks.get(track_index)
                        .map(|track| track.string_count)
                        .unwrap_or(6);

                    let notes: Vec<TabNote> = gp_beat
                        .note_ids
                        .iter()
                        .filter_map(|note_id| {
                            let gp_note = gp_notes.get(note_id)?;
                            // GP7 strings are 0-indexed (0=low E for 6-string).
                            // Convert to 1-indexed (1=high E) to match GP5 convention
                            // and the tab strip display order.
                            let display_string = string_count - gp_note.string;
                            Some(TabNote {
                                string: display_string,
                                fret: gp_note.fret,
                            })
                        })
                        .collect();

                    let is_rest = gp_beat.is_rest || notes.is_empty();

                    tab_beats.push(TabBeat {
                        bar_index,
                        beat_index,
                        tick: current_tick,
                        duration: tick_duration,
                        is_rest,
                        notes,
                    });

                    current_tick += tick_duration;
                    beat_index += 1;
                }
            }
        }

        tab_bars.push(TabBar {
            index: bar_index,
            first_beat_index,
            beat_count: beat_index - first_beat_index,
            time_sig_num: master_bar.time_sig_num,
            time_sig_denom: master_bar.time_sig_denom,
            tempo: global_tempo,
        });
    }

    Ok(TabScore {
        total_ticks: current_tick,
        beats: tab_beats,
        bars: tab_bars,
        tracks: tracks.to_vec(),
        title: String::new(),
        artist: String::new(),
    })
}

fn duration_ticks(duration_value: i8, is_dotted: bool, tuplet_enters: u8, tuplet_times: u8) -> f64 {
    let base = TICKS_PER_QUARTER * 4.0 / 2.0f64.powi(duration_value as i32 + 2);
    let dotted = if is_dotted { base * 1.5 } else { base };
    if tuplet_enters > 1 {
        dotted * tuplet_times as f64 / tuplet_enters as f64
    } else {
        dotted
    }
}

// ── XML helpers ──

fn extract_score_field(xml: &str, field: &str) -> String {
    // Find <Score> section, then extract <Field><![CDATA[...]]></Field>
    if let Some(score_start) = xml.find("<Score>") {
        if let Some(score_end) = xml[score_start..].find("</Score>") {
            let score_xml = &xml[score_start..score_start + score_end];
            return extract_cdata_text(score_xml, field).unwrap_or_default();
        }
    }
    String::new()
}

fn extract_tag_text<'a>(xml: &'a str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)?;
    Some(xml[start..start + end].trim().to_string())
}

fn extract_cdata_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)?;
    let content = xml[start..start + end].trim();
    // Strip CDATA wrapper if present
    if content.starts_with("<![CDATA[") && content.ends_with("]]>") {
        Some(content[9..content.len() - 3].to_string())
    } else {
        Some(content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct RefBeat {
        notes: Vec<RefNote>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct RefNote {
        string: u8,
        fret: u8,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct RefBar {
        time_sig_num: u8,
        time_sig_denom: u8,
        beat_count: usize,
        beats: Vec<RefBeat>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct RefScore {
        title: String,
        num_tracks: usize,
        track_name: String,
        bars: Vec<RefBar>,
    }

    fn score_to_reference(score: &TabScore) -> RefScore {
        let bars = score.bars.iter().map(|bar| {
            let beats = (0..bar.beat_count).map(|offset| {
                let beat = &score.beats[bar.first_beat_index + offset];
                RefBeat {
                    notes: beat.notes.iter().map(|note| RefNote {
                        string: note.string,
                        fret: note.fret,
                    }).collect(),
                }
            }).collect();
            RefBar {
                time_sig_num: bar.time_sig_num,
                time_sig_denom: bar.time_sig_denom,
                beat_count: bar.beat_count,
                beats,
            }
        }).collect();
        RefScore {
            title: score.title.clone(),
            num_tracks: score.tracks.len(),
            track_name: score.tracks.first()
                .map(|track| track.name.clone())
                .unwrap_or_default(),
            bars,
        }
    }

    /// Run with: cargo test generate_gp7_references -- --ignored --nocapture
    /// Generates reference JSON files from the GP7 parser output.
    /// Only needs to be re-run if test fixtures change.
    #[test]
    #[ignore]
    fn generate_gp7_references() {
        for file_num in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 15] {
            let gp_path = fixture_path(&format!("test_file_{}.gp", file_num));
            let (score, _) = parse_file(&gp_path)
                .unwrap_or_else(|error| panic!("Failed to parse file {}: {}", file_num, error));
            let reference = score_to_reference(&score);
            let json = serde_json::to_string_pretty(&reference).unwrap();
            let ref_path = fixture_path(&format!("gp7_reference_{}.json", file_num));
            std::fs::write(&ref_path, json).unwrap();
            println!("Generated {}: {} bars, {} tracks",
                ref_path.display(), reference.bars.len(), reference.num_tracks);
        }
    }

    fn assert_matches_reference(gp_file: &str, reference_file: &str) {
        let path = fixture_path(gp_file);
        let (score, _) = parse_file(&path)
            .expect("GP7 parse should succeed");

        let ref_json = std::fs::read_to_string(fixture_path(reference_file))
            .expect("reference JSON");
        let reference: RefScore = serde_json::from_str(&ref_json)
            .expect("parse reference JSON");

        assert_eq!(score.tracks.len(), reference.num_tracks, "track count");
        assert_eq!(score.tracks[0].name, reference.track_name, "track name");
        assert_eq!(score.bars.len(), reference.bars.len(), "bar count");

        for (bar_idx, (bar, ref_bar)) in score.bars.iter().zip(reference.bars.iter()).enumerate() {
            assert_eq!(bar.time_sig_num, ref_bar.time_sig_num,
                "bar {} time_sig_num", bar_idx);
            assert_eq!(bar.time_sig_denom, ref_bar.time_sig_denom,
                "bar {} time_sig_denom", bar_idx);
            assert_eq!(bar.beat_count, ref_bar.beat_count,
                "bar {} beat_count", bar_idx);

            for beat_offset in 0..bar.beat_count {
                let beat = &score.beats[bar.first_beat_index + beat_offset];
                let ref_beat = &ref_bar.beats[beat_offset];

                assert_eq!(beat.notes.len(), ref_beat.notes.len(),
                    "bar {} beat {} note count", bar_idx, beat_offset);

                for (note, ref_note) in beat.notes.iter().zip(ref_beat.notes.iter()) {
                    assert_eq!(note.string, ref_note.string,
                        "bar {} beat {} note string", bar_idx, beat_offset);
                    assert_eq!(note.fret, ref_note.fret,
                        "bar {} beat {} note fret", bar_idx, beat_offset);
                }
            }
        }
    }

    #[test]
    fn test_parse_gp7_file_01() {
        assert_matches_reference("test_file_1.gp", "gp7_reference_1.json");
    }

    #[test]
    fn test_parse_gp7_file_02() {
        assert_matches_reference("test_file_2.gp", "gp7_reference_2.json");
    }

    #[test]
    fn test_parse_gp7_file_03() {
        assert_matches_reference("test_file_3.gp", "gp7_reference_3.json");
    }

    #[test]
    fn test_parse_gp7_file_04() {
        assert_matches_reference("test_file_4.gp", "gp7_reference_4.json");
    }

    #[test]
    fn test_parse_gp7_file_05() {
        assert_matches_reference("test_file_5.gp", "gp7_reference_5.json");
    }

    #[test]
    fn test_parse_gp7_file_06() {
        assert_matches_reference("test_file_6.gp", "gp7_reference_6.json");
    }

    #[test]
    fn test_parse_gp7_file_07() {
        assert_matches_reference("test_file_7.gp", "gp7_reference_7.json");
    }

    #[test]
    fn test_parse_gp7_file_08() {
        assert_matches_reference("test_file_8.gp", "gp7_reference_8.json");
    }

    #[test]
    fn test_parse_gp7_file_09() {
        assert_matches_reference("test_file_9.gp", "gp7_reference_9.json");
    }

    #[test]
    fn test_parse_gp7_file_10() {
        assert_matches_reference("test_file_10.gp", "gp7_reference_10.json");
    }

    #[test]
    fn test_parse_gp7_file_11() {
        assert_matches_reference("test_file_11.gp", "gp7_reference_11.json");
    }

    #[test]
    fn test_parse_gp7_file_12() {
        assert_matches_reference("test_file_12.gp", "gp7_reference_12.json");
    }

    #[test]
    fn test_parse_gp7_file_15() {
        assert_matches_reference("test_file_15.gp", "gp7_reference_15.json");
    }
}

