use crate::service::ble_codec::{encode_single, CodecError, PayloadWriter};

// --- Channel IDs §4 ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelId {
    Input1 = 0x00,
    Input2 = 0x01,
    Input3 = 0x02,
    Input4 = 0x03,
    Input5 = 0x04,
    Input6 = 0x05,
    BtUsb = 0x06,
    Mon1 = 0x0a,
    Mon2 = 0x0b,
    Fx1 = 0x0c,
    Fx2 = 0x0d,
    Main = 0x0f,
}

impl ChannelId {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x00 => Some(Self::Input1),
            0x01 => Some(Self::Input2),
            0x02 => Some(Self::Input3),
            0x03 => Some(Self::Input4),
            0x04 => Some(Self::Input5),
            0x05 => Some(Self::Input6),
            0x06 => Some(Self::BtUsb),
            0x0a => Some(Self::Mon1),
            0x0b => Some(Self::Mon2),
            0x0c => Some(Self::Fx1),
            0x0d => Some(Self::Fx2),
            0x0f => Some(Self::Main),
            _ => None,
        }
    }

    pub fn is_input(self) -> bool {
        (self as u8) <= 0x06
    }
}

// --- Command type bytes ---

pub mod cmd {
    pub const CHANNEL_PAN: u8 = 0x00;
    pub const CHANNEL_SOLO: u8 = 0x01;
    pub const CHANNEL_GAIN: u8 = 0x02;
    pub const CHANNEL_GEQ: u8 = 0x03;
    pub const CHANNEL_HPF: u8 = 0x04;
    pub const CHANNEL_LABEL: u8 = 0x05;
    pub const CHANNEL_MASTER: u8 = 0x06;
    pub const GET_SNAPSHOT_LIST: u8 = 0x07;
    pub const CHANNEL_MUTE: u8 = 0x08;
    pub const CHANNEL_PEQ: u8 = 0x09;
    pub const CHANNEL_PHASE: u8 = 0x10;
    pub const FX_SETUP: u8 = 0x11;
    pub const SNAPSHOT_DELETE: u8 = 0x12;
    pub const CHANNEL_SQUEEZER: u8 = 0x13;
    pub const CHANNEL_LIMITER: u8 = 0x14;
    pub const CHANNEL_PHANTOM: u8 = 0x15;
    pub const GET_CHANNEL_STATE: u8 = 0x16;
    pub const SNAPSHOT_SAVE: u8 = 0x19;
    pub const SNAPSHOT_LOAD: u8 = 0x20;
    pub const METER_REQUEST: u8 = 0x21;
    pub const GET_CHANNEL_LABELS: u8 = 0x23;
    pub const GET_SETTING: u8 = 0x26;
    pub const SETTING: u8 = 0x25;
    pub const FACTORY_RESET: u8 = 0x29;
    pub const FX_LOAD_PRESET: u8 = 0x31;
    pub const SNAPSHOT_RENAME: u8 = 0x32;
    pub const GET_MIXER_STATE: u8 = 0x37;
    pub const HANDSHAKE_CLIENT: u8 = 0x39;
    pub const FX_TEMPO: u8 = 0x40;
    pub const OUTPUT_CHANNEL_SELECT: u8 = 0x41;
    pub const SYSEX_MIDI_DUMP: u8 = 0x4b;

    pub const HANDSHAKE_HOST: u8 = 0x35;
    pub const HANDSHAKE_REPLY: u8 = 0x36;
    pub const MIXER_STATE: u8 = 0x38;
    pub const METER_UPDATE: u8 = 0x22;
    pub const SNAPSHOT_LIST: u8 = 0x27;
    pub const CHANNEL_LABELS: u8 = 0x24;
    pub const CHANNEL_CONNECTION_STATE: u8 = 0x33;
    pub const INPUT_CHANNEL_STATE: u8 = 0x17;
    pub const OUTPUT_CHANNEL_STATE: u8 = 0x18;
    pub const FX_CHANNEL_STATE: u8 = 0x30;
}

pub mod setting_tag {
    pub const BT_USB_SWITCH: u8 = 0x01;
    pub const PHONES_ONLY: u8 = 0x02;
    pub const FOOT_SWITCH: u8 = 0x03;
    pub const DEVICE_NAME: u8 = 0x05;
    pub const USB_STREAMING: u8 = 0x07;
    pub const MON_ROUTING: u8 = 0x08;
    pub const CH56_USB12: u8 = 0x09;
    pub const CH78_USB34: u8 = 0x0a;
    pub const MON_STEREO_LINK: u8 = 0x0b;
    pub const HP_USE_MON: u8 = 0x0c;
    pub const HP_USE_POST: u8 = 0x0d;
    pub const MAIN_10DBV_PAD: u8 = 0x0e;
    pub const MON_10DBV_PAD: u8 = 0x0f;
    pub const SNAPSHOT_SCOPE: u8 = 0x10;
    pub const MONITOR_SETTING: u8 = 0x11;
    pub const CHANNEL_STATES: u8 = 0x80;
    pub const CLIENT_LINKED: u8 = 0xb0;
}

// --- Outbound commands ---

#[derive(Debug, Clone)]
pub enum BleCommand {
    ChannelPan {
        id: u8,
        value: f32,
    },
    ChannelSolo {
        id: u8,
        value: bool,
    },
    ChannelGain {
        id: u8,
        db: f32,
    },
    ChannelGEQ {
        id: u8,
        geq_id: u8,
        freq: u16,
        q: f32,
        gain: f32,
    },
    ChannelHPF {
        id: u8,
        on: bool,
        freq: u16,
    },
    ChannelLabel {
        id: u8,
        icon: u16,
        text: String,
    },
    ChannelMaster {
        src: u8,
        dst: u8,
        db: f32,
    },
    ChannelMute {
        id: u8,
        value: bool,
    },
    ChannelPEQ {
        id: u8,
        peq_id: u8,
        freq: u16,
        q: f32,
        gain: f32,
    },
    ChannelPhase {
        id: u8,
        value: bool,
    },
    ChannelSqueezer {
        id: u8,
        value: f32,
    },
    ChannelLimiter {
        id: u8,
        db: f32,
    },
    ChannelPhantomPower {
        id: u8,
        value: bool,
    },
    OutputChannelSelect {
        id: u8,
    },

    FXSetup {
        id: u8,
        val1: u8,
        val2: u8,
        val3: u8,
        enable_main: bool,
        enable_mon1: bool,
        enable_mon2: bool,
    },
    FXLoadPreset {
        id: u8,
        preset: u8,
    },
    FXTempo {
        bpm: u16,
    },

    GetSnapshotList,
    SnapshotSave {
        slot: u8,
        name: String,
    },
    SnapshotLoad {
        slot: u8,
    },
    SnapshotDelete {
        slot: u8,
    },
    SnapshotRename {
        slot: u8,
        name: String,
    },

    GetSetting {
        tag: u8,
    },
    Setting {
        tag: u8,
        data: Vec<u8>,
    },
    GetChannelLabels,
    FactoryReset,

    MeterRequest {
        channel_ids: Vec<u8>,
    },
    GetChannelState {
        id: u8,
    },
    GetMixerState,

    HandshakeClient {
        client_id: [u8; 16],
    },
    SysexMidiDump,
}

pub const DEFAULT_METER_IDS: &[u8] = &[0x40, 0x41, 0x42, 0x43, 0xC4, 0xC5, 0xC6, 0x8F];

pub fn encode_command(command: &BleCommand) -> Result<Vec<u8>, CodecError> {
    match command {
        BleCommand::ChannelPan { id, value } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_uf8(1, *value)?;
            Ok(encode_single(cmd::CHANNEL_PAN, &w.finish()))
        }
        BleCommand::ChannelSolo { id, value } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_bool(*value);
            Ok(encode_single(cmd::CHANNEL_SOLO, &w.finish()))
        }
        BleCommand::ChannelGain { id, db } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_uf8(5, *db)?;
            Ok(encode_single(cmd::CHANNEL_GAIN, &w.finish()))
        }
        BleCommand::ChannelGEQ {
            id,
            geq_id,
            freq,
            q,
            gain,
        } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_u8(*geq_id);
            w.write_u16be(*freq);
            w.write_uf8(6, *q)?;
            w.write_uf8(4, *gain)?;
            Ok(encode_single(cmd::CHANNEL_GEQ, &w.finish()))
        }
        BleCommand::ChannelHPF { id, on, freq } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_bool(*on);
            let payload_before_freq = w.finish();
            let mut w2 = PayloadWriter::new();
            w2.write_raw(&payload_before_freq);
            w2.write_u16be(*freq);
            Ok(encode_single(cmd::CHANNEL_HPF, &w2.finish()))
        }
        BleCommand::ChannelLabel { id, icon, text } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_u16be(*icon);
            w.write_string(text);
            Ok(encode_single(cmd::CHANNEL_LABEL, &w.finish()))
        }
        BleCommand::ChannelMaster { src, dst, db } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*src);
            w.write_u8(*dst);
            w.write_uf8(3, *db)?;
            Ok(encode_single(cmd::CHANNEL_MASTER, &w.finish()))
        }
        BleCommand::ChannelMute { id, value } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_bool(*value);
            Ok(encode_single(cmd::CHANNEL_MUTE, &w.finish()))
        }
        BleCommand::ChannelPEQ {
            id,
            peq_id,
            freq,
            q,
            gain,
        } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_u8(*peq_id);
            w.write_u16be(*freq);
            w.write_uf8(6, *q)?;
            w.write_uf8(4, *gain)?;
            Ok(encode_single(cmd::CHANNEL_PEQ, &w.finish()))
        }
        BleCommand::ChannelPhase { id, value } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_bool(*value);
            Ok(encode_single(cmd::CHANNEL_PHASE, &w.finish()))
        }
        BleCommand::ChannelSqueezer { id, value } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_uf8(2, *value)?;
            Ok(encode_single(cmd::CHANNEL_SQUEEZER, &w.finish()))
        }
        BleCommand::ChannelLimiter { id, db } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_uf8(5, *db)?;
            Ok(encode_single(cmd::CHANNEL_LIMITER, &w.finish()))
        }
        BleCommand::ChannelPhantomPower { id, value } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_bool(*value);
            Ok(encode_single(cmd::CHANNEL_PHANTOM, &w.finish()))
        }
        BleCommand::OutputChannelSelect { id } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            Ok(encode_single(cmd::OUTPUT_CHANNEL_SELECT, &w.finish()))
        }

        BleCommand::FXSetup {
            id,
            val1,
            val2,
            val3,
            enable_main,
            enable_mon1,
            enable_mon2,
        } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_u8(*val1);
            w.write_u8(*val2);
            w.write_u8(*val3);
            w.write_bool(*enable_main);
            w.write_bool(*enable_mon1);
            w.write_bool(*enable_mon2);
            Ok(encode_single(cmd::FX_SETUP, &w.finish()))
        }
        BleCommand::FXLoadPreset { id, preset } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            w.write_u8(*preset);
            Ok(encode_single(cmd::FX_LOAD_PRESET, &w.finish()))
        }
        BleCommand::FXTempo { bpm } => {
            let mut w = PayloadWriter::new();
            w.write_u16be(*bpm);
            Ok(encode_single(cmd::FX_TEMPO, &w.finish()))
        }

        BleCommand::GetSnapshotList => Ok(encode_single(cmd::GET_SNAPSHOT_LIST, &[])),
        BleCommand::SnapshotSave { slot, name } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*slot);
            w.write_string(name);
            Ok(encode_single(cmd::SNAPSHOT_SAVE, &w.finish()))
        }
        BleCommand::SnapshotLoad { slot } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*slot);
            Ok(encode_single(cmd::SNAPSHOT_LOAD, &w.finish()))
        }
        BleCommand::SnapshotDelete { slot } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*slot);
            Ok(encode_single(cmd::SNAPSHOT_DELETE, &w.finish()))
        }
        BleCommand::SnapshotRename { slot, name } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*slot);
            w.write_string(name);
            Ok(encode_single(cmd::SNAPSHOT_RENAME, &w.finish()))
        }

        BleCommand::GetSetting { tag } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*tag);
            Ok(encode_single(cmd::GET_SETTING, &w.finish()))
        }
        BleCommand::Setting { tag, data } => {
            let mut w = PayloadWriter::new();
            w.write_blob(*tag, data);
            Ok(encode_single(cmd::SETTING, &w.finish()))
        }
        BleCommand::GetChannelLabels => Ok(encode_single(cmd::GET_CHANNEL_LABELS, &[])),
        BleCommand::FactoryReset => Ok(encode_single(cmd::FACTORY_RESET, &[])),

        BleCommand::MeterRequest { channel_ids } => {
            let mut w = PayloadWriter::new();
            w.write_u8(channel_ids.len() as u8);
            for &id in channel_ids {
                w.write_u8(id);
            }
            Ok(encode_single(cmd::METER_REQUEST, &w.finish()))
        }
        BleCommand::GetChannelState { id } => {
            let mut w = PayloadWriter::new();
            w.write_u8(*id);
            Ok(encode_single(cmd::GET_CHANNEL_STATE, &w.finish()))
        }
        BleCommand::GetMixerState => Ok(encode_single(cmd::GET_MIXER_STATE, &[])),

        BleCommand::HandshakeClient { client_id } => {
            Ok(encode_single(cmd::HANDSHAKE_CLIENT, client_id))
        }
        BleCommand::SysexMidiDump => Ok(encode_single(cmd::SYSEX_MIDI_DUMP, &[])),
    }
}

// --- Inbound events (device→host) ---

#[derive(Debug, Clone)]
pub struct HandshakeHostData {
    pub device_id: [u8; 16],
    pub pairing_any: bool,
    pub protocol_version: u16,
    pub firmware_build: u16,
}

#[derive(Debug, Clone)]
pub struct MeterData {
    pub levels: [f32; 15],
    pub gain_reduction: u16,
}

#[derive(Debug, Clone)]
pub struct ChannelConnectionData {
    pub id: u8,
    pub sub_channel: u8,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct ChannelLabelData {
    pub id: u8,
    pub icon: u16,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum BleEvent {
    HandshakeHost(HandshakeHostData),
    HandshakeAck,
    MixerState(Vec<u8>),
    MeterUpdate(MeterData),
    SnapshotList(Vec<Option<String>>),
    Setting { tag: u8, data: Vec<u8> },
    ChannelLabels(Vec<ChannelLabelData>),
    ControlEcho { cmd_type: u8, payload: Vec<u8> },
    ChannelConnection(ChannelConnectionData),
    PairingRequired,
    Disconnected(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ble_codec::decode_single;

    #[test]
    fn encode_get_mixer_state() {
        let pkt = encode_command(&BleCommand::GetMixerState).unwrap();
        assert_eq!(pkt, vec![0x37, 0x01, 0x38]);
    }

    #[test]
    fn encode_mute_ch1_on() {
        let pkt = encode_command(&BleCommand::ChannelMute { id: 0, value: true }).unwrap();
        let decoded = decode_single(&pkt).unwrap();
        assert_eq!(decoded.cmd_type, 0x08);
        assert_eq!(decoded.payload[0], 0x00);
        assert_eq!(decoded.payload[1], 0x01);
    }

    #[test]
    fn encode_mute_ch1_off() {
        let pkt = encode_command(&BleCommand::ChannelMute {
            id: 0,
            value: false,
        })
        .unwrap();
        let decoded = decode_single(&pkt).unwrap();
        assert_eq!(decoded.cmd_type, 0x08);
        assert_eq!(decoded.payload[0], 0x00);
        assert_eq!(decoded.payload[1], 0x00);
    }

    #[test]
    fn encode_pan_center() {
        let pkt = encode_command(&BleCommand::ChannelPan { id: 0, value: 0.0 }).unwrap();
        let decoded = decode_single(&pkt).unwrap();
        assert_eq!(decoded.cmd_type, 0x00);
        assert_eq!(decoded.payload[0], 0x00);
        assert_eq!(decoded.payload[1], 0x7F);
    }

    #[test]
    fn encode_master_ch1_to_main() {
        let pkt = encode_command(&BleCommand::ChannelMaster {
            src: 0x00,
            dst: 0x0f,
            db: 0.0,
        })
        .unwrap();
        let decoded = decode_single(&pkt).unwrap();
        assert_eq!(decoded.cmd_type, 0x06);
        assert_eq!(decoded.payload[0], 0x00);
        assert_eq!(decoded.payload[1], 0x0f);
    }

    #[test]
    fn encode_handshake_client() {
        let id = [
            0xFD, 0x06, 0x2B, 0x06, 0x39, 0xF1, 0x7F, 0xE7, 0xB7, 0x27, 0x8B, 0x8F, 0x35, 0x5A,
            0x49, 0x5C,
        ];
        let pkt = encode_command(&BleCommand::HandshakeClient { client_id: id }).unwrap();
        assert_eq!(pkt[0], 0x39);
        assert_eq!(pkt[1], 0x01);
        assert_eq!(&pkt[2..18], &id);
    }

    #[test]
    fn encode_fx_tempo() {
        let pkt = encode_command(&BleCommand::FXTempo { bpm: 120 }).unwrap();
        let decoded = decode_single(&pkt).unwrap();
        assert_eq!(decoded.cmd_type, 0x40);
        assert_eq!(decoded.payload, vec![0x00, 0x78]);
    }

    #[test]
    fn encode_meter_request_default() {
        let pkt = encode_command(&BleCommand::MeterRequest {
            channel_ids: DEFAULT_METER_IDS.to_vec(),
        })
        .unwrap();
        let decoded = decode_single(&pkt).unwrap();
        assert_eq!(decoded.cmd_type, 0x21);
        assert_eq!(decoded.payload[0], 8);
        assert_eq!(&decoded.payload[1..], DEFAULT_METER_IDS);
    }

    #[test]
    fn encode_snapshot_save() {
        let pkt = encode_command(&BleCommand::SnapshotSave {
            slot: 0,
            name: "MySnap".to_string(),
        })
        .unwrap();
        let decoded = decode_single(&pkt).unwrap();
        assert_eq!(decoded.cmd_type, 0x19);
        assert_eq!(decoded.payload[0], 0x00);
        assert_eq!(decoded.payload[1], 6);
        assert_eq!(&decoded.payload[2..8], b"MySnap");
    }

    #[test]
    fn encode_setting_blob() {
        let pkt = encode_command(&BleCommand::Setting {
            tag: 0x05,
            data: b"FLOW 8".to_vec(),
        })
        .unwrap();
        let decoded = decode_single(&pkt).unwrap();
        assert_eq!(decoded.cmd_type, 0x25);
        assert_eq!(decoded.payload[0], 0x05);
        assert_eq!(decoded.payload[1], 6);
        assert_eq!(&decoded.payload[2..8], b"FLOW 8");
    }

    #[test]
    fn encode_fx_setup() {
        let pkt = encode_command(&BleCommand::FXSetup {
            id: 0,
            val1: 1,
            val2: 2,
            val3: 3,
            enable_main: true,
            enable_mon1: false,
            enable_mon2: true,
        })
        .unwrap();
        let decoded = decode_single(&pkt).unwrap();
        assert_eq!(decoded.cmd_type, 0x11);
        assert_eq!(decoded.payload[0], 0x00);
        assert_eq!(decoded.payload[1], 0x01);
        assert_eq!(decoded.payload[2], 0x02);
        assert_eq!(decoded.payload[3], 0x03);
        let bool_byte = decoded.payload[4];
        assert_eq!(bool_byte & 1, 1);
        assert_eq!((bool_byte >> 1) & 1, 0);
        assert_eq!((bool_byte >> 2) & 1, 1);
    }

    #[test]
    fn encode_gain_boundaries() {
        let max = encode_command(&BleCommand::ChannelGain { id: 0, db: 60.0 }).unwrap();
        let decoded = decode_single(&max).unwrap();
        assert_eq!(decoded.payload[1], 240);

        let min = encode_command(&BleCommand::ChannelGain { id: 0, db: -60.0 }).unwrap();
        let decoded = decode_single(&min).unwrap();
        assert_eq!(decoded.payload[1], 0);
    }
}
