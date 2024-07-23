mod controller;
pub mod midi;
mod model;
mod utils;

use controller::{
    interface_controller::{
        add_bus, add_channel, finalize_column, match_midi_command, CHANNEL_STRIP_WIDTH,
    },
    message::InterfaceMessage,
};
use image::load_from_memory;
use midi::get_midi_conn;
use model::{
    channels::{Bus, Channel},
    flow_8_controller::FLOW8Controller,
};

use iced::{
    executor,
    widget::{Column, Row},
    window::{self, icon::from_rgba, Position},
    Application, Command, Element, Font, Pixels, Settings, Size,
};

const APPLICATION_NAME: &str = "FLOW 8 MIDI Controller";
const ICON_HEIGHT: u32 = 32;
const ICON_WIDTH: u32 = 32;
const WINDOW_WIDTH: f32 = 1050.0;
const WINDOW_HEIGHT: f32 = 600.0;
static ICON: &[u8] = include_bytes!("../resources/flow_32x32.ico");

impl Application for FLOW8Controller {
    type Flags = ();
    type Theme = iced::Theme;
    type Message = InterfaceMessage;
    type Executor = executor::Default;

    fn new(mut _flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (FLOW8Controller::new(get_midi_conn()), Command::none())
    }

    fn title(&self) -> String {
        String::from(APPLICATION_NAME)
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    fn update(&mut self, message: InterfaceMessage) -> iced::Command<Self::Message> {
        update_interface(self, message);
        match_midi_command(message, &mut self.midi_conn);
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

pub fn update_interface(controller: &mut FLOW8Controller, message: InterfaceMessage) {
    match message {
        InterfaceMessage::Mute(chn_id, _) => {
            controller.channels[chn_id as usize].is_muted =
                !controller.channels[chn_id as usize].is_muted
        }
        InterfaceMessage::Solo(chn_id, _) => {
            controller.channels[chn_id as usize].is_soloed =
                !controller.channels[chn_id as usize].is_soloed
        }
        InterfaceMessage::Gain(chn_id, value) => {
            controller.channels[chn_id as usize].channel_strip.gain = value
        }
        InterfaceMessage::Level(chn_id, value) => {
            controller.channels[chn_id as usize].channel_strip.level = value
        }
        InterfaceMessage::Balance(chn_id, value) => {
            controller.channels[chn_id as usize].channel_strip.balance = value
        }
        InterfaceMessage::PhantomPower(chn_id, _) => {
            controller.channels[chn_id as usize].phantom_pwr.is_on =
                !controller.channels[chn_id as usize].phantom_pwr.is_on;
        }
        InterfaceMessage::Compressor(chn_id, value) => {
            controller.channels[chn_id as usize]
                .channel_strip
                .compressor = value
        }
        InterfaceMessage::EqLow(chn_id, value) => {
            controller.channels[chn_id as usize].four_band_eq.low = value
        }
        InterfaceMessage::EqLowMid(chn_id, value) => {
            controller.channels[chn_id as usize].four_band_eq.low_mid = value
        }
        InterfaceMessage::EqHiMid(chn_id, value) => {
            controller.channels[chn_id as usize].four_band_eq.hi_mid = value
        }
        InterfaceMessage::EqHi(chn_id, value) => {
            controller.channels[chn_id as usize].four_band_eq.hi = value
        }
        InterfaceMessage::BusLevel(bus_idx, _, value) => {
            controller.buses[bus_idx as usize].bus_strip.level = value
        }
        InterfaceMessage::BusBalance(bus_idx, _, value) => {
            controller.buses[bus_idx as usize].bus_strip.balance = value
        }
    }
}

fn main() {
    let image = load_from_memory(ICON).unwrap();
    let icon = from_rgba(image.as_bytes().to_vec(), ICON_HEIGHT, ICON_WIDTH).unwrap();

    let settings = Settings {
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
        ..Settings::default()
    };
    match FLOW8Controller::run(settings) {
        Ok(_) => (),
        Err(err) => println!("Something went wrong: {}.", err),
    }
}
