use midir::MidiOutput;

use super::channels::{
    AudioConnection, Bus, BusType, Channel, ChannelType, PhantomPower, PhantomPowerType,
};

pub struct InputParams {
    pub midi_out: Option<MidiOutput>,
}

pub struct FLOW8Controller {
    pub buses: Vec<Bus>,
    pub midi_out: Option<MidiOutput>,
    pub channels: Vec<Channel>,
}

impl FLOW8Controller {
    pub fn new(midi_out: Option<MidiOutput>) -> FLOW8Controller {
        FLOW8Controller {
            midi_out,
            channels: (0..=6)
                .map(|c_id| Channel {
                    id: c_id,
                    phantom_pwr: PhantomPower {
                        is_on: false,
                        phanton_power_type: match c_id {
                            0..=1 => PhantomPowerType::Set48v(0),
                            _ => PhantomPowerType::None,
                        },
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
            buses: (7..=8)
                .map(|b_id| Bus {
                    id: b_id,
                    bus_type: {
                        match b_id {
                            7 => BusType::Main,
                            _ => BusType::Monitor,
                        }
                    },
                    ..Default::default()
                })
                .collect(),
        }
    }
}
