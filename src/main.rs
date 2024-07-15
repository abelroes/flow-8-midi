mod controller;
pub mod midi;
mod model;

use controller::{interface_controller::*, message::InterfaceMessage};
use image::load_from_memory;
use midi::get_midi_conn;
use model::{
    channels::{Bus, Channel},
    flow_8_controller::FLOW8Controller,
};

use iced::{
    settings,
    widget::{Column, Row},
    window::{self, icon::from_rgba, Position},
    Element, Sandbox, Size,
};

static ICON: &[u8] = include_bytes!("../resources/flow_32x32.ico");
const ICON_HEIGHT: u32 = 32;
const ICON_WIDTH: u32 = 32;

impl Sandbox for FLOW8Controller {
    type Message = InterfaceMessage;

    fn new() -> Self {
        FLOW8Controller::new()
    }

    fn title(&self) -> String {
        String::from("FLOW 8 Controller")
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    fn update(&mut self, _message: InterfaceMessage) {
        match _message {
            InterfaceMessage::Mute(c) => (),
            InterfaceMessage::Solo(c) => (),
            InterfaceMessage::Gain(c, v) => (),
            InterfaceMessage::Level(c, v) => (),
            InterfaceMessage::Balance(c, v) => (),
            InterfaceMessage::PhantomPower(c, v) => (),
            InterfaceMessage::Compressor(c, v) => (),
            InterfaceMessage::EqLow(c, v) => (),
            InterfaceMessage::EqLowMid(c, v) => (),
            InterfaceMessage::EqHiMid(c, v) => (),
            InterfaceMessage::EqHi(c, v) => (),
        }
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

    //Temporary flag for midi connection bypass
    let connection_optional = true;

    let mut midi_conn = get_midi_conn(connection_optional).unwrap();

    let settings = settings::Settings {
        window: window::Settings {
            size: Size {
                width: 1050.0,
                height: 600.0,
            },
            position: Position::Centered,
            icon: Some(icon),
            ..Default::default()
        },
        ..Default::default()
    };
    match FLOW8Controller::run(settings) {
        Ok(_) => (),
        Err(err) => println!("Something went wrong: {}.", err),
    }
}
