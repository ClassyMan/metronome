/// Audio playback using rodio — replaces GStreamer for click/sample playback.

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;

const CLICK_HIGH: &[u8] = include_bytes!("../../data/resources/audio/clicker-high.ogg");
const CLICK_LOW: &[u8] = include_bytes!("../../data/resources/audio/clicker-low.ogg");

pub struct ClickPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    volume: f32,
}

impl ClickPlayer {
    pub fn new() -> Option<Self> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(Self {
            _stream: stream,
            handle,
            volume: 1.0,
        })
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn click(&self, is_downbeat: bool) {
        let data = if is_downbeat { CLICK_HIGH } else { CLICK_LOW };
        if let Ok(source) = Decoder::new(Cursor::new(data)) {
            if let Ok(sink) = Sink::try_new(&self.handle) {
                sink.set_volume(self.volume);
                sink.append(source);
                sink.detach();
            }
        }
    }
}
