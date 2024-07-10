use super::channels::{AudioConnection, Bus, BusType, Channel, ChannelType, PhantomPower};

#[derive(Debug)]
pub struct FLOW8Controller {
    pub channels: Vec<Channel>,
    pub buses: Vec<Bus>,
}

impl FLOW8Controller {
    pub fn new() -> FLOW8Controller {
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
