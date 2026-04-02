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
            callback(beat_index, notes);
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

        // Dispatch MIDI events in this buffer window
        let buffer_end = sample_position + BUFFER_FRAMES as u64;
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
