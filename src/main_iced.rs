mod chord_builder;
mod fluidsynth_ffi;
mod gp5_parser;
mod gp7_parser;
mod recent_files;
mod scale_data;
mod tab_audio_thread;
mod tab_midi;
mod tab_models;

mod ui;

fn main() -> iced::Result {
    pretty_env_logger::init();
    iced::application(ui::App::new, ui::App::update, ui::App::view)
        .title("Metronome")
        .theme(ui::App::theme)
        .subscription(ui::App::subscription)
        .window_size(iced::Size::new(500.0, 700.0))
        .run()
}
