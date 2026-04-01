use crate::chord_builder::VoicingNote;
use crate::scale_data::STANDARD_TUNING;
use gtk::glib;
use std::time::Duration;

const MIDI_BASE: u8 = 36;
const MIDI_MIN: u8 = 40;
const MIDI_MAX: u8 = 84;
const STRUM_DELAY_MS: u64 = 28;
const STRUM_JITTER_MS: u64 = 4;
const VOLUME_MIN: f64 = 0.68;
const VOLUME_MAX: f64 = 0.85;

/// Simple pseudo-random u32 using thread-local xorshift state.
fn rand_u32() -> u32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u32> = Cell::new(0xDEAD_BEEF);
    }
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        s.set(x);
        x
    })
}

fn rand_f64() -> f64 {
    (rand_u32() as f64) / (u32::MAX as f64)
}

fn guitar_uri(midi_note: u8) -> String {
    let clamped = midi_note.clamp(MIDI_MIN, MIDI_MAX);
    format!(
        "resource:///com/adrienplazas/Metronome/audio/guitar/guitar_{}.ogg",
        clamped
    )
}

/// Plays a chord voicing as a strummed guitar sound using GStreamer.
pub fn play_chord(voicing: &[VoicingNote]) {
    if voicing.is_empty() {
        return;
    }

    // Sort by string_index descending (low E = 5 first → downstroke strum order)
    let mut sorted: Vec<&VoicingNote> = voicing.iter().collect();
    sorted.sort_by(|a, b| b.string_index.cmp(&a.string_index));

    for (strum_order, note) in sorted.iter().enumerate() {
        let physical_string = 5 - note.string_index;
        let open_note = STANDARD_TUNING[physical_string];
        let midi_note = (MIDI_BASE + open_note + note.fret as u8).clamp(MIDI_MIN, MIDI_MAX);
        let uri = guitar_uri(midi_note);

        // Humanized strum: randomize timing and velocity per note
        let jitter = (rand_u32() % (STRUM_JITTER_MS as u32 + 1)) as u64;
        let delay = Duration::from_millis(strum_order as u64 * STRUM_DELAY_MS + jitter);
        let velocity = VOLUME_MIN + (rand_f64() * (VOLUME_MAX - VOLUME_MIN));

        glib::timeout_add_local_once(delay, move || {
            let player =
                gstreamer_play::Play::new(None::<gstreamer_play::PlayVideoRenderer>);
            player.set_uri(Some(&uri));
            player.set_volume(velocity);
            player.play();

            glib::timeout_add_local_once(Duration::from_secs(3), move || {
                player.stop();
            });
        });
    }
}
