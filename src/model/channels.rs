use core::fmt;
use std::time::Instant;

pub type BusId = u8;
pub type BusIdx = u8;
pub type ChannelId = u8;
pub type FxSlotId = u8;

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
            AudioConnection::UsbBt => "USB | BT",
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BusType {
    Main,
    Monitor,
    Fx,
}

#[derive(Copy, Clone, Debug)]
pub struct PhantomPower {
    pub is_on: bool,
    pub phantom_power_type: PhantomPowerType,
}

#[derive(Copy, Clone, Debug)]
pub enum PhantomPowerType {
    None,
    Set48v,
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
    pub name_synced: bool,
    pub is_muted: bool,
    pub is_soloed: bool,
    pub mute_synced: bool,
    pub solo_synced: bool,
    pub phantom_pwr: PhantomPower,
    pub phantom_synced: bool,
    pub phantom_last_click: Option<Instant>,
    pub channel_type: ChannelType,
    pub audio_connection: AudioConnection,
    pub channel_strip: BasicChannelStrip,
    pub four_band_eq: FourBandEQ,
    pub sends: ChannelSends,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            name_synced: false,
            is_muted: false,
            is_soloed: false,
            mute_synced: false,
            solo_synced: false,
            phantom_pwr: PhantomPower {
                is_on: false,
                phantom_power_type: PhantomPowerType::None,
            },
            phantom_synced: false,
            phantom_last_click: None,
            channel_type: ChannelType::Mono,
            audio_connection: AudioConnection::UsbBt,
            channel_strip: BasicChannelStrip::default(),
            four_band_eq: FourBandEQ::default(),
            sends: ChannelSends::default(),
        }
    }
}

impl Channel {
    pub fn display_label(&self) -> String {
        if matches!(self.audio_connection, AudioConnection::UsbBt) {
            return format!("{}", self.audio_connection);
        }
        let number = self.physical_number();
        format!("Ch {} · {}", number, self.audio_connection)
    }

    fn physical_number(&self) -> String {
        match self.channel_type {
            ChannelType::Mono => format!("{}", self.id + 1),
            ChannelType::Stereo => {
                let start = (self.id as u16).min(4) + (self.id as u16).saturating_sub(4) * 2 + 1;
                format!("{}/{}", start, start + 1)
            }
        }
    }

    pub fn has_gain(&self) -> bool {
        !matches!(self.audio_connection, AudioConnection::UsbBt)
    }

    pub fn has_compressor(&self) -> bool {
        !matches!(self.audio_connection, AudioConnection::UsbBt)
    }

    pub fn has_low_cut(&self) -> bool {
        !matches!(self.audio_connection, AudioConnection::UsbBt)
    }

    pub fn mark_all_synced(&mut self) {
        self.name_synced = true;
        self.mute_synced = true;
        self.solo_synced = true;
        self.phantom_synced = true;
        self.channel_strip.mark_all_synced();
        self.four_band_eq.mark_all_synced();
        self.sends.mark_all_synced();
    }

    pub fn mark_all_unsynced(&mut self) {
        self.name_synced = false;
        self.mute_synced = false;
        self.solo_synced = false;
        self.phantom_synced = false;
        self.channel_strip.mark_all_unsynced();
        self.four_band_eq.mark_all_unsynced();
        self.sends.mark_all_unsynced();
    }

    pub fn has_phantom(&self) -> bool {
        matches!(self.phantom_pwr.phantom_power_type, PhantomPowerType::Set48v)
    }

    pub fn is_all_synced(&self) -> bool {
        self.name_synced
            && self.mute_synced
            && self.solo_synced
            && (!self.has_phantom() || self.phantom_synced)
            && self.channel_strip.is_all_synced(self)
            && self.four_band_eq.is_all_synced()
            && self.sends.is_all_synced()
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

impl Bus {
    pub fn label(&self) -> String {
        match (self.bus_type, self.index) {
            (BusType::Main, _) => "Main".to_string(),
            (BusType::Monitor, idx) => format!("Mon {}", idx),
            (BusType::Fx, idx) => format!("FX {}", idx - 2),
        }
    }

    pub fn has_balance(&self) -> bool {
        self.bus_type == BusType::Main
    }

    pub fn has_limiter(&self) -> bool {
        self.bus_type != BusType::Fx
    }

    pub fn has_nine_band_eq(&self) -> bool {
        self.bus_type != BusType::Fx
    }

    pub fn mark_all_synced(&mut self) {
        self.bus_strip.mark_all_synced();
        self.nine_band_eq.mark_all_synced();
    }

    pub fn mark_all_unsynced(&mut self) {
        self.bus_strip.mark_all_unsynced();
        self.nine_band_eq.mark_all_unsynced();
    }

    pub fn is_all_synced(&self) -> bool {
        self.bus_strip.level_synced
            && (!self.has_balance() || self.bus_strip.balance_synced)
            && (!self.has_limiter() || self.bus_strip.limiter_synced)
            && (!self.has_nine_band_eq() || self.nine_band_eq.is_all_synced())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BasicChannelStrip {
    pub gain: u8,
    pub level: u8,
    pub balance: u8,
    pub low_cut: u8,
    pub compressor: u8,
    pub gain_synced: bool,
    pub level_synced: bool,
    pub balance_synced: bool,
    pub low_cut_synced: bool,
    pub compressor_synced: bool,
}

impl Default for BasicChannelStrip {
    fn default() -> Self {
        Self {
            gain: 64,
            level: 64,
            balance: 64,
            low_cut: 0,
            compressor: 0,
            gain_synced: false,
            level_synced: false,
            balance_synced: false,
            low_cut_synced: false,
            compressor_synced: false,
        }
    }
}

impl BasicChannelStrip {
    pub fn mark_all_synced(&mut self) {
        self.gain_synced = true;
        self.level_synced = true;
        self.balance_synced = true;
        self.low_cut_synced = true;
        self.compressor_synced = true;
    }

    pub fn mark_all_unsynced(&mut self) {
        self.gain_synced = false;
        self.level_synced = false;
        self.balance_synced = false;
        self.low_cut_synced = false;
        self.compressor_synced = false;
    }

    pub fn is_all_synced(&self, channel: &Channel) -> bool {
        self.level_synced
            && self.balance_synced
            && (!channel.has_gain() || self.gain_synced)
            && (!channel.has_compressor() || self.compressor_synced)
            && (!channel.has_low_cut() || self.low_cut_synced)
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct ChannelSends {
    pub mon1: u8,
    pub mon2: u8,
    pub fx1: u8,
    pub fx2: u8,
    pub mon1_synced: bool,
    pub mon2_synced: bool,
    pub fx1_synced: bool,
    pub fx2_synced: bool,
}

impl ChannelSends {
    pub fn mark_all_synced(&mut self) {
        self.mon1_synced = true;
        self.mon2_synced = true;
        self.fx1_synced = true;
        self.fx2_synced = true;
    }

    pub fn mark_all_unsynced(&mut self) {
        self.mon1_synced = false;
        self.mon2_synced = false;
        self.fx1_synced = false;
        self.fx2_synced = false;
    }

    pub fn is_all_synced(&self) -> bool {
        self.mon1_synced && self.mon2_synced && self.fx1_synced && self.fx2_synced
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BusStrip {
    pub level: u8,
    pub balance: u8,
    pub limiter: u8,
    pub level_synced: bool,
    pub balance_synced: bool,
    pub limiter_synced: bool,
}

impl Default for BusStrip {
    fn default() -> Self {
        Self {
            level: 64,
            balance: 64,
            limiter: 127,
            level_synced: false,
            balance_synced: false,
            limiter_synced: false,
        }
    }
}

impl BusStrip {
    pub fn mark_all_synced(&mut self) {
        self.level_synced = true;
        self.balance_synced = true;
        self.limiter_synced = true;
    }

    pub fn mark_all_unsynced(&mut self) {
        self.level_synced = false;
        self.balance_synced = false;
        self.limiter_synced = false;
    }

    pub fn is_all_synced(&self) -> bool {
        self.level_synced && self.balance_synced && self.limiter_synced
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FourBandEQ {
    pub low: u8,
    pub low_mid: u8,
    pub hi_mid: u8,
    pub hi: u8,
    pub low_synced: bool,
    pub low_mid_synced: bool,
    pub hi_mid_synced: bool,
    pub hi_synced: bool,
}

impl Default for FourBandEQ {
    fn default() -> Self {
        Self {
            low: 64,
            low_mid: 64,
            hi_mid: 64,
            hi: 64,
            low_synced: false,
            low_mid_synced: false,
            hi_mid_synced: false,
            hi_synced: false,
        }
    }
}

impl FourBandEQ {
    pub fn mark_all_synced(&mut self) {
        self.low_synced = true;
        self.low_mid_synced = true;
        self.hi_mid_synced = true;
        self.hi_synced = true;
    }

    pub fn mark_all_unsynced(&mut self) {
        self.low_synced = false;
        self.low_mid_synced = false;
        self.hi_mid_synced = false;
        self.hi_synced = false;
    }

    pub fn is_all_synced(&self) -> bool {
        self.low_synced && self.low_mid_synced && self.hi_mid_synced && self.hi_synced
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
    pub bands_synced: [bool; 9],
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
            bands_synced: [false; 9],
        }
    }
}

impl NineBandEQ {
    pub fn mark_all_synced(&mut self) {
        self.bands_synced = [true; 9];
    }

    pub fn mark_all_unsynced(&mut self) {
        self.bands_synced = [false; 9];
    }

    pub fn is_all_synced(&self) -> bool {
        self.bands_synced.iter().all(|&s| s)
    }
}

pub struct FxPresetInfo {
    pub name: &'static str,
    pub param1_label: &'static str,
    pub param2_off: &'static str,
    pub param2_on: &'static str,
}

pub const FX1_PRESETS: [FxPresetInfo; 16] = [
    FxPresetInfo { name: "Ambience",  param1_label: "Decay",     param2_off: "Instrument", param2_on: "Vocal" },
    FxPresetInfo { name: "Perc-Rev1", param1_label: "Decay",     param2_off: "Dull",       param2_on: "Bright" },
    FxPresetInfo { name: "Perc-Rev2", param1_label: "Decay",     param2_off: "Dull",       param2_on: "Bright" },
    FxPresetInfo { name: "Guit-Rev1", param1_label: "Decay",     param2_off: "Less Bass",  param2_on: "More Bass" },
    FxPresetInfo { name: "Guit-Rev2", param1_label: "Decay",     param2_off: "Less Bass",  param2_on: "More Bass" },
    FxPresetInfo { name: "Chamber",   param1_label: "Decay",     param2_off: "Pre-D Off",  param2_on: "Pre-D On" },
    FxPresetInfo { name: "Room",      param1_label: "Decay",     param2_off: "Instrument", param2_on: "Vocal" },
    FxPresetInfo { name: "Concert",   param1_label: "Decay",     param2_off: "Instrument", param2_on: "Vocal" },
    FxPresetInfo { name: "Church",    param1_label: "Decay",     param2_off: "Instrument", param2_on: "Vocal" },
    FxPresetInfo { name: "Cathedral", param1_label: "Decay",     param2_off: "Small",      param2_on: "Large" },
    FxPresetInfo { name: "Temple",    param1_label: "Decay",     param2_off: "Pre-D Off",  param2_on: "Pre-D On" },
    FxPresetInfo { name: "Stadium",   param1_label: "Decay",     param2_off: "Pre-D Off",  param2_on: "Pre-D On" },
    FxPresetInfo { name: "Flanger",   param1_label: "Mod Speed", param2_off: "Mono",       param2_on: "Stereo" },
    FxPresetInfo { name: "Soft Chor", param1_label: "Density",   param2_off: "Trem Off",   param2_on: "Trem On" },
    FxPresetInfo { name: "Warm Chor", param1_label: "Density",   param2_off: "Trem Off",   param2_on: "Trem On" },
    FxPresetInfo { name: "Deep Chor", param1_label: "Density",   param2_off: "Trem Off",   param2_on: "Trem On" },
];

pub const FX2_PRESETS: [FxPresetInfo; 16] = [
    FxPresetInfo { name: "Delay 1/1",   param1_label: "Feedback", param2_off: "Dull",     param2_on: "Bright" },
    FxPresetInfo { name: "Delay 1/2",   param1_label: "Feedback", param2_off: "Dull",     param2_on: "Bright" },
    FxPresetInfo { name: "Delay 1/3",   param1_label: "Feedback", param2_off: "Dull",     param2_on: "Bright" },
    FxPresetInfo { name: "Delay 2/1",   param1_label: "Feedback", param2_off: "Dull",     param2_on: "Bright" },
    FxPresetInfo { name: "Echo 1/1",    param1_label: "Feedback", param2_off: "Soft",     param2_on: "Strong" },
    FxPresetInfo { name: "Echo 1/2",    param1_label: "Feedback", param2_off: "Soft",     param2_on: "Strong" },
    FxPresetInfo { name: "Echo 1/3",    param1_label: "Feedback", param2_off: "Soft",     param2_on: "Strong" },
    FxPresetInfo { name: "Echo 2/1",    param1_label: "Feedback", param2_off: "Soft",     param2_on: "Strong" },
    FxPresetInfo { name: "Wide Echo",   param1_label: "Feedback", param2_off: "Wide",     param2_on: "Narrow" },
    FxPresetInfo { name: "Ping Pong",   param1_label: "Feedback", param2_off: "Wide",     param2_on: "Narrow" },
    FxPresetInfo { name: "Ping P 1/3",  param1_label: "Feedback", param2_off: "Wide",     param2_on: "Narrow" },
    FxPresetInfo { name: "Ping P R>L",  param1_label: "Feedback", param2_off: "Wide",     param2_on: "Narrow" },
    FxPresetInfo { name: "Flanger",     param1_label: "Mod Speed", param2_off: "Mono",    param2_on: "Stereo" },
    FxPresetInfo { name: "Soft Chor",   param1_label: "Density",  param2_off: "Trem Off", param2_on: "Trem On" },
    FxPresetInfo { name: "Warm Chor",   param1_label: "Density",  param2_off: "Trem Off", param2_on: "Trem On" },
    FxPresetInfo { name: "Deep Chor",   param1_label: "Density",  param2_off: "Trem Off", param2_on: "Trem On" },
];

#[derive(Copy, Clone, Debug)]
pub struct FxSlot {
    pub id: FxSlotId,
    pub preset: u8,
    pub param1: u8,
    pub param2: u8,
    pub preset_synced: bool,
    pub param1_synced: bool,
    pub param2_synced: bool,
}

impl FxSlot {
    pub fn new(id: FxSlotId) -> Self {
        Self {
            id,
            preset: 0,
            param1: 0,
            param2: 0,
            preset_synced: false,
            param1_synced: false,
            param2_synced: false,
        }
    }

    pub fn preset_info(&self) -> &'static FxPresetInfo {
        let presets = if self.id == 0 { &FX1_PRESETS } else { &FX2_PRESETS };
        let idx = (self.preset as usize).min(presets.len() - 1);
        &presets[idx]
    }

    pub fn presets(&self) -> &'static [FxPresetInfo; 16] {
        if self.id == 0 { &FX1_PRESETS } else { &FX2_PRESETS }
    }

    pub fn param2_is_on(&self) -> bool {
        self.param2 > 63
    }

    pub fn mark_all_synced(&mut self) {
        self.preset_synced = true;
        self.param1_synced = true;
        self.param2_synced = true;
    }

    pub fn mark_all_unsynced(&mut self) {
        self.preset_synced = false;
        self.param1_synced = false;
        self.param2_synced = false;
    }

    pub fn is_all_synced(&self) -> bool {
        self.preset_synced && self.param1_synced && self.param2_synced
    }
}
