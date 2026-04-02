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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gp7_parser;

    const BUFFER_FRAMES: u64 = 256;

    // ── Engine simulator ────────────────────────────────────────────────
    // Mirrors run_audio_loop state machine without FluidSynth / GStreamer.

    enum Cmd {
        Play,
        Pause,
        SetTimeline(MidiTimeline),
        SeekToBeat(usize),
        SetLoop(Option<(usize, usize)>),
    }

    struct Engine {
        timeline: Option<MidiTimeline>,
        is_playing: bool,
        sample_position: u64,
        event_index: usize,
        beat_marker_index: usize,
        loop_range: Option<(u64, u64)>,
        loop_beat_range: Option<(usize, usize)>,
        fired_beats: Vec<usize>,
    }

    impl Engine {
        fn new() -> Self {
            Self {
                timeline: None,
                is_playing: false,
                sample_position: 0,
                event_index: 0,
                beat_marker_index: 0,
                loop_range: None,
                loop_beat_range: None,
                fired_beats: Vec::new(),
            }
        }

        fn command(&mut self, cmd: Cmd) {
            match cmd {
                Cmd::Play => {
                    if self.timeline.is_some() {
                        self.is_playing = true;
                    }
                }
                Cmd::Pause => {
                    self.is_playing = false;
                }
                Cmd::SetTimeline(tl) => {
                    self.sample_position = 0;
                    self.event_index = 0;
                    self.beat_marker_index = 0;
                    self.timeline = Some(tl);
                    self.recompute_loop_samples();
                }
                Cmd::SeekToBeat(beat_index) => {
                    if let Some(ref tl) = self.timeline {
                        if let Some(marker) = tl
                            .beat_markers
                            .iter()
                            .find(|m| m.beat_index == beat_index)
                        {
                            self.sample_position = marker.sample_position;
                            self.rewind_indices();
                        }
                    }
                }
                Cmd::SetLoop(beat_range) => {
                    self.loop_beat_range = beat_range;
                    self.recompute_loop_samples();
                }
            }
        }

        /// Advance one buffer. Returns false when playback stopped (end-of-song).
        fn tick(&mut self) -> bool {
            if !self.is_playing {
                return false;
            }
            let tl = match self.timeline.as_ref() {
                Some(tl) => tl,
                None => return false,
            };

            let raw_buffer_end = self.sample_position + BUFFER_FRAMES;
            let boundary = self
                .loop_range
                .map(|(_, end)| end)
                .unwrap_or(tl.total_samples);
            let buffer_end = raw_buffer_end.min(boundary);

            // Skip MIDI event dispatch (no synth) but advance the index
            while self.event_index < tl.events.len() {
                if tl.events[self.event_index].sample_position() >= buffer_end {
                    break;
                }
                self.event_index += 1;
            }

            // Fire beat callbacks
            while self.beat_marker_index < tl.beat_markers.len() {
                let marker = &tl.beat_markers[self.beat_marker_index];
                if marker.sample_position >= buffer_end {
                    break;
                }
                if marker.sample_position >= self.sample_position {
                    self.fired_beats.push(marker.beat_index);
                }
                self.beat_marker_index += 1;
            }

            self.sample_position += BUFFER_FRAMES;

            // Loop / end-of-song
            if self.sample_position >= boundary {
                if self.loop_range.is_some() {
                    let start = self.loop_range.unwrap().0;
                    self.sample_position = start;
                    self.rewind_indices();
                } else {
                    self.is_playing = false;
                    self.sample_position = 0;
                    self.event_index = 0;
                    self.beat_marker_index = 0;
                }
            }

            true
        }

        fn run(&mut self, iterations: usize) {
            for _ in 0..iterations {
                if !self.tick() {
                    break;
                }
            }
        }

        fn take_fired(&mut self) -> Vec<usize> {
            std::mem::take(&mut self.fired_beats)
        }

        fn rewind_indices(&mut self) {
            if let Some(ref tl) = self.timeline {
                self.event_index = tl
                    .events
                    .partition_point(|e| e.sample_position() < self.sample_position);
                self.beat_marker_index = tl
                    .beat_markers
                    .partition_point(|m| m.sample_position < self.sample_position);
            }
        }

        fn recompute_loop_samples(&mut self) {
            match (&self.timeline, &self.loop_beat_range) {
                (Some(tl), Some((start_beat, end_beat))) => {
                    let start_sample = tl
                        .beat_markers
                        .iter()
                        .find(|m| m.beat_index == *start_beat)
                        .map(|m| m.sample_position)
                        .unwrap_or(0);
                    let end_sample = tl
                        .beat_markers
                        .iter()
                        .find(|m| m.beat_index == *end_beat + 1)
                        .map(|m| m.sample_position)
                        .unwrap_or(tl.total_samples);
                    self.loop_range = Some((start_sample, end_sample));
                }
                _ => {
                    self.loop_range = None;
                }
            }
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    fn load_all_hammers() -> TabScore {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/test_file_1.gp");
        let (score, _) = gp7_parser::parse_file(&path).expect("parse All Hammers");
        score
    }

    fn hammers_engine() -> Engine {
        let score = load_all_hammers();
        let tl = build_timeline(&score, 0, 100.0, false);
        let mut engine = Engine::new();
        engine.command(Cmd::SetTimeline(tl));
        engine
    }

    fn assert_beats_in_range(beats: &[usize], lo: usize, hi: usize) {
        for &beat_index in beats {
            assert!(
                beat_index >= lo && beat_index <= hi,
                "beat {} outside [{}, {}]",
                beat_index,
                lo,
                hi,
            );
        }
    }

    // ── Timeline basics ────────────────────────────────────────────────

    #[test]
    fn test_timeline_markers_sorted_and_in_range() {
        let score = load_all_hammers();
        let tl = build_timeline(&score, 0, 100.0, false);

        assert_eq!(tl.beat_markers.len(), 184);
        assert_eq!(tl.beat_markers[0].sample_position, 0);
        assert!(tl.total_samples > 0);

        for window in tl.beat_markers.windows(2) {
            assert!(window[0].sample_position <= window[1].sample_position);
        }
        for marker in &tl.beat_markers {
            assert!(marker.beat_index < score.beats.len());
        }
    }

    #[test]
    fn test_timeline_tempo_scaling() {
        let score = load_all_hammers();
        let tl_100 = build_timeline(&score, 0, 100.0, false);
        let tl_50 = build_timeline(&score, 0, 50.0, false);
        let tl_200 = build_timeline(&score, 0, 200.0, false);

        // Half tempo → double duration; double tempo → half duration
        let ratio_slow = tl_50.total_samples as f64 / tl_100.total_samples as f64;
        let ratio_fast = tl_200.total_samples as f64 / tl_100.total_samples as f64;
        assert!((ratio_slow - 2.0).abs() < 0.01, "50% should be ~2x: {}", ratio_slow);
        assert!((ratio_fast - 0.5).abs() < 0.01, "200% should be ~0.5x: {}", ratio_fast);
    }

    #[test]
    fn test_beat_index_matches_score_position() {
        let score = load_all_hammers();
        let tl = build_timeline(&score, 0, 100.0, false);

        for marker in &tl.beat_markers {
            let beat = &score.beats[marker.beat_index];
            assert!(beat.bar_index < score.bars.len());
            assert!(!beat.notes.is_empty(), "beat {} has no notes", marker.beat_index);
        }
    }

    // ── Full playback ──────────────────────────────────────────────────

    #[test]
    fn test_full_playback_fires_all_beats_in_order() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(1_000_000);

        let fired = engine.take_fired();
        assert_eq!(fired.len(), 184);
        for (position, beat_index) in fired.iter().enumerate() {
            assert_eq!(*beat_index, position);
        }
        assert!(!engine.is_playing, "should stop at end-of-song");
    }

    #[test]
    fn test_end_of_song_resets_position() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(1_000_000);

        assert_eq!(engine.sample_position, 0);
        assert_eq!(engine.beat_marker_index, 0);
        assert_eq!(engine.event_index, 0);
    }

    // ── Pause / resume ─────────────────────────────────────────────────

    #[test]
    fn test_pause_preserves_position() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(100);
        let pos_before = engine.sample_position;
        let beats_before = engine.take_fired().len();
        assert!(beats_before > 0);

        engine.command(Cmd::Pause);
        assert!(!engine.is_playing);

        // Tick while paused — nothing happens
        engine.run(100);
        assert_eq!(engine.sample_position, pos_before);
        assert!(engine.take_fired().is_empty());

        // Resume — continues from same position
        engine.command(Cmd::Play);
        engine.run(100);
        let beats_after = engine.take_fired();
        assert!(!beats_after.is_empty());
        // First beat after resume should follow last beat before pause
        assert!(beats_after[0] > 0);
    }

    #[test]
    fn test_pause_resume_no_duplicate_beats() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(50);
        let first_batch = engine.take_fired();

        engine.command(Cmd::Pause);
        engine.command(Cmd::Play);
        engine.run(1_000_000);
        let second_batch = engine.take_fired();

        // Combined should be all 184 beats in order, no duplicates
        let mut all: Vec<usize> = first_batch;
        all.extend(second_batch);
        assert_eq!(all.len(), 184);
        for (position, beat_index) in all.iter().enumerate() {
            assert_eq!(*beat_index, position);
        }
    }

    // ── Seek ───────────────────────────────────────────────────────────

    #[test]
    fn test_seek_forward() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.command(Cmd::SeekToBeat(100));
        engine.run(1_000_000);

        let fired = engine.take_fired();
        assert_eq!(fired[0], 100, "first beat after seek should be 100");
        assert_eq!(*fired.last().unwrap(), 183);
    }

    #[test]
    fn test_seek_to_start() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(200);
        engine.take_fired();

        engine.command(Cmd::SeekToBeat(0));
        engine.run(1_000_000);

        let fired = engine.take_fired();
        assert_eq!(fired[0], 0);
        assert_eq!(*fired.last().unwrap(), 183);
    }

    #[test]
    fn test_seek_to_last_beat() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.command(Cmd::SeekToBeat(183));
        engine.run(1_000_000);

        let fired = engine.take_fired();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0], 183);
    }

    #[test]
    fn test_seek_to_nonexistent_beat_is_noop() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(50);
        let pos = engine.sample_position;

        engine.command(Cmd::SeekToBeat(9999));
        assert_eq!(engine.sample_position, pos, "invalid seek should not move");
    }

    #[test]
    fn test_seek_backward_during_playback() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(500);
        let first_pass = engine.take_fired();
        let last_beat = *first_pass.last().unwrap();

        engine.command(Cmd::SeekToBeat(0));
        engine.run(1_000_000);
        let second_pass = engine.take_fired();

        assert_eq!(second_pass[0], 0);
        assert_eq!(*second_pass.last().unwrap(), 183);
        // Beats from both passes — some played twice (that's OK for seek)
        assert!(last_beat > 0);
    }

    // ── Loops ──────────────────────────────────────────────────────────

    #[test]
    fn test_loop_first_bar() {
        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((0, 15))));
        engine.command(Cmd::Play);
        engine.run(200_000);

        let fired = engine.take_fired();
        assert!(fired.len() >= 48, "expected at least 3 cycles of 16 beats");
        assert_beats_in_range(&fired, 0, 15);

        // Verify sequential ordering within each cycle
        for chunk in fired.chunks(16) {
            if chunk.len() < 16 {
                break;
            }
            for (offset, beat_index) in chunk.iter().enumerate() {
                assert_eq!(*beat_index, offset);
            }
        }
    }

    #[test]
    fn test_loop_middle_bars() {
        let mut engine = hammers_engine();
        // Bars 4-5 = beats 64..95
        engine.command(Cmd::SetLoop(Some((64, 95))));
        engine.command(Cmd::Play);
        engine.run(500_000);

        let fired = engine.take_fired();
        // First 64 beats play through, then loop starts
        let loop_beats: Vec<_> = fired.iter().filter(|&&b| b >= 64 && b <= 95).copied().collect();
        assert!(loop_beats.len() >= 64, "expected several loop cycles");

        // Beats 0..63 should appear exactly once (initial play-through)
        let pre_loop: Vec<_> = fired.iter().filter(|&&b| b < 64).copied().collect();
        assert_eq!(pre_loop.len(), 64);
        for (position, beat_index) in pre_loop.iter().enumerate() {
            assert_eq!(*beat_index, position);
        }

        // No beats past 95 should ever fire
        assert!(fired.iter().all(|&b| b <= 95), "beat past loop end fired");
    }

    #[test]
    fn test_loop_last_bar() {
        let mut engine = hammers_engine();
        // Last bar = bar 11, beats 176..183 (8 beats in last bar)
        engine.command(Cmd::SetLoop(Some((176, 183))));
        engine.command(Cmd::Play);
        engine.run(1_000_000);

        let fired = engine.take_fired();
        // Beats 0..183 play through initially, then loop 176..183
        let loop_beats: Vec<_> = fired.iter().filter(|&&b| b >= 176).copied().collect();
        assert!(loop_beats.len() >= 16, "expected at least 2 cycles of last bar");
        assert!(fired.iter().all(|&b| b <= 183), "beat index out of range");
    }

    #[test]
    fn test_loop_single_beat() {
        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((42, 42))));
        engine.command(Cmd::Play);
        engine.run(100_000);

        let fired = engine.take_fired();
        // Beats 0..42 play through, then only beat 42 repeats
        let repeats: Vec<_> = fired.iter().filter(|&&b| b == 42).copied().collect();
        assert!(repeats.len() >= 3, "single-beat loop should repeat, got {}", repeats.len());
        // No beat past 42 should fire
        assert!(fired.iter().all(|&b| b <= 42));
    }

    #[test]
    fn test_loop_cleared_continues_playback() {
        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((0, 15))));
        engine.command(Cmd::Play);
        engine.run(10_000);
        engine.take_fired();

        // Clear loop while playing
        engine.command(Cmd::SetLoop(None));
        engine.run(1_000_000);
        let fired = engine.take_fired();

        // After clearing, playback should continue to the end
        assert!(*fired.last().unwrap() == 183, "should reach end after loop cleared");
    }

    #[test]
    fn test_loop_set_while_past_range() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        // Play past beat 100
        engine.run(5000);
        let pre = engine.take_fired();
        assert!(*pre.last().unwrap() > 50, "should be well past beat 50");

        // Set loop on first bar — position is already past the range
        engine.command(Cmd::SetLoop(Some((0, 15))));
        engine.run(200_000);
        let fired = engine.take_fired();

        // Should rewind and loop within 0..15
        assert!(!fired.is_empty());
        assert_beats_in_range(&fired, 0, 15);
    }

    #[test]
    fn test_loop_change_mid_loop() {
        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((0, 15))));
        engine.command(Cmd::Play);
        engine.run(10_000);
        engine.take_fired();

        // Switch to a different loop range
        engine.command(Cmd::SetLoop(Some((32, 47))));
        engine.run(200_000);
        let fired = engine.take_fired();

        // Should now be looping beats 32..47
        let in_new_range: Vec<_> = fired.iter().filter(|&&b| b >= 32 && b <= 47).copied().collect();
        assert!(in_new_range.len() >= 32, "should loop several cycles in new range");
        // No beats past 47 (after transition settles)
        let tail: Vec<_> = fired.iter().skip(fired.len().saturating_sub(32)).copied().collect();
        assert_beats_in_range(&tail, 32, 47);
    }

    // ── End-of-song → loop → play again ────────────────────────────────

    #[test]
    fn test_play_to_end_then_loop_then_play() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(1_000_000);
        assert!(!engine.is_playing);
        engine.take_fired();

        // Set loop on bar 0, play again
        engine.command(Cmd::SetLoop(Some((0, 15))));
        engine.command(Cmd::Play);
        engine.run(100_000);

        let fired = engine.take_fired();
        assert!(!fired.is_empty(), "should fire beats after restart");
        assert_beats_in_range(&fired, 0, 15);
    }

    #[test]
    fn test_play_to_end_then_play_again_no_loop() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(1_000_000);
        assert!(!engine.is_playing);
        let first = engine.take_fired();
        assert_eq!(first.len(), 184);

        // Play again from the top
        engine.command(Cmd::Play);
        engine.run(1_000_000);
        let second = engine.take_fired();
        assert_eq!(second.len(), 184);
        assert_eq!(second[0], 0);
    }

    // ── Seek + loop combinations ───────────────────────────────────────

    #[test]
    fn test_seek_inside_loop_range() {
        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((0, 31))));
        engine.command(Cmd::Play);
        engine.run(1000);
        engine.take_fired();

        engine.command(Cmd::SeekToBeat(16));
        engine.run(100_000);
        let fired = engine.take_fired();

        assert_eq!(fired[0], 16, "should resume from seek point");
        assert_beats_in_range(&fired, 0, 31);
    }

    #[test]
    fn test_seek_outside_loop_range() {
        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((0, 15))));
        engine.command(Cmd::Play);
        engine.run(1000);
        engine.take_fired();

        // Seek past loop end
        engine.command(Cmd::SeekToBeat(100));
        engine.run(100_000);
        let fired = engine.take_fired();

        // Engine should still respect loop — it will play from 100, hit the
        // loop boundary clamp, and rewind. The first beat fired is 100, then
        // it wraps back into 0..15.
        assert!(!fired.is_empty());
    }

    // ── Timeline swap ──────────────────────────────────────────────────

    #[test]
    fn test_timeline_swap_resets_state() {
        let score = load_all_hammers();

        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((0, 15))));
        engine.command(Cmd::Play);
        engine.run(5000);
        engine.take_fired();

        // Swap to a new timeline (simulates track switch)
        let new_tl = build_timeline(&score, 0, 75.0, false);
        engine.command(Cmd::SetTimeline(new_tl));
        engine.run(1_000_000);
        let fired = engine.take_fired();

        // After timeline swap, position resets to 0.
        // Loop was set on the OLD timeline's beat range but recomputed
        // on the new timeline. Should still work.
        assert_eq!(fired[0], 0);
        assert_beats_in_range(&fired, 0, 15);
    }

    #[test]
    fn test_timeline_swap_clears_stale_indices() {
        let score = load_all_hammers();

        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(5000);
        engine.take_fired();

        // The engine has advanced event_index and beat_marker_index.
        // A timeline swap must reset them.
        let new_tl = build_timeline(&score, 0, 100.0, true); // with metronome
        engine.command(Cmd::SetTimeline(new_tl));

        assert_eq!(engine.sample_position, 0);
        assert_eq!(engine.event_index, 0);
        assert_eq!(engine.beat_marker_index, 0);
    }

    // ── Negative / edge cases ──────────────────────────────────────────

    #[test]
    fn test_play_without_timeline_is_noop() {
        let mut engine = Engine::new();
        engine.command(Cmd::Play);
        assert!(!engine.is_playing);
        engine.run(100);
        assert!(engine.take_fired().is_empty());
    }

    #[test]
    fn test_pause_without_play_is_safe() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Pause);
        assert!(!engine.is_playing);
        engine.run(100);
        assert!(engine.take_fired().is_empty());
    }

    #[test]
    fn test_seek_without_timeline_is_safe() {
        let mut engine = Engine::new();
        engine.command(Cmd::SeekToBeat(42));
        assert_eq!(engine.sample_position, 0);
    }

    #[test]
    fn test_set_loop_without_timeline_is_safe() {
        let mut engine = Engine::new();
        engine.command(Cmd::SetLoop(Some((0, 15))));
        assert!(engine.loop_range.is_none());
    }

    #[test]
    fn test_loop_with_invalid_beat_range() {
        let mut engine = hammers_engine();
        // end_beat+1 = 9999 doesn't exist → falls back to total_samples
        engine.command(Cmd::SetLoop(Some((0, 9999))));
        engine.command(Cmd::Play);
        engine.run(1_000_000);

        let fired = engine.take_fired();
        // Should play all beats and loop back to 0
        assert!(fired.len() > 184, "should loop at least once");
        // First 184 should be 0..183
        for (position, beat_index) in fired.iter().take(184).enumerate() {
            assert_eq!(*beat_index, position);
        }
    }

    #[test]
    fn test_double_play_is_idempotent() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.command(Cmd::Play);
        engine.run(1_000_000);

        let fired = engine.take_fired();
        assert_eq!(fired.len(), 184, "double play shouldn't duplicate beats");
    }

    #[test]
    fn test_rapid_seek_during_playback() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        for target in [50, 0, 100, 30, 183, 0] {
            engine.run(20);
            engine.command(Cmd::SeekToBeat(target));
        }
        engine.run(1_000_000);

        // Should eventually reach end from the last seek position (0)
        let fired = engine.take_fired();
        assert!(!fired.is_empty());
        assert_eq!(*fired.last().unwrap(), 183);
    }

    #[test]
    fn test_rapid_loop_changes() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);

        for &(start, end) in &[(0, 15), (32, 47), (64, 79), (0, 31)] {
            engine.command(Cmd::SetLoop(Some((start, end))));
            engine.run(5000);
            engine.take_fired();
        }

        // Final state: looping 0..31
        engine.run(100_000);
        let fired = engine.take_fired();
        assert_beats_in_range(&fired, 0, 31);
    }

    #[test]
    fn test_metronome_events_do_not_affect_beat_markers() {
        let score = load_all_hammers();
        let tl_no_met = build_timeline(&score, 0, 100.0, false);
        let tl_met = build_timeline(&score, 0, 100.0, true);

        assert_eq!(tl_no_met.beat_markers.len(), tl_met.beat_markers.len());
        for (without, with) in tl_no_met.beat_markers.iter().zip(tl_met.beat_markers.iter()) {
            assert_eq!(without.beat_index, with.beat_index);
            assert_eq!(without.sample_position, with.sample_position);
        }
        // Metronome adds extra MIDI events
        assert!(tl_met.events.len() > tl_no_met.events.len());
    }

    // ── Strengthened edge cases ────────────────────────────────────────

    #[test]
    fn test_seek_outside_loop_does_not_fire_out_of_range() {
        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((0, 15))));
        engine.command(Cmd::Play);
        engine.run(1000);
        engine.take_fired();

        // Seek well past loop end
        engine.command(Cmd::SeekToBeat(100));
        engine.run(100_000);
        let fired = engine.take_fired();

        // Beat 100 should never fire — position is past the clamped boundary,
        // so the engine rewinds before any callbacks fire.
        assert!(!fired.contains(&100), "beat 100 should not fire inside loop [0,15]");
        assert_beats_in_range(&fired, 0, 15);
    }

    #[test]
    fn test_seek_while_paused_then_play() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(200);
        engine.command(Cmd::Pause);
        engine.take_fired();

        engine.command(Cmd::SeekToBeat(80));
        assert!(!engine.is_playing);

        engine.command(Cmd::Play);
        engine.run(1_000_000);
        let fired = engine.take_fired();
        assert_eq!(fired[0], 80);
        assert_eq!(*fired.last().unwrap(), 183);
    }

    #[test]
    fn test_loop_set_while_paused_then_play() {
        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((32, 47))));
        // Don't play yet — just set up the loop while paused
        assert!(!engine.is_playing);

        engine.command(Cmd::Play);
        engine.run(200_000);
        let fired = engine.take_fired();

        // Should play 0..47 then loop 32..47
        let tail: Vec<_> = fired.iter().skip(fired.len().saturating_sub(48)).copied().collect();
        assert_beats_in_range(&tail, 32, 47);
    }

    #[test]
    fn test_no_beats_fire_after_end_of_song() {
        let mut engine = hammers_engine();
        engine.command(Cmd::Play);
        engine.run(1_000_000);
        let first = engine.take_fired();
        assert_eq!(first.len(), 184);

        // Engine is stopped. Running more ticks should produce nothing.
        engine.run(10_000);
        assert!(engine.take_fired().is_empty());
    }

    #[test]
    fn test_loop_survives_pause_resume() {
        let mut engine = hammers_engine();
        engine.command(Cmd::SetLoop(Some((0, 15))));
        engine.command(Cmd::Play);
        engine.run(10_000);
        engine.take_fired();

        engine.command(Cmd::Pause);
        engine.run(100);
        engine.command(Cmd::Play);
        engine.run(100_000);
        let fired = engine.take_fired();

        // Loop should still be active
        assert_beats_in_range(&fired, 0, 15);
    }

    // ── Assembly tests: GP5 ────────────────────────────────────────────
    // Full pipeline: parse → timeline → engine → on_beat verification

    fn load_gp5(fixture: &str) -> TabScore {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture);
        let (score, _) = crate::gp5_parser::parse_file(&path)
            .unwrap_or_else(|error| panic!("parse {}: {}", fixture, error));
        score
    }

    fn load_gp7(fixture: &str) -> TabScore {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture);
        let (score, _) = crate::gp7_parser::parse_file(&path)
            .unwrap_or_else(|error| panic!("parse {}: {}", fixture, error));
        score
    }

    /// Assembly test: parse, build timeline, simulate full playback, verify
    /// every fired beat_index resolves to a valid score entry with correct
    /// bar_index — the same lookup on_beat does on the main thread.
    fn assert_full_pipeline(score: &TabScore, label: &str) {
        let tl = build_timeline(score, 0, 100.0, false);

        assert!(
            !tl.beat_markers.is_empty(),
            "{}: no beat markers in timeline",
            label,
        );
        assert_eq!(
            tl.beat_markers.len(),
            score.beats.len(),
            "{}: marker count != beat count",
            label,
        );

        // Simulate playback
        let mut engine = Engine::new();
        engine.command(Cmd::SetTimeline(tl));
        engine.command(Cmd::Play);
        engine.run(5_000_000);
        let fired = engine.take_fired();

        assert_eq!(
            fired.len(),
            score.beats.len(),
            "{}: fired {} beats, expected {}",
            label,
            fired.len(),
            score.beats.len(),
        );

        // Verify on_beat lookup for every fired beat
        for &beat_index in &fired {
            let beat = score.beats.get(beat_index);
            assert!(
                beat.is_some(),
                "{}: score.beats[{}] is None (len={})",
                label,
                beat_index,
                score.beats.len(),
            );
            let beat = beat.unwrap();
            assert!(
                beat.bar_index < score.bars.len(),
                "{}: beat {} has bar_index {} but only {} bars",
                label,
                beat_index,
                beat.bar_index,
                score.bars.len(),
            );
            // Verify the bar's first_beat_index is consistent
            let bar = &score.bars[beat.bar_index];
            assert!(
                beat_index >= bar.first_beat_index
                    && beat_index < bar.first_beat_index + bar.beat_count,
                "{}: beat {} claims bar {} but bar range is [{}, {})",
                label,
                beat_index,
                beat.bar_index,
                bar.first_beat_index,
                bar.first_beat_index + bar.beat_count,
            );
        }

        // Verify sequential order
        for (position, &beat_index) in fired.iter().enumerate() {
            assert_eq!(
                beat_index, position,
                "{}: beat at position {} has index {}",
                label, position, beat_index,
            );
        }

        assert!(!engine.is_playing, "{}: should stop at end-of-song", label);
    }

    /// Assembly test for looping: parse, build timeline, loop first bar,
    /// verify all fired beats are in range.
    fn assert_loop_pipeline(score: &TabScore, label: &str) {
        let tl = build_timeline(score, 0, 100.0, false);
        if score.bars.is_empty() || score.beats.is_empty() {
            return;
        }
        let first_bar = &score.bars[0];
        let loop_end_beat = first_bar.first_beat_index + first_bar.beat_count - 1;

        let mut engine = Engine::new();
        engine.command(Cmd::SetTimeline(tl));
        engine.command(Cmd::SetLoop(Some((0, loop_end_beat))));
        engine.command(Cmd::Play);
        engine.run(500_000);
        let fired = engine.take_fired();

        let expected_per_cycle = first_bar.beat_count;
        assert!(
            fired.len() >= expected_per_cycle * 2,
            "{}: expected at least 2 loop cycles ({} beats each), got {}",
            label,
            expected_per_cycle,
            fired.len(),
        );
        assert_beats_in_range(&fired, 0, loop_end_beat);
    }

    #[test]
    fn test_assembly_gp7_all_hammers() {
        let score = load_all_hammers();
        assert_full_pipeline(&score, "All Hammers (GP7)");
    }

    #[test]
    fn test_assembly_gp7_all_hammers_loop() {
        let score = load_all_hammers();
        assert_loop_pipeline(&score, "All Hammers (GP7)");
    }

    #[test]
    fn test_assembly_gp5_muted_legato() {
        let score = load_gp5("test_file_13.gp5");
        assert_full_pipeline(&score, "Muted Legato Arpeggios (GP5)");
    }

    #[test]
    fn test_assembly_gp5_muted_legato_loop() {
        let score = load_gp5("test_file_13.gp5");
        assert_loop_pipeline(&score, "Muted Legato Arpeggios (GP5)");
    }

    #[test]
    fn test_assembly_gp5_lydian_tapping() {
        let score = load_gp5("test_file_16.gp5");
        assert_full_pipeline(&score, "Lydian Legato Tapping (GP5)");
    }

    #[test]
    fn test_assembly_gp5_hybrid_picking() {
        let score = load_gp5("test_file_17.gp5");
        assert_full_pipeline(&score, "Legato/Hybrid Picking (GP5)");
    }

    #[test]
    fn test_assembly_gp7_string_crossing() {
        let score = load_gp7("test_file_2.gp");
        assert_full_pipeline(&score, "String Crossing 1 (GP7)");
    }

    #[test]
    fn test_assembly_gp7_string_crossing_loop() {
        let score = load_gp7("test_file_2.gp");
        assert_loop_pipeline(&score, "String Crossing 1 (GP7)");
    }

    #[test]
    fn test_assembly_gp7_primordial_arpeggios() {
        // Largest GP7 fixture: 286 beats
        let score = load_gp7("test_file_11.gp");
        assert_full_pipeline(&score, "Primordial Arpeggios 2 (GP7)");
    }

    #[test]
    fn test_assembly_gp7_diminished() {
        // Most beats: 330
        let score = load_gp7("test_file_15.gp");
        assert_full_pipeline(&score, "Diminished (GP7)");
    }

    #[test]
    fn test_assembly_gp7_diminished_loop() {
        let score = load_gp7("test_file_15.gp");
        assert_loop_pipeline(&score, "Diminished (GP7)");
    }

    #[test]
    fn test_assembly_gp5_pygp_funky_guy() {
        let score = load_gp5("pygp_001_Funky_Guy.gp5");
        assert_full_pipeline(&score, "Funky Guy (PyGP GP5)");
    }

    #[test]
    fn test_assembly_gp5_pygp_effects() {
        let score = load_gp5("pygp_Effects.gp5");
        assert_full_pipeline(&score, "Effects (PyGP GP5)");
    }

    // ── Cross-tempo assembly ───────────────────────────────────────────

    #[test]
    fn test_assembly_all_tempos() {
        let score = load_all_hammers();
        for tempo_pct in [25.0, 50.0, 75.0, 100.0, 150.0, 200.0] {
            let tl = build_timeline(&score, 0, tempo_pct, false);
            let mut engine = Engine::new();
            engine.command(Cmd::SetTimeline(tl));
            engine.command(Cmd::Play);
            engine.run(10_000_000);
            let fired = engine.take_fired();
            assert_eq!(
                fired.len(),
                184,
                "tempo {}%: expected 184 beats, got {}",
                tempo_pct,
                fired.len(),
            );
        }
    }

    // ── tick_to_sample component tests ─────────────────────────────────

    #[test]
    fn test_tick_to_sample_zero() {
        let score = load_all_hammers();
        assert_eq!(tick_to_sample(0.0, &score, 100.0), 0);
    }

    #[test]
    fn test_tick_to_sample_monotonic() {
        let score = load_all_hammers();
        let mut prev = 0u64;
        for beat in &score.beats {
            let sample = tick_to_sample(beat.tick, &score, 100.0);
            assert!(
                sample >= prev,
                "tick_to_sample not monotonic: tick {} → sample {} < prev {}",
                beat.tick,
                sample,
                prev,
            );
            prev = sample;
        }
    }

    #[test]
    fn test_tick_to_sample_tempo_scaling() {
        let score = load_all_hammers();
        let tick = score.beats.last().unwrap().tick;
        let s100 = tick_to_sample(tick, &score, 100.0);
        let s50 = tick_to_sample(tick, &score, 50.0);
        let s200 = tick_to_sample(tick, &score, 200.0);

        let ratio_slow = s50 as f64 / s100 as f64;
        let ratio_fast = s200 as f64 / s100 as f64;
        assert!((ratio_slow - 2.0).abs() < 0.01);
        assert!((ratio_fast - 0.5).abs() < 0.01);
    }
}
