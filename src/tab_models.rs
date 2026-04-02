/// Data model for guitar tablature — consumed by the MIDI timeline builder,
/// tab strip widget, and tab fretboard widget. No GTK dependencies.

pub const TICKS_PER_QUARTER: f64 = 960.0;

#[derive(Debug, Clone)]
pub struct TabNote {
    pub string: u8,
    pub fret: u8,
}

#[derive(Debug, Clone)]
pub struct TabBeat {
    pub bar_index: usize,
    pub beat_index: usize,
    pub tick: f64,
    pub duration: f64,
    pub is_rest: bool,
    pub notes: Vec<TabNote>,
}

#[derive(Debug, Clone)]
pub struct TabBar {
    pub index: usize,
    pub first_beat_index: usize,
    pub beat_count: usize,
    pub time_sig_num: u8,
    pub time_sig_denom: u8,
    pub tempo: f64,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub name: String,
    pub tuning: Vec<u8>,
    pub capo: u8,
    pub string_count: u8,
    pub midi_channel: u8,
    pub midi_port: u8,
}

#[derive(Debug, Clone)]
pub struct TabScore {
    pub beats: Vec<TabBeat>,
    pub bars: Vec<TabBar>,
    pub total_ticks: f64,
    pub tracks: Vec<TrackInfo>,
    pub title: String,
    pub artist: String,
}
