use std::ops::RangeInclusive;

use midir::MidiOutputConnection;

use super::channels::{
    AudioConnection, Bus, BusType, Channel, ChannelType, PhantomPower, PhantomPowerType,
};

pub struct FLOW8Controller {
    pub buses: Vec<Bus>,
    pub midi_conn: MidiOutputConnection,
    pub channels: Vec<Channel>,
}

const CHANNEL_RANGE: RangeInclusive<u8> = 0..=6;
const BUS_RANGE: RangeInclusive<u8> = 7..=8;

impl FLOW8Controller {
    pub fn new(midi_conn: MidiOutputConnection) -> FLOW8Controller {
        FLOW8Controller {
            midi_conn,
            channels: CHANNEL_RANGE
                .map(|c_id| Channel {
                    id: c_id,
                    phantom_pwr: PhantomPower {
                        is_on: false,
                        is_confirmed: false,
                        phantom_power_type: match c_id {
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
            buses: BUS_RANGE
                .map(|b_id| Bus {
                    id: b_id,
                    index: b_id / BUS_RANGE.max().unwrap(),
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
