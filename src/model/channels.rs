use core::fmt;

pub type BusId = u8;
pub type BusIdx = u8;
pub type ChannelId = u8;

#[derive(Copy, Clone, Debug)]
pub enum AudioConnection {
    Xlr,
    Line,
    UsbBt,
    ComboXlr,
}

impl fmt::Display for AudioConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match *self {
            AudioConnection::Xlr => "XLR",
            AudioConnection::Line => "Line",
            AudioConnection::UsbBt => "USB/BT",
            AudioConnection::ComboXlr => "Combo",
        };
        write!(f, "{}", text)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelType {
    Mono,
    Stereo,
}

#[derive(Copy, Clone, Debug)]
pub enum BusType {
    Fx,
    Main,
    Monitor,
}

impl fmt::Display for BusType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match *self {
            BusType::Fx => "FX",
            BusType::Main => "Main",
            BusType::Monitor => "Monitor",
        };
        write!(f, "{}", text)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PhantomPower {
    pub is_on: bool,
    pub is_confirmed: bool,
    pub phantom_power_type: PhantomPowerType,
}

#[derive(Copy, Clone, Debug)]
pub enum PhantomPowerType {
    None,
    Set48v(u8),
}

#[derive(Copy, Clone, Debug)]
pub struct Channel {
    pub id: ChannelId,
    pub is_muted: bool,
    pub is_soloed: bool,
    pub phantom_pwr: PhantomPower,
    pub channel_type: ChannelType,
    pub audio_connection: AudioConnection,
    pub channel_strip: BasicChannelStrip,
    pub four_band_eq: FourBandEQ,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            id: 0,
            is_muted: false,
            is_soloed: false,
            phantom_pwr: PhantomPower {
                is_on: false,
                is_confirmed:false,
                phantom_power_type: PhantomPowerType::None,
            },
            channel_type: ChannelType::Mono,
            audio_connection: AudioConnection::UsbBt,
            channel_strip: BasicChannelStrip::default(),
            four_band_eq: FourBandEQ::default(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Bus {
    pub id: BusId,
    pub index: BusIdx,
    pub bus_type: BusType,
    pub bus_strip: BusStrip,
    pub nine_band_eq: NineBandEQ,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            id: 7,
            index: 0,
            bus_type: BusType::Main,
            bus_strip: BusStrip::default(),
            nine_band_eq: NineBandEQ::default(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BasicChannelStrip {
    pub gain: u8,
    pub level: u8,
    pub balance: u8,
    pub mute: u8,
    pub solo: u8,
    pub compressor: u8,
}

impl Default for BasicChannelStrip {
    fn default() -> Self {
        Self {
            gain: 64,
            level: 64,
            balance: 64,
            mute: 0,
            solo: 0,
            compressor: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BusStrip {
    pub level: u8,
    pub balance: u8,
    pub limiter: u8,
}

impl Default for BusStrip {
    fn default() -> Self {
        Self {
            level: 64,
            balance: 64,
            limiter: 127,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FourBandEQ {
    pub low: u8,
    pub low_mid: u8,
    pub hi_mid: u8,
    pub hi: u8,
}

impl Default for FourBandEQ {
    fn default() -> Self {
        Self {
            low: 64,
            low_mid: 64,
            hi_mid: 64,
            hi: 64,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NineBandEQ {
    pub freq_62_hz: u8,
    pub freq_125_hz: u8,
    pub freq_250_hz: u8,
    pub freq_500_hz: u8,
    pub freq_1_khz: u8,
    pub freq_2_khz: u8,
    pub freq_4_khz: u8,
    pub freq_8_khz: u8,
    pub freq_16_khz: u8,
}

impl Default for NineBandEQ {
    fn default() -> Self {
        Self {
            freq_62_hz: 64,
            freq_125_hz: 64,
            freq_250_hz: 64,
            freq_500_hz: 64,
            freq_1_khz: 64,
            freq_2_khz: 64,
            freq_4_khz: 64,
            freq_8_khz: 64,
            freq_16_khz: 64,
        }
    }
}
