pub mod audio;
mod fretboard_canvas;
mod metronome_page;
mod scales_page;
pub mod settings;
mod tab_fretboard_canvas;
mod tab_player_page;
mod tab_strip_canvas;

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Subscription, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Metronome,
    Scales,
    TabPlayer,
}

#[derive(Debug, Clone)]
pub enum Message {
    SwitchPage(Page),
    Metronome(metronome_page::Message),
    Scales(scales_page::Message),
    TabPlayer(tab_player_page::Message),
}

pub struct App {
    page: Page,
    metronome: metronome_page::MetronomePage,
    scales: scales_page::ScalesPage,
    tab_player: tab_player_page::TabPlayerPage,
    settings: settings::Settings,
}

impl App {
    pub fn new() -> Self {
        let settings = settings::Settings::load();
        let mut metronome = metronome_page::MetronomePage::new();
        metronome.restore(&settings);
        let mut scales = scales_page::ScalesPage::new();
        scales.restore(&settings);
        let mut tab_player = tab_player_page::TabPlayerPage::new();
        tab_player.restore(&settings);
        Self {
            page: Page::Metronome,
            metronome,
            scales,
            tab_player,
            settings,
        }
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        // Only persist on user-initiated changes, not on tick/poll messages
        let should_persist = !matches!(
            message,
            Message::Metronome(metronome_page::Message::Tick)
                | Message::TabPlayer(tab_player_page::Message::PollBeats)
                | Message::TabPlayer(tab_player_page::Message::OnBeat(_))
        );

        let task = match message {
            Message::SwitchPage(page) => {
                self.page = page;
                iced::Task::none()
            }
            Message::Metronome(msg) => self.metronome.update(msg).map(Message::Metronome),
            Message::Scales(msg) => self.scales.update(msg).map(Message::Scales),
            Message::TabPlayer(msg) => self.tab_player.update(msg).map(Message::TabPlayer),
        };

        if should_persist {
            self.metronome.save(&mut self.settings);
            self.scales.save(&mut self.settings);
            self.tab_player.save(&mut self.settings);
            self.settings.save();
        }
        task
    }

    pub fn view(&self) -> Element<Message> {
        let nav = row![
            tab_button("Metronome", Page::Metronome, self.page),
            tab_button("Scales", Page::Scales, self.page),
            tab_button("Tab Player", Page::TabPlayer, self.page),
        ]
        .spacing(4)
        .padding(8);

        let content: Element<Message> = match self.page {
            Page::Metronome => self.metronome.view().map(Message::Metronome),
            Page::Scales => self.scales.view().map(Message::Scales),
            Page::TabPlayer => self.tab_player.view().map(Message::TabPlayer),
        };

        let page = column![nav, content].spacing(0);

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let metronome_sub = self.metronome.subscription().map(Message::Metronome);
        let tab_sub = self.tab_player.subscription().map(Message::TabPlayer);
        Subscription::batch([metronome_sub, tab_sub])
    }
}

fn tab_button(label: &str, target: Page, current: Page) -> Element<Message> {
    let style = if target == current {
        button::primary
    } else {
        button::secondary
    };
    button(text(label).size(14))
        .style(style)
        .on_press(Message::SwitchPage(target))
        .into()
}

fn placeholder(label: &str) -> Element<'_, Message> {
    container(text(label).size(24))
        .center(Length::Fill)
        .into()
}
