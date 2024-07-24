use super::channels::{BusId, BusIdx, ChannelId};

#[derive(Debug, Clone, Copy)]
pub enum InterfaceMessage {
    Mute(ChannelId, bool),
    Solo(ChannelId, bool),
    Gain(ChannelId, u8),
    Level(ChannelId, u8),
    Balance(ChannelId, u8),
    PhantomPower(ChannelId, bool, bool),
    Compressor(ChannelId, u8),
    EqLow(ChannelId, u8),
    EqLowMid(ChannelId, u8),
    EqHiMid(ChannelId, u8),
    EqHi(ChannelId, u8),
    BusLevel(BusIdx, BusId, u8),
    BusBalance(BusIdx, BusId, u8),
}
