use super::audio::ClickPlayer;
use iced::widget::{button, column, container, row, slider, text, Space};
use iced::{time, Element, Length, Subscription};
use std::time::{Duration, Instant};

const BPM_MIN: u32 = 20;
const BPM_MAX: u32 = 260;
const BPB_MIN: u8 = 1;
const BPB_MAX: u8 = 99;

#[derive(Debug, Clone)]
pub enum Message {
    TogglePlay,
    SetBpm(u32),
    IncrementBpm,
    DecrementBpm,
    IncrementBpb,
    DecrementBpb,
    SetVolume(f32),
    Tap,
    Tick,
}

pub struct MetronomePage {
    bpm: u32,
    beats_per_bar: u8,
    volume: f32,
    is_playing: bool,
    current_beat: u32,
    tap_times: Vec<Instant>,
    click_player: Option<ClickPlayer>,
}

impl MetronomePage {
    pub fn new() -> Self {
        Self {
            bpm: 100,
            beats_per_bar: 4,
            volume: 1.0,
            is_playing: false,
            current_beat: 0,
            tap_times: Vec::new(),
            click_player: ClickPlayer::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::TogglePlay => {
                self.is_playing = !self.is_playing;
                if !self.is_playing {
                    self.current_beat = 0;
                }
                iced::Task::none()
            }
            Message::Tick => {
                if self.is_playing {
                    self.current_beat = (self.current_beat + 1) % self.beats_per_bar as u32;
                    if let Some(ref player) = self.click_player {
                        player.click(self.current_beat == 0);
                    }
                }
                iced::Task::none()
            }
            Message::SetBpm(bpm) => {
                self.bpm = bpm.clamp(BPM_MIN, BPM_MAX);
                iced::Task::none()
            }
            Message::IncrementBpm => {
                self.bpm = (self.bpm + 1).min(BPM_MAX);
                iced::Task::none()
            }
            Message::DecrementBpm => {
                self.bpm = self.bpm.saturating_sub(1).max(BPM_MIN);
                iced::Task::none()
            }
            Message::IncrementBpb => {
                if self.beats_per_bar < BPB_MAX {
                    self.beats_per_bar += 1;
                }
                iced::Task::none()
            }
            Message::DecrementBpb => {
                if self.beats_per_bar > BPB_MIN {
                    self.beats_per_bar -= 1;
                }
                iced::Task::none()
            }
            Message::SetVolume(volume) => {
                self.volume = volume.clamp(0.0, 1.0);
                if let Some(ref mut player) = self.click_player {
                    player.set_volume(self.volume);
                }
                iced::Task::none()
            }
            Message::Tap => {
                let now = Instant::now();
                self.tap_times
                    .retain(|t| now.duration_since(*t) < Duration::from_secs(3));
                self.tap_times.push(now);
                if self.tap_times.len() >= 2 {
                    let intervals: Vec<f64> = self
                        .tap_times
                        .windows(2)
                        .map(|pair| pair[1].duration_since(pair[0]).as_secs_f64())
                        .collect();
                    let avg = intervals.iter().sum::<f64>() / intervals.len() as f64;
                    let tapped_bpm = (60.0 / avg).round() as u32;
                    self.bpm = tapped_bpm.clamp(BPM_MIN, BPM_MAX);
                }
                iced::Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.is_playing {
            let interval = Duration::from_secs_f64(60.0 / self.bpm as f64);
            time::every(interval).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    pub fn view(&self) -> Element<Message> {
        let beat_indicator = beat_display(self.current_beat, self.beats_per_bar, self.is_playing);

        let bpm_display = text(format!("{}", self.bpm)).size(72);

        let bpm_controls = row![
            button(text("-").size(20))
                .on_press(Message::DecrementBpm)
                .width(40),
            slider(BPM_MIN as f64..=BPM_MAX as f64, self.bpm as f64, |value| {
                Message::SetBpm(value as u32)
            }),
            button(text("+").size(20))
                .on_press(Message::IncrementBpm)
                .width(40),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([0, 16]);

        let bpb_controls = row![
            button(text("-").size(16))
                .on_press(Message::DecrementBpb)
                .width(36),
            text(format!("{} beats/bar", self.beats_per_bar)).size(14),
            button(text("+").size(16))
                .on_press(Message::IncrementBpb)
                .width(36),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let play_label = if self.is_playing { "Stop" } else { "Play" };
        let play_button = button(text(play_label).size(18))
            .on_press(Message::TogglePlay)
            .padding([12, 32]);

        let tap_button = button(text("TAP").size(16))
            .on_press(Message::Tap)
            .padding([8, 24]);

        let volume_row = row![
            text("Vol").size(12),
            slider(0.0..=1.0, self.volume as f64, |value| {
                Message::SetVolume(value as f32)
            })
            .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([0, 16]);

        let content = column![
            Space::new().height(24),
            beat_indicator,
            Space::new().height(16),
            container(bpm_display).center_x(Length::Fill),
            Space::new().height(8),
            bpm_controls,
            Space::new().height(12),
            container(bpb_controls).center_x(Length::Fill),
            Space::new().height(24),
            container(row![play_button, tap_button].spacing(12)).center_x(Length::Fill),
            Space::new().height(16),
            volume_row,
        ]
        .align_x(iced::Alignment::Center);

        container(content)
            .width(Length::Fill)
            .max_width(480)
            .center_x(Length::Fill)
            .into()
    }
}

fn beat_display<'a>(
    current_beat: u32,
    beats_per_bar: u8,
    is_playing: bool,
) -> Element<'a, Message> {
    let dots: Vec<Element<Message>> = (0..beats_per_bar as u32)
        .map(|beat_index| {
            let is_active = is_playing && beat_index == current_beat;
            let is_downbeat = beat_index == 0;
            let label = if is_active {
                if is_downbeat {
                    "O"
                } else {
                    "o"
                }
            } else {
                "."
            };
            let size = if is_downbeat { 28.0 } else { 20.0 };
            text(label).size(size).into()
        })
        .collect();
    container(row(dots).spacing(8))
        .center_x(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_page() -> MetronomePage {
        // Tests don't need audio — create without the click player
        MetronomePage {
            bpm: 100,
            beats_per_bar: 4,
            volume: 1.0,
            is_playing: false,
            current_beat: 0,
            tap_times: Vec::new(),
            click_player: None,
        }
    }

    #[test]
    fn test_initial_state() {
        let page = make_page();
        assert_eq!(page.bpm, 100);
        assert_eq!(page.beats_per_bar, 4);
        assert!(!page.is_playing);
        assert_eq!(page.current_beat, 0);
    }

    #[test]
    fn test_set_bpm() {
        let mut page = make_page();
        page.update(Message::SetBpm(120));
        assert_eq!(page.bpm, 120);
    }

    #[test]
    fn test_bpm_clamped_to_range() {
        let mut page = make_page();
        page.update(Message::SetBpm(0));
        assert_eq!(page.bpm, BPM_MIN);
        page.update(Message::SetBpm(999));
        assert_eq!(page.bpm, BPM_MAX);
    }

    #[test]
    fn test_increment_decrement_bpm() {
        let mut page = make_page();
        page.update(Message::IncrementBpm);
        assert_eq!(page.bpm, 101);
        page.update(Message::DecrementBpm);
        assert_eq!(page.bpm, 100);
    }

    #[test]
    fn test_decrement_bpm_at_minimum() {
        let mut page = make_page();
        page.update(Message::SetBpm(BPM_MIN));
        page.update(Message::DecrementBpm);
        assert_eq!(page.bpm, BPM_MIN);
    }

    #[test]
    fn test_increment_decrement_bpb() {
        let mut page = make_page();
        assert_eq!(page.beats_per_bar, 4);
        page.update(Message::IncrementBpb);
        assert_eq!(page.beats_per_bar, 5);
        page.update(Message::DecrementBpb);
        assert_eq!(page.beats_per_bar, 4);
    }

    #[test]
    fn test_bpb_clamped_to_range() {
        let mut page = make_page();
        for _ in 0..200 {
            page.update(Message::IncrementBpb);
        }
        assert_eq!(page.beats_per_bar, BPB_MAX);
        for _ in 0..200 {
            page.update(Message::DecrementBpb);
        }
        assert_eq!(page.beats_per_bar, BPB_MIN);
    }

    #[test]
    fn test_volume() {
        let mut page = make_page();
        page.update(Message::SetVolume(0.5));
        assert!((page.volume - 0.5).abs() < f32::EPSILON);
        page.update(Message::SetVolume(-1.0));
        assert!((page.volume - 0.0).abs() < f32::EPSILON);
        page.update(Message::SetVolume(5.0));
        assert!((page.volume - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_tick_advances_beat() {
        let mut page = make_page();
        page.is_playing = true;
        page.beats_per_bar = 4;
        page.update(Message::Tick);
        assert_eq!(page.current_beat, 1);
        page.update(Message::Tick);
        assert_eq!(page.current_beat, 2);
        page.update(Message::Tick);
        assert_eq!(page.current_beat, 3);
        page.update(Message::Tick);
        assert_eq!(page.current_beat, 0); // wraps
    }

    #[test]
    fn test_tick_does_nothing_when_stopped() {
        let mut page = make_page();
        page.is_playing = false;
        page.update(Message::Tick);
        assert_eq!(page.current_beat, 0);
    }

    #[test]
    fn test_toggle_play() {
        let mut page = make_page();
        assert!(!page.is_playing);
        page.update(Message::TogglePlay);
        assert!(page.is_playing);
        page.update(Message::TogglePlay);
        assert!(!page.is_playing);
    }

    #[test]
    fn test_stop_resets_beat() {
        let mut page = make_page();
        page.is_playing = true;
        page.current_beat = 3;
        page.update(Message::TogglePlay); // stop
        assert_eq!(page.current_beat, 0);
    }

    #[test]
    fn test_tap_tempo_two_taps() {
        let mut page = make_page();
        page.tap_times
            .push(Instant::now() - Duration::from_millis(500));
        page.update(Message::Tap);
        // ~500ms interval = ~120 BPM
        assert!((page.bpm as i32 - 120).abs() <= 2);
    }

    #[test]
    fn test_tap_tempo_single_tap_no_change() {
        let mut page = make_page();
        let original_bpm = page.bpm;
        page.update(Message::Tap);
        assert_eq!(page.bpm, original_bpm);
    }

    #[test]
    fn test_subscription_active_when_playing() {
        let mut page = make_page();
        page.is_playing = true;
        // Just verify it doesn't panic — actual subscription testing
        // requires the Iced runtime
        let _ = page.subscription();
    }

    #[test]
    fn test_subscription_none_when_stopped() {
        let page = make_page();
        let _ = page.subscription();
    }
}
