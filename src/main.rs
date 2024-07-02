// mod midi;
mod channels;

// use midi::run;
use channels::{ChannelId, AudioConnection, Channel, ChannelType};

use iced::widget::{button, column, vertical_slider, Row};
use iced::{Element, Sandbox, Settings};

#[derive(Debug)]
struct FLOW8Controller {
    channels: Vec<Channel>,
}

impl FLOW8Controller {
    fn new () -> FLOW8Controller{
        FLOW8Controller {
            channels: (0..10)
                .map(|id| Channel {
                    id: id,
                    channel_type: {
                        match id {
                            0..=3 => ChannelType::Mono,
                            4..=8 => ChannelType::Stereo,
                            _ => ChannelType::Stereo,
                        }
                    },
                    audio_connection: {
                        match id {
                            0..=1 => AudioConnection::XLR,
                            2..=3 => AudioConnection::ComboXLR,
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

#[derive(Debug, Clone, Copy)]
enum InterfaceMessage {
    Mute(ChannelId),
    Solo(ChannelId),
    Gain(ChannelId, u8),
    Level(ChannelId, u8),
    Balance(ChannelId, u8),
    Compressor(ChannelId, u8),
    EQ_Low(ChannelId, u8),
    EQ_Low_Mid(ChannelId, u8),
    EQ_Hi_Mid(ChannelId, u8),
    EQ_Hi(ChannelId, u8),
}

impl Sandbox for FLOW8Controller {
    type Message = InterfaceMessage;

    fn new() -> Self {
        FLOW8Controller::new()
    }
    fn title(&self) -> String {
        String::from("FLOW 8 Controller")
    }
    fn update(&mut self, _message: InterfaceMessage) {
        match _message {
            InterfaceMessage::Mute(c) => (),
            InterfaceMessage::Solo(c) => (),
            InterfaceMessage::Gain(c, v) => (),
            InterfaceMessage::Level(c, v) => (),
            InterfaceMessage::Balance(c, v) => (),
            InterfaceMessage::Compressor(c, v) => (),
            InterfaceMessage::EQ_Low(c, v) => (),
            InterfaceMessage::EQ_Low_Mid(c, v) => (),
            InterfaceMessage::EQ_Hi_Mid(c, v) => (),
            InterfaceMessage::EQ_Hi(c, v) => (),
        }
    }
    fn view(&self) -> Element<InterfaceMessage> {
        let row = Row::new();
        column![
            button("Mute").on_press(InterfaceMessage::Mute),
            vertical_slider(1u8..=127u8, self.volume, |v| InterfaceMessage::Level(v))
        ]
        .into()
    }
    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
        // or
        // iced::Theme::Light
    }
} 

fn main() {
    match FLOW8Controller::run(Settings::default()) {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}
