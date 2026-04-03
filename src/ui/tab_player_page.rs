use super::tab_fretboard_canvas::TabFretboardCanvas;
use super::tab_strip_canvas::TabStripCanvas;
use crate::gp5_parser;
use crate::gp7_parser;
use crate::tab_audio_thread::{BeatCallback, TabAudioCommand, TabAudioThread};
use crate::tab_midi;
use crate::tab_models::TabScore;
use iced::widget::{button, canvas, column, container, row, slider, text, Space};
use iced::{Element, Length, Subscription};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum Message {
    OpenFile,
    FileSelected(PathBuf),
    Play,
    Pause,
    Stop,
    SeekToBeat(usize),
    PrevBar,
    NextBar,
    ToggleLoop,
    ToggleMetronome,
    CycleTone,
    SetTempo(f64),
    SetGuitarVolume(f64),
    SetMetronomeVolume(f64),
    OnBeat(usize),
    SetTrack(usize),
    PollBeats,
}

pub struct TabPlayerPage {
    score: Option<TabScore>,
    file_path: Option<PathBuf>,
    is_playing: bool,
    current_beat: usize,
    selected_track: usize,
    tempo_percent: f64,
    guitar_volume: u8,
    metronome_volume: u8,
    metronome_enabled: bool,
    loop_active: bool,
    tone_index: usize,
    audio_thread: Option<TabAudioThread>,
    beat_receiver: Option<Arc<Mutex<Vec<usize>>>>,
    tab_strip: TabStripCanvas,
    tab_fretboard: TabFretboardCanvas,
}

const TONES: [&str; 3] = ["Clean", "Crunch", "Lead"];
const TONE_PROGRAMS: [u32; 3] = [25, 29, 30];

impl TabPlayerPage {
    pub fn new() -> Self {
        Self {
            score: None,
            file_path: None,
            is_playing: false,
            current_beat: 0,
            selected_track: 0,
            tempo_percent: 100.0,
            guitar_volume: 100,
            metronome_volume: 100,
            metronome_enabled: false,
            loop_active: false,
            tone_index: 0,
            audio_thread: None,
            beat_receiver: None,
            tab_strip: TabStripCanvas::new(),
            tab_fretboard: TabFretboardCanvas::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::FileSelected(path) => {
                self.load_file(&path);
                iced::Task::none()
            }
            Message::OpenFile => {
                iced::Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("GuitarPro", &["gp5", "gp", "gp4", "gp3"])
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    |path| match path {
                        Some(path) => Message::FileSelected(path),
                        None => Message::PollBeats, // cancelled — no-op
                    },
                )
            }
            Message::Play => {
                if self.score.is_some() {
                    self.is_playing = true;
                    if let Some(ref audio) = self.audio_thread {
                        audio.send(TabAudioCommand::Play);
                    }
                }
                iced::Task::none()
            }
            Message::Pause => {
                self.is_playing = false;
                if let Some(ref audio) = self.audio_thread {
                    audio.send(TabAudioCommand::Pause);
                }
                iced::Task::none()
            }
            Message::Stop => {
                self.is_playing = false;
                self.current_beat = 0;
                if let Some(ref audio) = self.audio_thread {
                    audio.send(TabAudioCommand::Pause);
                    audio.send(TabAudioCommand::SeekToBeat(0));
                }
                iced::Task::none()
            }
            Message::SeekToBeat(beat) => {
                self.current_beat = beat;
                if let Some(ref audio) = self.audio_thread {
                    audio.send(TabAudioCommand::SeekToBeat(beat));
                }
                iced::Task::none()
            }
            Message::PrevBar => {
                self.navigate_bar(-1);
                iced::Task::none()
            }
            Message::NextBar => {
                self.navigate_bar(1);
                iced::Task::none()
            }
            Message::ToggleLoop => {
                self.loop_active = !self.loop_active;
                if !self.loop_active {
                    if let Some(ref audio) = self.audio_thread {
                        audio.send(TabAudioCommand::SetLoop(None));
                    }
                }
                iced::Task::none()
            }
            Message::ToggleMetronome => {
                self.metronome_enabled = !self.metronome_enabled;
                self.rebuild_timeline();
                iced::Task::none()
            }
            Message::CycleTone => {
                self.tone_index = (self.tone_index + 1) % TONES.len();
                if let Some(ref audio) = self.audio_thread {
                    audio.send(TabAudioCommand::SetGuitarProgram(
                        TONE_PROGRAMS[self.tone_index],
                    ));
                }
                iced::Task::none()
            }
            Message::SetTempo(pct) => {
                self.tempo_percent = pct.clamp(25.0, 200.0);
                self.rebuild_timeline();
                iced::Task::none()
            }
            Message::SetGuitarVolume(vol) => {
                self.guitar_volume = (vol as u8).min(127);
                if let Some(ref audio) = self.audio_thread {
                    audio.send(TabAudioCommand::SetGuitarVolume(self.guitar_volume));
                }
                iced::Task::none()
            }
            Message::SetMetronomeVolume(vol) => {
                self.metronome_volume = (vol as u8).min(127);
                if let Some(ref audio) = self.audio_thread {
                    audio.send(TabAudioCommand::SetMetronomeVolume(self.metronome_volume));
                }
                iced::Task::none()
            }
            Message::OnBeat(beat_index) => {
                self.current_beat = beat_index;
                self.tab_strip.set_current_beat(beat_index as i32);
                // Update fretboard with notes from this beat
                if let Some(ref score) = self.score {
                    if let Some(beat) = score.beats.get(beat_index) {
                        let note_pairs: Vec<(u8, u8)> =
                            beat.notes.iter().map(|n| (n.string, n.fret)).collect();
                        if !note_pairs.is_empty() {
                            self.tab_fretboard.set_active_notes(&note_pairs);
                        }
                    }
                    if beat_index + 1 >= score.beats.len() && !self.loop_active {
                        self.is_playing = false;
                    }
                }
                iced::Task::none()
            }
            Message::SetTrack(track) => {
                self.selected_track = track;
                self.current_beat = 0;
                self.reload_for_track();
                iced::Task::none()
            }
            Message::PollBeats => {
                // Tick the fretboard animation regardless of new beats
                self.tab_fretboard.tick();

                if let Some(ref receiver) = self.beat_receiver {
                    let mut beats = receiver.lock().unwrap();
                    if let Some(&last) = beats.last() {
                        beats.clear();
                        drop(beats);
                        return self.update(Message::OnBeat(last));
                    }
                }
                iced::Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.beat_receiver.is_some() {
            // Poll at 60fps for new beat callbacks from the audio thread.
            // Can't capture in map(), so we use a simple tick and let
            // the update handler drain the receiver.
            iced::time::every(std::time::Duration::from_millis(16))
                .map(|_| Message::PollBeats)
        } else {
            Subscription::none()
        }
    }

    pub fn restore(&mut self, settings: &super::settings::Settings) {
        self.tempo_percent = settings.tab_tempo_percent;
        self.guitar_volume = settings.tab_guitar_volume;
        self.metronome_volume = settings.tab_metronome_volume;
        self.metronome_enabled = settings.tab_metronome_enabled;
        self.tone_index = settings.tab_guitar_tone;
    }

    pub fn save(&self, settings: &mut super::settings::Settings) {
        settings.tab_tempo_percent = self.tempo_percent;
        settings.tab_guitar_volume = self.guitar_volume;
        settings.tab_metronome_volume = self.metronome_volume;
        settings.tab_metronome_enabled = self.metronome_enabled;
        settings.tab_guitar_tone = self.tone_index;
    }

    fn load_file(&mut self, path: &std::path::Path) {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let result: Result<(TabScore, usize), String> = match extension.as_str() {
            "gp" => gp7_parser::parse_file(path).map_err(|error| error.to_string()),
            _ => gp5_parser::parse_file(path).map_err(|error| error.to_string()),
        };

        if let Ok((score, default_track)) = result {
            self.selected_track = default_track;
            self.file_path = Some(path.to_path_buf());
            self.current_beat = 0;
            self.tab_strip.set_score(&score);
            self.tab_fretboard.clear_notes();
            self.score = Some(score);
            self.ensure_audio_thread();
            self.rebuild_timeline();
        }
    }

    fn ensure_audio_thread(&mut self) {
        if self.audio_thread.is_some() {
            return;
        }

        let sonivox = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/soundfonts/sonivox.sf2");
        let metronome_sf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/soundfonts/metronome_clicks.sf2");

        let fired = Arc::new(Mutex::new(Vec::<usize>::new()));
        let fired_ref = fired.clone();
        let callback = BeatCallback::new(move |beat_index, _notes| {
            fired_ref.lock().unwrap().push(beat_index);
        });

        self.beat_receiver = Some(fired);
        self.audio_thread = TabAudioThread::new(sonivox, metronome_sf, callback);
    }

    fn rebuild_timeline(&mut self) {
        let score = match self.score.as_ref() {
            Some(score) => score,
            None => return,
        };
        let timeline = tab_midi::build_timeline(
            score,
            self.selected_track,
            self.tempo_percent,
            self.metronome_enabled,
        );
        if let Some(ref audio) = self.audio_thread {
            audio.send(TabAudioCommand::SetTimeline(timeline));
        }
    }

    fn navigate_bar(&mut self, direction: i32) {
        let score = match self.score.as_ref() {
            Some(score) => score,
            None => return,
        };
        let current_bar = score
            .beats
            .get(self.current_beat)
            .map(|beat| beat.bar_index)
            .unwrap_or(0);
        let target_bar =
            (current_bar as i32 + direction).clamp(0, score.bars.len() as i32 - 1) as usize;
        let target_beat = score.bars[target_bar].first_beat_index;
        if let Some(ref audio) = self.audio_thread {
            audio.send(TabAudioCommand::SeekToBeat(target_beat));
        }
        self.current_beat = target_beat;
    }

    fn reload_for_track(&mut self) {
        if let Some(ref path) = self.file_path.clone() {
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_lowercase();
            let result: Result<(TabScore, usize), String> = match extension.as_str() {
                "gp" => gp7_parser::parse_file_for_track(path, self.selected_track)
                    .map_err(|error| error.to_string()),
                _ => gp5_parser::parse_file_for_track(path, self.selected_track)
                    .map(|score| (score, self.selected_track))
                    .map_err(|error| error.to_string()),
            };
            if let Ok((score, _)) = result {
                self.score = Some(score);
                self.rebuild_timeline();
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let title = match self.score.as_ref() {
            Some(score) if !score.title.is_empty() => {
                if !score.artist.is_empty() {
                    format!("{} — {}", score.title, score.artist)
                } else {
                    score.title.clone()
                }
            }
            _ => "No file loaded".to_string(),
        };

        let header = row![
            button(text("Open").size(14))
                .on_press(Message::OpenFile)
                .padding([6, 12]),
            text(title).size(16),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .padding([8, 12]);

        let transport = row![
            button(text("|<").size(14))
                .on_press(Message::SeekToBeat(0))
                .width(36),
            button(text("<<").size(14))
                .on_press(Message::PrevBar)
                .width(36),
            button(text("Stop").size(14))
                .on_press(Message::Stop)
                .width(48),
            if self.is_playing {
                button(text("Pause").size(14)).on_press(Message::Pause)
            } else {
                button(text("Play").size(14)).on_press(Message::Play)
            }
            .width(56),
            button(text(">>").size(14))
                .on_press(Message::NextBar)
                .width(36),
            button(text(">|").size(14))
                .on_press(Message::SeekToBeat(
                    self.score
                        .as_ref()
                        .and_then(|s| s.bars.last())
                        .map(|b| b.first_beat_index)
                        .unwrap_or(0),
                ))
                .width(36),
            Space::new().width(12),
            button(text(if self.loop_active { "Loop ON" } else { "Loop" }).size(12))
                .on_press(Message::ToggleLoop)
                .padding([4, 8]),
            button(text(if self.metronome_enabled { "Met ON" } else { "Met" }).size(12))
                .on_press(Message::ToggleMetronome)
                .padding([4, 8]),
            button(text(TONES[self.tone_index]).size(12))
                .on_press(Message::CycleTone)
                .padding([4, 8]),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .padding([4, 12]);

        let tempo_row = row![
            text("Tempo").size(12),
            slider(25.0..=200.0, self.tempo_percent, Message::SetTempo).width(Length::Fill),
            text(format!("{}%", self.tempo_percent as u32)).size(12).width(40),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([0, 12]);

        let volume_row = row![
            text("Guitar").size(11),
            slider(0.0..=127.0, self.guitar_volume as f64, Message::SetGuitarVolume)
                .width(Length::Fill),
            text("Met").size(11),
            slider(
                0.0..=127.0,
                self.metronome_volume as f64,
                Message::SetMetronomeVolume,
            )
            .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([0, 12]);

        let beat_info = if let Some(ref score) = self.score {
            let bar_index = score
                .beats
                .get(self.current_beat)
                .map(|b| b.bar_index + 1)
                .unwrap_or(0);
            text(format!(
                "Bar {}/{} — Beat {}/{}",
                bar_index,
                score.bars.len(),
                self.current_beat + 1,
                score.beats.len(),
            ))
            .size(13)
        } else {
            text("Open a GuitarPro file to begin").size(13)
        };

        let tab_strip_view = canvas(&self.tab_strip)
            .width(self.tab_strip.total_width().max(400.0))
            .height(self.tab_strip.content_height());

        let tab_fretboard_view = canvas(&self.tab_fretboard)
            .width(Length::Fill)
            .height(TabFretboardCanvas::content_height());

        let content = column![
            header,
            transport,
            tempo_row,
            volume_row,
            Space::new().height(4),
            container(beat_info).padding([4, 12]),
            Space::new().height(4),
            tab_strip_view,
            Space::new().height(4),
            tab_fretboard_view,
        ];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_page() -> TabPlayerPage {
        TabPlayerPage::new()
    }

    #[test]
    fn test_initial_state() {
        let page = make_page();
        assert!(!page.is_playing);
        assert_eq!(page.current_beat, 0);
        assert!(page.score.is_none());
        assert_eq!(page.tempo_percent, 100.0);
    }

    #[test]
    fn test_play_without_score_is_noop() {
        let mut page = make_page();
        page.update(Message::Play);
        assert!(!page.is_playing);
    }

    #[test]
    fn test_stop_resets_state() {
        let mut page = make_page();
        page.is_playing = true;
        page.current_beat = 42;
        page.update(Message::Stop);
        assert!(!page.is_playing);
        assert_eq!(page.current_beat, 0);
    }

    #[test]
    fn test_tempo_clamped() {
        let mut page = make_page();
        page.update(Message::SetTempo(0.0));
        assert_eq!(page.tempo_percent, 25.0);
        page.update(Message::SetTempo(999.0));
        assert_eq!(page.tempo_percent, 200.0);
    }

    #[test]
    fn test_volume_clamped() {
        let mut page = make_page();
        page.update(Message::SetGuitarVolume(200.0));
        assert_eq!(page.guitar_volume, 127);
    }

    #[test]
    fn test_toggle_loop() {
        let mut page = make_page();
        assert!(!page.loop_active);
        page.update(Message::ToggleLoop);
        assert!(page.loop_active);
        page.update(Message::ToggleLoop);
        assert!(!page.loop_active);
    }

    #[test]
    fn test_toggle_metronome() {
        let mut page = make_page();
        assert!(!page.metronome_enabled);
        page.update(Message::ToggleMetronome);
        assert!(page.metronome_enabled);
    }

    #[test]
    fn test_cycle_tone() {
        let mut page = make_page();
        assert_eq!(page.tone_index, 0);
        page.update(Message::CycleTone);
        assert_eq!(page.tone_index, 1);
        page.update(Message::CycleTone);
        assert_eq!(page.tone_index, 2);
        page.update(Message::CycleTone);
        assert_eq!(page.tone_index, 0);
    }

    #[test]
    fn test_on_beat_updates_position() {
        let mut page = make_page();
        page.update(Message::OnBeat(42));
        assert_eq!(page.current_beat, 42);
    }

    #[test]
    fn test_on_beat_max_is_noop() {
        let mut page = make_page();
        page.current_beat = 5;
        page.update(Message::OnBeat(usize::MAX));
        assert_eq!(page.current_beat, usize::MAX); // still updates — but no score to trigger end-of-song
    }
}
