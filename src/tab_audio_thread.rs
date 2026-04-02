/// Audio rendering thread for the tab player.
///
/// Owns a FluidSynthEngine, renders 256-frame stereo buffers, dispatches
/// MIDI events at correct sample positions, and pushes PCM to a GStreamer
/// appsrc pipeline. Beat callbacks are posted to the GLib main thread.

use crate::fluidsynth_ffi::FluidSynthEngine;
use crate::tab_midi::{BeatMarker, MidiEvent, MidiTimeline, SAMPLE_RATE};
use gst::prelude::*;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

const BUFFER_FRAMES: usize = 256;
const GUITAR_CHANNEL: u8 = 0;
const METRONOME_CHANNEL: u8 = 9;
const GUITAR_SFONT_PROGRAM: u32 = 25; // Electric Guitar (Clean)
const CC_VOLUME: u8 = 7;

pub enum TabAudioCommand {
    Play,
    PlayWithCountIn { bars: u32, bpm: f64, beats_per_bar: u8 },
    Pause,
    Stop,
    SeekToBeat(usize),
    SetTimeline(MidiTimeline),
    SetGuitarProgram(u32),
    SetGuitarVolume(u8),
    SetMetronomeVolume(u8),
    SetLoop(Option<(usize, usize)>),
}

#[derive(Clone)]
pub struct BeatCallback {
    callback: Arc<Mutex<Box<dyn Fn(usize, &[(u8, u8)]) + Send>>>,
}

impl BeatCallback {
    pub fn new<F: Fn(usize, &[(u8, u8)]) + Send + 'static>(callback: F) -> Self {
        Self {
            callback: Arc::new(Mutex::new(Box::new(callback))),
        }
    }

    fn fire(&self, beat_index: usize, notes: &[(u8, u8)]) {
        if let Ok(callback) = self.callback.lock() {
            log::debug!("[audio] fire beat_index={} notes={}", beat_index, notes.len());
            callback(beat_index, notes);
        } else {
            log::warn!("[audio] BeatCallback mutex poisoned, dropping beat {}", beat_index);
        }
    }
}

pub struct TabAudioThread {
    command_sender: mpsc::Sender<TabAudioCommand>,
}

impl std::fmt::Debug for TabAudioThread {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("TabAudioThread").finish()
    }
}

impl TabAudioThread {
    pub fn new(
        sonivox_path: PathBuf,
        metronome_sf_path: PathBuf,
        beat_callback: BeatCallback,
    ) -> Option<Self> {
        let (command_sender, command_receiver) = mpsc::channel();

        thread::spawn(move || {
            run_audio_loop(
                sonivox_path,
                metronome_sf_path,
                command_receiver,
                beat_callback,
            );
        });

        Some(Self { command_sender })
    }

    pub fn send(&self, command: TabAudioCommand) {
        self.command_sender.send(command).unwrap_or_default();
    }
}

impl Drop for TabAudioThread {
    fn drop(&mut self) {
        self.command_sender
            .send(TabAudioCommand::Stop)
            .unwrap_or_default();
    }
}

fn run_audio_loop(
    sonivox_path: PathBuf,
    metronome_sf_path: PathBuf,
    command_receiver: mpsc::Receiver<TabAudioCommand>,
    beat_callback: BeatCallback,
) {
    let synth = match FluidSynthEngine::new() {
        Some(synth) => synth,
        None => {
            log::error!("Failed to create FluidSynth engine");
            return;
        }
    };

    let guitar_sfont_id = match synth.load_soundfont(&sonivox_path) {
        Some(sfont_id) => sfont_id,
        None => {
            log::error!("Failed to load guitar SoundFont: {:?}", sonivox_path);
            return;
        }
    };

    let metronome_sfont_id = match synth.load_soundfont(&metronome_sf_path) {
        Some(sfont_id) => sfont_id,
        None => {
            log::error!(
                "Failed to load metronome SoundFont: {:?}",
                metronome_sf_path
            );
            return;
        }
    };

    // Set up instruments
    synth.program_select(GUITAR_CHANNEL, guitar_sfont_id, 0, GUITAR_SFONT_PROGRAM);
    synth.program_select(METRONOME_CHANNEL, metronome_sfont_id, 128, 0);
    synth.cc(GUITAR_CHANNEL, CC_VOLUME, 100);
    synth.cc(METRONOME_CHANNEL, CC_VOLUME, 100);

    // Set up GStreamer pipeline
    let pipeline = match create_pipeline() {
        Some(pipeline) => pipeline,
        None => {
            log::error!("Failed to create GStreamer pipeline");
            return;
        }
    };

    let appsrc = pipeline
        .by_name("tabsrc")
        .and_then(|element| element.downcast::<gst_app::AppSrc>().ok())
        .expect("Pipeline must have appsrc named 'tabsrc'");

    let mut timeline: Option<MidiTimeline> = None;
    let mut is_playing = false;
    let mut sample_position: u64 = 0;
    let mut event_index: usize = 0;
    let mut beat_marker_index: usize = 0;
    let mut loop_range: Option<(u64, u64)> = None; // (start_sample, end_sample)
    let mut loop_beat_range: Option<(usize, usize)> = None;

    let recv_timeout = std::time::Duration::from_millis(10);
    let mut pcm_buffer = vec![0i16; BUFFER_FRAMES * 2];

    #[allow(unused_assignments)]
    loop {
        // Process commands
        let command = if is_playing {
            command_receiver.try_recv().ok()
        } else {
            match command_receiver.recv_timeout(recv_timeout) {
                Ok(command) => Some(command),
                Err(_) => None,
            }
        };

        if let Some(command) = command {
            match command {
                TabAudioCommand::Play => {
                    if timeline.is_some() {
                        log::info!("[audio] Play — starting playback");
                        is_playing = true;
                        let _ = pipeline.set_state(gst::State::Playing);
                    }
                }
                TabAudioCommand::PlayWithCountIn { bars, bpm, beats_per_bar } => {
                    if timeline.is_some() {
                        let _ = pipeline.set_state(gst::State::Playing);
                        play_count_in(
                            &synth, &appsrc, &mut pcm_buffer,
                            bars, bpm, beats_per_bar,
                        );
                        is_playing = true;
                    }
                }
                TabAudioCommand::Pause => {
                    log::info!("[audio] Pause received");
                    is_playing = false;
                }
                TabAudioCommand::Stop => {
                    is_playing = false;
                    let _ = pipeline.set_state(gst::State::Null);
                    break;
                }
                TabAudioCommand::SetTimeline(new_timeline) => {
                    sample_position = 0;
                    event_index = 0;
                    beat_marker_index = 0;
                    timeline = Some(new_timeline);
                    update_loop_samples(&timeline, &loop_beat_range, &mut loop_range);
                }
                TabAudioCommand::SeekToBeat(beat_index) => {
                    if let Some(ref current_timeline) = timeline {
                        if let Some(marker) = current_timeline
                            .beat_markers
                            .iter()
                            .find(|marker| marker.beat_index == beat_index)
                        {
                            sample_position = marker.sample_position;
                            rewind_indices(
                                current_timeline,
                                sample_position,
                                &mut event_index,
                                &mut beat_marker_index,
                            );
                            synth.all_notes_off(GUITAR_CHANNEL);
                            synth.all_notes_off(METRONOME_CHANNEL);
                        }
                    }
                }
                TabAudioCommand::SetGuitarProgram(program) => {
                    synth.program_select(GUITAR_CHANNEL, guitar_sfont_id, 0, program);
                }
                TabAudioCommand::SetGuitarVolume(volume) => {
                    synth.cc(GUITAR_CHANNEL, CC_VOLUME, volume);
                }
                TabAudioCommand::SetMetronomeVolume(volume) => {
                    synth.cc(METRONOME_CHANNEL, CC_VOLUME, volume);
                }
                TabAudioCommand::SetLoop(beat_range) => {
                    loop_beat_range = beat_range;
                    update_loop_samples(&timeline, &loop_beat_range, &mut loop_range);
                }
            }
        }

        if !is_playing {
            continue;
        }

        let current_timeline = match timeline.as_ref() {
            Some(current_timeline) => current_timeline,
            None => continue,
        };

        // Clamp the buffer window to the loop/song boundary so events and
        // beat callbacks past the end never fire before the rewind check.
        let raw_buffer_end = sample_position + BUFFER_FRAMES as u64;
        let boundary = loop_range
            .map(|(_, end)| end)
            .unwrap_or(current_timeline.total_samples);
        let buffer_end = raw_buffer_end.min(boundary);

        // Dispatch MIDI events in this buffer window
        while event_index < current_timeline.events.len() {
            let event = &current_timeline.events[event_index];
            if event.sample_position() >= buffer_end {
                break;
            }
            if event.sample_position() >= sample_position {
                dispatch_event(&synth, event);
            }
            event_index += 1;
        }

        // Fire beat callbacks
        while beat_marker_index < current_timeline.beat_markers.len() {
            let marker = &current_timeline.beat_markers[beat_marker_index];
            if marker.sample_position >= buffer_end {
                break;
            }
            if marker.sample_position >= sample_position {
                // Collect notes for this beat
                let beat_notes: Vec<(u8, u8)> = collect_beat_notes(current_timeline, marker);
                beat_callback.fire(marker.beat_index, &beat_notes);
            }
            beat_marker_index += 1;
        }

        // Render audio
        synth.render_s16(&mut pcm_buffer, BUFFER_FRAMES);

        // Push to GStreamer
        let byte_data: Vec<u8> = pcm_buffer
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect();

        let buffer = gst::Buffer::from_slice(byte_data);
        if appsrc.push_buffer(buffer).is_err() {
            log::warn!("Failed to push buffer to appsrc");
            break;
        }

        sample_position += BUFFER_FRAMES as u64;

        // Check loop / end
        let end_sample = loop_range
            .map(|(_, end)| end)
            .unwrap_or(current_timeline.total_samples);

        if sample_position >= end_sample {
            let start_sample = loop_range.map(|(start, _)| start).unwrap_or(0);
            if loop_range.is_some() {
                // Loop: reset to start
                sample_position = start_sample;
                rewind_indices(
                    current_timeline,
                    sample_position,
                    &mut event_index,
                    &mut beat_marker_index,
                );
                synth.all_notes_off(GUITAR_CHANNEL);
                synth.all_notes_off(METRONOME_CHANNEL);
            } else {
                // End of song
                is_playing = false;
                sample_position = 0;
                event_index = 0;
                beat_marker_index = 0;
                synth.all_notes_off(GUITAR_CHANNEL);
                synth.all_notes_off(METRONOME_CHANNEL);
            }
        }
    }

    let _ = pipeline.set_state(gst::State::Null);
}

fn create_pipeline() -> Option<gst::Pipeline> {
    let pipeline = gst::Pipeline::with_name("tab-player-pipeline");

    let appsrc = gst::ElementFactory::make("appsrc")
        .name("tabsrc")
        .build()
        .ok()?;

    let caps = gst::Caps::builder("audio/x-raw")
        .field("format", "S16LE")
        .field("rate", SAMPLE_RATE as i32)
        .field("channels", 2i32)
        .field("layout", "interleaved")
        .build();

    appsrc.set_property("caps", &caps);
    appsrc.set_property("format", gst::Format::Time);
    appsrc.set_property("is-live", true);
    appsrc.set_property("do-timestamp", true);
    // Block push_buffer when internal queue is full — provides hardware-paced
    // backpressure instead of thread::sleep, same as Android's AudioTrack.write()
    appsrc.set_property("block", true);
    // Keep queue small: 4 buffers × 256 frames × 2 channels × 2 bytes = 4096 bytes
    appsrc.set_property("max-bytes", 4096u64);

    let audioconvert = gst::ElementFactory::make("audioconvert").build().ok()?;
    let audiosink = gst::ElementFactory::make("autoaudiosink").build().ok()?;

    pipeline.add_many([&appsrc, &audioconvert, &audiosink]).ok()?;
    gst::Element::link_many([&appsrc, &audioconvert, &audiosink]).ok()?;

    Some(pipeline)
}

fn play_count_in(
    synth: &FluidSynthEngine,
    appsrc: &gst_app::AppSrc,
    pcm_buffer: &mut [i16],
    bars: u32,
    bpm: f64,
    beats_per_bar: u8,
) {
    let samples_per_beat = (SAMPLE_RATE * 60.0 / bpm) as u64;
    let total_beats = bars * beats_per_bar as u32;
    let accent_note: u8 = 75; // claves
    let regular_note: u8 = 37; // side stick
    let click_duration_samples: u64 = (SAMPLE_RATE * 0.05) as u64; // 50ms

    for beat in 0..total_beats {
        let is_accent = beat % beats_per_bar as u32 == 0;
        let note = if is_accent { accent_note } else { regular_note };
        let velocity: u8 = if is_accent { 100 } else { 80 };

        synth.note_on(METRONOME_CHANNEL, note, velocity);

        let mut samples_rendered: u64 = 0;
        let mut note_off_sent = false;

        while samples_rendered < samples_per_beat {
            if !note_off_sent && samples_rendered >= click_duration_samples {
                synth.note_off(METRONOME_CHANNEL, note);
                note_off_sent = true;
            }

            synth.render_s16(pcm_buffer, BUFFER_FRAMES);
            let byte_data: Vec<u8> = pcm_buffer
                .iter()
                .flat_map(|sample| sample.to_le_bytes())
                .collect();
            let buffer = gst::Buffer::from_slice(byte_data);
            if appsrc.push_buffer(buffer).is_err() {
                return;
            }
            samples_rendered += BUFFER_FRAMES as u64;
        }

        if !note_off_sent {
            synth.note_off(METRONOME_CHANNEL, note);
        }
    }
}

fn dispatch_event(synth: &FluidSynthEngine, event: &MidiEvent) {
    match event {
        MidiEvent::NoteOn {
            channel,
            key,
            velocity,
            ..
        } => synth.note_on(*channel, *key, *velocity),
        MidiEvent::NoteOff { channel, key, .. } => synth.note_off(*channel, *key),
    }
}

fn rewind_indices(
    timeline: &MidiTimeline,
    target_sample: u64,
    event_index: &mut usize,
    beat_marker_index: &mut usize,
) {
    *event_index = timeline
        .events
        .partition_point(|event| event.sample_position() < target_sample);
    *beat_marker_index = timeline
        .beat_markers
        .partition_point(|marker| marker.sample_position < target_sample);
}

fn update_loop_samples(
    timeline: &Option<MidiTimeline>,
    loop_beat_range: &Option<(usize, usize)>,
    loop_range: &mut Option<(u64, u64)>,
) {
    match (timeline, loop_beat_range) {
        (Some(current_timeline), Some((start_beat, end_beat))) => {
            let start_sample = current_timeline
                .beat_markers
                .iter()
                .find(|marker| marker.beat_index == *start_beat)
                .map(|marker| marker.sample_position)
                .unwrap_or(0);

            // End sample is the start of the beat AFTER end_beat
            let end_sample = current_timeline
                .beat_markers
                .iter()
                .find(|marker| marker.beat_index == *end_beat + 1)
                .map(|marker| marker.sample_position)
                .unwrap_or(current_timeline.total_samples);

            *loop_range = Some((start_sample, end_sample));
        }
        _ => {
            *loop_range = None;
        }
    }
}

fn collect_beat_notes(timeline: &MidiTimeline, marker: &BeatMarker) -> Vec<(u8, u8)> {
    // Find the next beat marker to determine the beat's time range
    let next_marker_pos = timeline
        .beat_markers
        .iter()
        .find(|next_marker| next_marker.sample_position > marker.sample_position)
        .map(|next_marker| next_marker.sample_position)
        .unwrap_or(timeline.total_samples);

    timeline
        .events
        .iter()
        .filter_map(|event| match event {
            MidiEvent::NoteOn {
                sample_position,
                channel,
                key,
                ..
            } if *channel == GUITAR_CHANNEL
                && *sample_position >= marker.sample_position
                && *sample_position < next_marker_pos =>
            {
                // Return (string, fret) — but we don't have that mapping here.
                // Instead return (channel, key) and let the UI resolve it.
                Some((*channel, *key))
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tab_midi::{BeatMarker, MidiEvent, MidiTimeline};

    fn make_timeline(beat_positions: &[u64], note_positions: &[u64]) -> MidiTimeline {
        let beat_markers: Vec<BeatMarker> = beat_positions
            .iter()
            .enumerate()
            .map(|(index, &pos)| BeatMarker {
                sample_position: pos,
                beat_index: index,
            })
            .collect();
        let mut events: Vec<MidiEvent> = Vec::new();
        for &pos in note_positions {
            events.push(MidiEvent::NoteOn {
                sample_position: pos,
                channel: GUITAR_CHANNEL,
                key: 60,
                velocity: 80,
            });
            events.push(MidiEvent::NoteOff {
                sample_position: pos + 1000,
                channel: GUITAR_CHANNEL,
                key: 60,
            });
        }
        events.sort_by_key(|e| e.sample_position());
        let total_samples = beat_positions
            .last()
            .copied()
            .unwrap_or(0)
            + 5000;
        MidiTimeline {
            events,
            beat_markers,
            total_samples,
        }
    }

    // ── rewind_indices ─────────────────────────────────────────────────

    #[test]
    fn test_rewind_to_start() {
        let tl = make_timeline(&[0, 1000, 2000, 3000], &[0, 1000, 2000, 3000]);
        let mut event_idx = 99;
        let mut beat_idx = 99;
        rewind_indices(&tl, 0, &mut event_idx, &mut beat_idx);
        assert_eq!(event_idx, 0);
        assert_eq!(beat_idx, 0);
    }

    #[test]
    fn test_rewind_to_midpoint() {
        let tl = make_timeline(&[0, 1000, 2000, 3000], &[0, 1000, 2000, 3000]);
        let mut event_idx = 0;
        let mut beat_idx = 0;
        rewind_indices(&tl, 2000, &mut event_idx, &mut beat_idx);
        // Beat markers: [0, 1000, 2000, 3000]. partition_point(< 2000) = 2
        assert_eq!(beat_idx, 2);
    }

    #[test]
    fn test_rewind_past_end() {
        let tl = make_timeline(&[0, 1000, 2000], &[0, 1000, 2000]);
        let mut event_idx = 0;
        let mut beat_idx = 0;
        rewind_indices(&tl, 99999, &mut event_idx, &mut beat_idx);
        assert_eq!(beat_idx, tl.beat_markers.len());
        assert_eq!(event_idx, tl.events.len());
    }

    #[test]
    fn test_rewind_between_markers() {
        let tl = make_timeline(&[0, 1000, 2000, 3000], &[]);
        let mut event_idx = 0;
        let mut beat_idx = 0;
        rewind_indices(&tl, 1500, &mut event_idx, &mut beat_idx);
        // First marker at or after 1500 is index 2 (pos=2000). partition_point(< 1500) = 2.
        assert_eq!(beat_idx, 2);
    }

    // ── update_loop_samples ────────────────────────────────────────────

    #[test]
    fn test_update_loop_basic() {
        let tl = make_timeline(&[0, 1000, 2000, 3000], &[]);
        let timeline = Some(tl);
        let beat_range = Some((1, 2)); // beats 1..2, end = beat 3's position
        let mut loop_range = None;
        update_loop_samples(&timeline, &beat_range, &mut loop_range);

        assert_eq!(loop_range, Some((1000, 3000)));
    }

    #[test]
    fn test_update_loop_first_beat() {
        let tl = make_timeline(&[0, 1000, 2000], &[]);
        let timeline = Some(tl);
        let beat_range = Some((0, 0));
        let mut loop_range = None;
        update_loop_samples(&timeline, &beat_range, &mut loop_range);

        // Start=0, end=beat 1's position=1000
        assert_eq!(loop_range, Some((0, 1000)));
    }

    #[test]
    fn test_update_loop_last_beat_falls_back_to_total() {
        let tl = make_timeline(&[0, 1000, 2000], &[]);
        let total = tl.total_samples;
        let timeline = Some(tl);
        let beat_range = Some((2, 2)); // end_beat+1=3 doesn't exist
        let mut loop_range = None;
        update_loop_samples(&timeline, &beat_range, &mut loop_range);

        assert_eq!(loop_range, Some((2000, total)));
    }

    #[test]
    fn test_update_loop_none_clears() {
        let tl = make_timeline(&[0, 1000], &[]);
        let timeline = Some(tl);
        let mut loop_range = Some((0, 1000));
        update_loop_samples(&timeline, &None, &mut loop_range);
        assert_eq!(loop_range, None);
    }

    #[test]
    fn test_update_loop_no_timeline_clears() {
        let mut loop_range = Some((0, 1000));
        update_loop_samples(&None, &Some((0, 5)), &mut loop_range);
        assert_eq!(loop_range, None);
    }

    #[test]
    fn test_update_loop_invalid_start_beat_defaults_to_zero() {
        let tl = make_timeline(&[0, 1000, 2000], &[]);
        let timeline = Some(tl);
        let beat_range = Some((999, 999)); // neither beat exists
        let mut loop_range = None;
        update_loop_samples(&timeline, &beat_range, &mut loop_range);

        // start defaults to 0 (unwrap_or), end defaults to total_samples
        let total = timeline.as_ref().unwrap().total_samples;
        assert_eq!(loop_range, Some((0, total)));
    }

    // ── collect_beat_notes ─────────────────────────────────────────────

    #[test]
    fn test_collect_notes_single_beat() {
        let tl = make_timeline(&[0, 1000], &[0]);
        let marker = &tl.beat_markers[0];
        let notes = collect_beat_notes(&tl, marker);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0], (GUITAR_CHANNEL, 60));
    }

    #[test]
    fn test_collect_notes_excludes_adjacent_beat() {
        let tl = make_timeline(&[0, 1000, 2000], &[0, 1000, 2000]);
        // Beat 1 should only collect the note at 1000, not 0 or 2000
        let marker = &tl.beat_markers[1];
        let notes = collect_beat_notes(&tl, marker);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn test_collect_notes_last_beat() {
        let tl = make_timeline(&[0, 1000, 2000], &[2000]);
        let marker = &tl.beat_markers[2]; // last beat
        let notes = collect_beat_notes(&tl, marker);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn test_collect_notes_rest_beat() {
        let tl = make_timeline(&[0, 1000, 2000], &[]); // no notes at all
        let marker = &tl.beat_markers[1];
        let notes = collect_beat_notes(&tl, marker);
        assert_eq!(notes.len(), 0);
    }

    #[test]
    fn test_collect_notes_ignores_metronome_channel() {
        let mut tl = make_timeline(&[0, 1000], &[0]);
        // Add a metronome note at the same position
        tl.events.push(MidiEvent::NoteOn {
            sample_position: 0,
            channel: METRONOME_CHANNEL,
            key: 75,
            velocity: 100,
        });
        tl.events.sort_by_key(|e| e.sample_position());

        let notes = collect_beat_notes(&tl, &tl.beat_markers[0]);
        // Should only return the guitar note, not the metronome
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].0, GUITAR_CHANNEL);
    }

    // ── Assembly tests ─────────────────────────────────────────────────
    // Real TabAudioThread + real FluidSynth + real GStreamer pipeline.
    // No mocks — exercises actual threading, mutex contention, and timing.

    fn soundfont_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/soundfonts")
    }

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    /// Collect beat callbacks from a real audio thread into a shared vec.
    fn collecting_callback() -> (BeatCallback, Arc<Mutex<Vec<usize>>>) {
        let fired = Arc::new(Mutex::new(Vec::new()));
        let fired_ref = fired.clone();
        let callback = BeatCallback::new(move |beat_index, _notes| {
            fired_ref.lock().unwrap().push(beat_index);
        });
        (callback, fired)
    }

    fn wait_until(
        fired: &Arc<Mutex<Vec<usize>>>,
        predicate: impl Fn(&[usize]) -> bool,
        timeout: std::time::Duration,
    ) -> Vec<usize> {
        let start = std::time::Instant::now();
        loop {
            let snapshot = fired.lock().unwrap().clone();
            if predicate(&snapshot) || start.elapsed() > timeout {
                return snapshot;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[test]
    fn test_assembly_real_playback_short_loop() {
        gst::init().expect("GStreamer init");

        let (callback, fired) = collecting_callback();
        let audio_thread = TabAudioThread::new(
            soundfont_dir().join("sonivox.sf2"),
            soundfont_dir().join("metronome_clicks.sf2"),
            callback,
        )
        .expect("audio thread");

        // Parse All Hammers, build timeline at 200% speed
        let path = fixture_path("test_file_1.gp");
        let (score, _) = crate::gp7_parser::parse_file(&path).unwrap();
        let timeline = crate::tab_midi::build_timeline(&score, 0, 200.0, false);

        audio_thread.send(TabAudioCommand::SetTimeline(timeline));
        // Loop first bar only (16 beats) — takes ~2s at 200%
        audio_thread.send(TabAudioCommand::SetLoop(Some((0, 15))));
        audio_thread.send(TabAudioCommand::Play);

        // Wait for at least 2 full loop cycles (32 beats)
        let beats = wait_until(&fired, |b| b.len() >= 32, std::time::Duration::from_secs(10));

        assert!(
            beats.len() >= 32,
            "expected >= 32 beats from 2 loop cycles, got {}",
            beats.len(),
        );

        // All beats should be within the loop range
        for &beat_index in &beats {
            assert!(
                beat_index <= 15,
                "beat {} leaked past loop end [0, 15]",
                beat_index,
            );
        }

        // Verify sequential ordering within each 16-beat cycle
        for chunk in beats.chunks(16) {
            if chunk.len() < 16 {
                break;
            }
            for (offset, &beat_index) in chunk.iter().enumerate() {
                assert_eq!(beat_index, offset, "wrong order in loop cycle");
            }
        }

        audio_thread.send(TabAudioCommand::Stop);
    }

    #[test]
    fn test_assembly_real_pause_resume() {
        gst::init().expect("GStreamer init");

        let (callback, fired) = collecting_callback();
        let audio_thread = TabAudioThread::new(
            soundfont_dir().join("sonivox.sf2"),
            soundfont_dir().join("metronome_clicks.sf2"),
            callback,
        )
        .expect("audio thread");

        let path = fixture_path("test_file_1.gp");
        let (score, _) = crate::gp7_parser::parse_file(&path).unwrap();
        let timeline = crate::tab_midi::build_timeline(&score, 0, 200.0, false);

        audio_thread.send(TabAudioCommand::SetTimeline(timeline));
        audio_thread.send(TabAudioCommand::SetLoop(Some((0, 15))));
        audio_thread.send(TabAudioCommand::Play);

        // Wait for some beats
        wait_until(&fired, |b| b.len() >= 8, std::time::Duration::from_secs(5));

        // Pause
        audio_thread.send(TabAudioCommand::Pause);
        std::thread::sleep(std::time::Duration::from_millis(200));
        let count_at_pause = fired.lock().unwrap().len();

        // Wait — no new beats should arrive while paused
        std::thread::sleep(std::time::Duration::from_millis(500));
        let count_after_wait = fired.lock().unwrap().len();
        assert_eq!(
            count_at_pause, count_after_wait,
            "beats arrived while paused: {} → {}",
            count_at_pause, count_after_wait,
        );

        // Resume — beats should flow again
        audio_thread.send(TabAudioCommand::Play);
        wait_until(
            &fired,
            |b| b.len() > count_after_wait + 4,
            std::time::Duration::from_secs(5),
        );
        let final_count = fired.lock().unwrap().len();
        assert!(
            final_count > count_after_wait,
            "no beats after resume: {} == {}",
            final_count,
            count_after_wait,
        );

        audio_thread.send(TabAudioCommand::Stop);
    }

    #[test]
    fn test_assembly_real_seek_resets_position() {
        gst::init().expect("GStreamer init");

        let (callback, fired) = collecting_callback();
        let audio_thread = TabAudioThread::new(
            soundfont_dir().join("sonivox.sf2"),
            soundfont_dir().join("metronome_clicks.sf2"),
            callback,
        )
        .expect("audio thread");

        let path = fixture_path("test_file_1.gp");
        let (score, _) = crate::gp7_parser::parse_file(&path).unwrap();
        let timeline = crate::tab_midi::build_timeline(&score, 0, 200.0, false);

        audio_thread.send(TabAudioCommand::SetTimeline(timeline));
        audio_thread.send(TabAudioCommand::SetLoop(Some((0, 31))));
        audio_thread.send(TabAudioCommand::Play);

        // Let it play a bit
        wait_until(&fired, |b| b.len() >= 8, std::time::Duration::from_secs(5));

        // Seek to beat 0 — should restart the loop from the beginning
        audio_thread.send(TabAudioCommand::SeekToBeat(0));
        std::thread::sleep(std::time::Duration::from_millis(500));

        let beats = fired.lock().unwrap().clone();
        // After seek to 0, beat 0 should appear more than once (initial + post-seek)
        let zero_count = beats.iter().filter(|&&b| b == 0).count();
        assert!(
            zero_count >= 2,
            "beat 0 should fire at least twice (initial + seek), got {}",
            zero_count,
        );

        audio_thread.send(TabAudioCommand::Stop);
    }

    #[test]
    fn test_assembly_real_loop_change_mid_playback() {
        gst::init().expect("GStreamer init");

        let (callback, fired) = collecting_callback();
        let audio_thread = TabAudioThread::new(
            soundfont_dir().join("sonivox.sf2"),
            soundfont_dir().join("metronome_clicks.sf2"),
            callback,
        )
        .expect("audio thread");

        let path = fixture_path("test_file_1.gp");
        let (score, _) = crate::gp7_parser::parse_file(&path).unwrap();
        let timeline = crate::tab_midi::build_timeline(&score, 0, 200.0, false);

        audio_thread.send(TabAudioCommand::SetTimeline(timeline));
        audio_thread.send(TabAudioCommand::SetLoop(Some((0, 15))));
        audio_thread.send(TabAudioCommand::Play);

        // Let first loop run
        wait_until(&fired, |b| b.len() >= 20, std::time::Duration::from_secs(5));

        // Switch to bar 2 loop (beats 32-47)
        audio_thread.send(TabAudioCommand::SetLoop(Some((32, 47))));

        // Wait for the transition to complete and several new-loop cycles
        // to run. The audio thread plays through beats 16-47 before the
        // first loop-back to 32, so we need enough beats for that
        // transition plus 2+ clean cycles.
        wait_until(&fired, |b| {
            let in_new_loop = b.iter().filter(|&&x| x >= 32 && x <= 47).count();
            in_new_loop >= 48 // at least 3 clean cycles
        }, std::time::Duration::from_secs(15));

        let beats = fired.lock().unwrap().clone();
        // After the transition settles, the LAST 32 beats should all be
        // within the new loop range (2 full cycles, no transition beats).
        let stable_tail: Vec<_> = beats.iter().rev().take(32).copied().collect();
        for &beat_index in &stable_tail {
            assert!(
                beat_index >= 32 && beat_index <= 47,
                "stable tail beat {} outside new loop [32, 47]",
                beat_index,
            );
        }

        audio_thread.send(TabAudioCommand::Stop);
    }

    #[test]
    fn test_assembly_real_stop_terminates_thread() {
        gst::init().expect("GStreamer init");

        let (callback, fired) = collecting_callback();
        let audio_thread = TabAudioThread::new(
            soundfont_dir().join("sonivox.sf2"),
            soundfont_dir().join("metronome_clicks.sf2"),
            callback,
        )
        .expect("audio thread");

        let path = fixture_path("test_file_1.gp");
        let (score, _) = crate::gp7_parser::parse_file(&path).unwrap();
        let timeline = crate::tab_midi::build_timeline(&score, 0, 200.0, false);

        audio_thread.send(TabAudioCommand::SetTimeline(timeline));
        audio_thread.send(TabAudioCommand::SetLoop(Some((0, 15))));
        audio_thread.send(TabAudioCommand::Play);

        wait_until(&fired, |b| b.len() >= 8, std::time::Duration::from_secs(5));

        audio_thread.send(TabAudioCommand::Stop);
        std::thread::sleep(std::time::Duration::from_millis(200));
        let count_at_stop = fired.lock().unwrap().len();

        std::thread::sleep(std::time::Duration::from_millis(500));
        let count_after = fired.lock().unwrap().len();
        assert_eq!(
            count_at_stop, count_after,
            "beats still arriving after Stop: {} → {}",
            count_at_stop, count_after,
        );
    }

    #[test]
    fn test_assembly_gp5_real_playback() {
        gst::init().expect("GStreamer init");

        let (callback, fired) = collecting_callback();
        let audio_thread = TabAudioThread::new(
            soundfont_dir().join("sonivox.sf2"),
            soundfont_dir().join("metronome_clicks.sf2"),
            callback,
        )
        .expect("audio thread");

        // Use a short GP5 fixture (Harmonics: 1 bar, 4 beats)
        let path = fixture_path("pygp_Harmonics.gp5");
        let (score, _) = crate::gp5_parser::parse_file(&path).unwrap();
        let timeline = crate::tab_midi::build_timeline(&score, 0, 200.0, false);
        let total_beats = score.beats.len();

        audio_thread.send(TabAudioCommand::SetTimeline(timeline));
        audio_thread.send(TabAudioCommand::Play);

        // Short file — should finish quickly
        let beats = wait_until(
            &fired,
            |b| b.len() >= total_beats,
            std::time::Duration::from_secs(10),
        );

        assert_eq!(
            beats.len(),
            total_beats,
            "GP5: expected {} beats, got {}",
            total_beats,
            beats.len(),
        );
        for (position, &beat_index) in beats.iter().enumerate() {
            assert_eq!(beat_index, position);
        }

        audio_thread.send(TabAudioCommand::Stop);
    }
}
