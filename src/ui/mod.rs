pub mod audio;
mod metronome_page;
mod scales_page;

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
}

pub struct App {
    page: Page,
    metronome: metronome_page::MetronomePage,
    scales: scales_page::ScalesPage,
}

impl App {
    pub fn new() -> Self {
        Self {
            page: Page::Metronome,
            metronome: metronome_page::MetronomePage::new(),
            scales: scales_page::ScalesPage::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::SwitchPage(page) => {
                self.page = page;
                iced::Task::none()
            }
            Message::Metronome(msg) => self.metronome.update(msg).map(Message::Metronome),
            Message::Scales(msg) => self.scales.update(msg).map(Message::Scales),
        }
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
            Page::TabPlayer => placeholder("Tab Player"),
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
        self.metronome.subscription().map(Message::Metronome)
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
