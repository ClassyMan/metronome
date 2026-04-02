/// Converts a TabScore into a sample-accurate MidiTimeline for FluidSynth rendering.
///
/// The timeline is a flat list of MIDI events sorted by sample position,
/// plus beat markers for driving UI updates (cursor, fretboard highlights).

use crate::tab_models::*;

pub const SAMPLE_RATE: f64 = 44100.0;

const METRONOME_CHANNEL: u8 = 9;
const GUITAR_CHANNEL: u8 = 0;
const METRONOME_ACCENT_NOTE: u8 = 75; // claves (downbeat)
const METRONOME_REGULAR_NOTE: u8 = 37; // side stick
const METRONOME_ACCENT_VELOCITY: u8 = 100;
const METRONOME_REGULAR_VELOCITY: u8 = 80;
const METRONOME_DURATION_TICKS: f64 = 48.0; // short click

#[derive(Debug, Clone)]
pub enum MidiEvent {
    NoteOn {
        sample_position: u64,
        channel: u8,
        key: u8,
        velocity: u8,
    },
    NoteOff {
        sample_position: u64,
        channel: u8,
        key: u8,
    },
}

impl MidiEvent {
    pub fn sample_position(&self) -> u64 {
        match self {
            MidiEvent::NoteOn { sample_position, .. } => *sample_position,
            MidiEvent::NoteOff { sample_position, .. } => *sample_position,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BeatMarker {
    pub sample_position: u64,
    pub beat_index: usize,
}

#[derive(Debug, Clone)]
pub struct MidiTimeline {
    pub events: Vec<MidiEvent>,
    pub beat_markers: Vec<BeatMarker>,
    pub total_samples: u64,
}

pub fn build_timeline(
    score: &TabScore,
    track_index: usize,
    tempo_percent: f64,
    include_metronome: bool,
) -> MidiTimeline {
    let track = match score.tracks.get(track_index) {
        Some(track) => track,
        None => {
            log::error!(
                "Track index {} out of range ({})",
                track_index,
                score.tracks.len()
            );
            return MidiTimeline {
                events: Vec::new(),
                beat_markers: Vec::new(),
                total_samples: 0,
            };
        }
    };
    let mut events = Vec::new();
    let mut beat_markers = Vec::new();

    for bar in &score.bars {
        let effective_bpm = bar.tempo * tempo_percent / 100.0;
        let _samples_per_tick = (SAMPLE_RATE * 60.0) / (effective_bpm * TICKS_PER_QUARTER);

        // Add metronome events for this bar
        if include_metronome {
            let ticks_per_bar_beat =
                TICKS_PER_QUARTER * 4.0 / bar.time_sig_denom as f64;

            for metronome_beat in 0..bar.time_sig_num {
                let first_beat = &score.beats[bar.first_beat_index];
                let beat_tick =
                    first_beat.tick + metronome_beat as f64 * ticks_per_bar_beat;
                let sample_pos = tick_to_sample(beat_tick, score, tempo_percent);

                let (note, velocity) = if metronome_beat == 0 {
                    (METRONOME_ACCENT_NOTE, METRONOME_ACCENT_VELOCITY)
                } else {
                    (METRONOME_REGULAR_NOTE, METRONOME_REGULAR_VELOCITY)
                };

                events.push(MidiEvent::NoteOn {
                    sample_position: sample_pos,
                    channel: METRONOME_CHANNEL,
                    key: note,
                    velocity,
                });

                let off_tick = beat_tick + METRONOME_DURATION_TICKS;
                let off_sample = tick_to_sample(off_tick, score, tempo_percent);
                events.push(MidiEvent::NoteOff {
                    sample_position: off_sample,
                    channel: METRONOME_CHANNEL,
                    key: note,
                });
            }
        }

        // Add guitar events + beat markers for each beat in this bar
        for beat_offset in 0..bar.beat_count {
            let beat = &score.beats[bar.first_beat_index + beat_offset];
            let sample_pos = tick_to_sample(beat.tick, score, tempo_percent);

            beat_markers.push(BeatMarker {
                sample_position: sample_pos,
                beat_index: beat.beat_index,
            });

            if beat.is_rest {
                continue;
            }

            for note in &beat.notes {
                let midi_key = compute_midi_key(note, track);
                if midi_key > 127 {
                    continue;
                }

                events.push(MidiEvent::NoteOn {
                    sample_position: sample_pos,
                    channel: GUITAR_CHANNEL,
                    key: midi_key,
                    velocity: 80,
                });

                let off_tick = beat.tick + beat.duration;
                let off_sample = tick_to_sample(off_tick, score, tempo_percent);
                events.push(MidiEvent::NoteOff {
                    sample_position: off_sample,
                    channel: GUITAR_CHANNEL,
                    key: midi_key,
                });
            }
        }
    }

    events.sort_by_key(|event| event.sample_position());
    beat_markers.sort_by_key(|marker| marker.sample_position);

    let total_samples = if let Some(last_bar) = score.bars.last() {
        let last_beat_idx = last_bar.first_beat_index + last_bar.beat_count.saturating_sub(1);
        if let Some(last_beat) = score.beats.get(last_beat_idx) {
            tick_to_sample(last_beat.tick + last_beat.duration, score, tempo_percent)
        } else {
            0
        }
    } else {
        0
    };

    MidiTimeline {
        events,
        beat_markers,
        total_samples,
    }
}

fn compute_midi_key(note: &TabNote, track: &TrackInfo) -> u8 {
    // String numbering: 1 = high E, 6 = low E (after GP7 normalization)
    // Tuning array: index 0 = low E, index 5 = high E (GP7 order, low-to-high)
    // Mapping: string 1 (high E) → tuning[5], string 6 (low E) → tuning[0]
    let tuning_index = track.tuning.len().saturating_sub(note.string as usize);
    if tuning_index >= track.tuning.len() {
        return 0;
    }
    let open_string_midi = track.tuning[tuning_index];
    open_string_midi.saturating_add(note.fret).saturating_add(track.capo)
}

fn tick_to_sample(tick: f64, score: &TabScore, tempo_percent: f64) -> u64 {
    let mut sample_pos: f64 = 0.0;
    let mut prev_tick: f64 = 0.0;
    let mut current_tempo = score.bars.first().map(|bar| bar.tempo).unwrap_or(120.0);

    for bar in &score.bars {
        let bar_start_tick = if bar.first_beat_index < score.beats.len() {
            score.beats[bar.first_beat_index].tick
        } else {
            break;
        };

        if bar_start_tick > tick {
            break;
        }

        if (bar.tempo - current_tempo).abs() > 0.01 {
            let effective_bpm = current_tempo * tempo_percent / 100.0;
            let samples_per_tick =
                (SAMPLE_RATE * 60.0) / (effective_bpm * TICKS_PER_QUARTER);
            sample_pos += (bar_start_tick - prev_tick) * samples_per_tick;
            prev_tick = bar_start_tick;
            current_tempo = bar.tempo;
        }
    }

    let effective_bpm = current_tempo * tempo_percent / 100.0;
    let samples_per_tick = (SAMPLE_RATE * 60.0) / (effective_bpm * TICKS_PER_QUARTER);
    sample_pos += (tick - prev_tick) * samples_per_tick;

    sample_pos as u64
}
