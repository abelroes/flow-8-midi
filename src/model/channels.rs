pub type ChannelId = u8;

#[derive(Copy, Clone, Debug)]
pub enum AudioConnection {
    Xlr,
    Line,
    UsbBt,
    ComboXlr,
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelType {
    Mono,
    Stereo,
}

#[derive(Copy, Clone, Debug)]
pub enum PhantomPower {
    None,
    Set48v(u8),
}

#[derive(Copy, Clone, Debug)]
pub struct Channel {
    pub id: ChannelId,
    pub phantom_pwr: PhantomPower,
    pub channel_type: ChannelType,
    pub audio_connection: AudioConnection,
    pub channel_strip: BasicChannelStrip,
    pub four_band_eq: FourBandEQ,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            id:0,
            phantom_pwr: PhantomPower::None,
            channel_type: ChannelType::Mono,
            audio_connection:AudioConnection::UsbBt,
            channel_strip:BasicChannelStrip::default(),
            four_band_eq:FourBandEQ::default(),
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
            gain:64,
            level:64,
            balance:64,
            mute:0,
            solo:0,
            compressor:0,
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
            low:64,
            low_mid:64,
            hi_mid:64,
            hi:64,
        }
    }
}