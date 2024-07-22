mod controller;
pub mod midi;
mod model;

use controller::{interface_controller::*, message::InterfaceMessage};
use image::load_from_memory;
use midi::{get_midi_conn, get_midi_output};
use model::{
    channels::{Bus, Channel},
    flow_8_controller::{FLOW8Controller, InputParams},
};

use iced::{
    executor, settings,
    widget::{Column, Row},
    window::{self, icon::from_rgba, Position},
    Application, Command, Element, Font, Pixels, Size,
};

static ICON: &[u8] = include_bytes!("../resources/flow_32x32.ico");
const ICON_HEIGHT: u32 = 32;
const ICON_WIDTH: u32 = 32;
const WINDOW_WIDTH: f32 = 1050.0;
const WINDOW_HEIGHT: f32 = 600.0;

impl Application for FLOW8Controller {
    type Flags = InputParams;
    type Theme = iced::Theme;
    type Message = InterfaceMessage;
    type Executor = executor::Default;

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            FLOW8Controller::new(_flags.midi_out),
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("FLOW 8 Controller")
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    fn update(&mut self, message: InterfaceMessage) -> iced::Command<Self::Message> {
        match_midi_command(message, self.midi_out);
        Command::none()
    }

    fn view(&self) -> Element<InterfaceMessage> {
        Row::with_children(self.channels.iter().map(|c: &Channel| {
            let mut column = Column::new().width(CHANNEL_STRIP_WIDTH);
            column = add_channel(column, c);
            finalize_column(column).into()
        }))
        .extend(self.buses.iter().map(|b: &Bus| {
            let mut column = Column::new().width(CHANNEL_STRIP_WIDTH);
            column = add_bus(column, b);
            finalize_column(column).into()
        }))
        .into()
    }
}

fn main() {
    let image = load_from_memory(ICON).unwrap();
    let icon = from_rgba(image.as_bytes().to_vec(), ICON_HEIGHT, ICON_WIDTH).unwrap();

    let flags = InputParams {
        midi_out: get_midi_output(),
    };

    let settings = settings::Settings {
        flags: flags,
        window: window::Settings {
            size: Size {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            position: Position::Centered,
            icon: Some(icon),
            ..Default::default()
        },
        id: None,
        fonts: Vec::new(),
        antialiasing: true,
        default_font: Font::default(),
        default_text_size: Pixels(16.0),
    };
    match FLOW8Controller::run(settings) {
        Ok(_) => (),
        Err(err) => println!("Something went wrong: {}.", err),
    }
}
