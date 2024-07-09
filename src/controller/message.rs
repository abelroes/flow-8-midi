use crate::model::channels::ChannelId;

#[derive(Debug, Clone, Copy)]
pub enum InterfaceMessage {
    Mute(ChannelId),
    Solo(ChannelId),
    Gain(ChannelId, u8),
    Level(ChannelId, u8),
    Balance(ChannelId, u8),
    PhantomPower(ChannelId, u8),
    Compressor(ChannelId, u8),
    EqLow(ChannelId, u8),
    EqLowMid(ChannelId, u8),
    EqHiMid(ChannelId, u8),
    EqHi(ChannelId, u8),
}