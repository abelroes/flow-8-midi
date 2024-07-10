mod controller;
mod model;

use controller::{interface_controller::*, message::InterfaceMessage};
use image::load_from_memory;
use model::channels::{AudioConnection, Channel, ChannelType, PhantomPower};

use iced::{
    settings,
    widget::{Column, Row},
    window::{self, icon::from_rgba, Position},
    Element, Sandbox, Size,
};

static ICON: &[u8] = include_bytes!("../resources/flow_32x32.ico");
const ICON_HEIGHT: u32 = 32;
const ICON_WIDTH: u32 = 32;

#[derive(Debug)]
struct FLOW8Controller {
    channels: Vec<Channel>,
}

impl FLOW8Controller {
    fn new() -> FLOW8Controller {
        FLOW8Controller {
            channels: (0..=6)
                .map(|c_id| Channel {
                    id: c_id,
                    phantom_pwr: {
                        match c_id {
                            0..=1 => PhantomPower::Set48v(0),
                            _ => PhantomPower::None,
                        }
                    },
                    channel_type: {
                        match c_id {
                            0..=3 => ChannelType::Mono,
                            _ => ChannelType::Stereo,
                        }
                    },
                    audio_connection: {
                        match c_id {
                            0..=1 => AudioConnection::Xlr,
                            2..=3 => AudioConnection::ComboXlr,
                            4..=5 => AudioConnection::Line,
                            _ => AudioConnection::UsbBt,
                        }
                    },
                    ..Default::default()
                })
                .collect(),
        }
    }
}

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

            column = add_channel_name(column, c);
            column = add_mute_solo(column, c);
            column = add_phantom(column, c);
            column = add_vertical_slider(column, c);

            finalize_column(column).into()
        }))
        .into()
    }
}

fn main() {
    let image = load_from_memory(ICON).unwrap();
    let icon = from_rgba(image.as_bytes().to_vec(), ICON_HEIGHT, ICON_WIDTH).unwrap();

    let settings = settings::Settings {
        window: window::Settings {
            size: Size { width: 1000.0, height: 600.0 },
            position: Position::Centered,
            icon: Some(icon),
            ..Default::default()
        },
        ..Default::default()
    };
    match FLOW8Controller::run(settings) {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}
