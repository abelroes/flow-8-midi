use crate::service::ble_codec::{
    decode_single, CodecError, MultiChunkAssembler, Packet, PayloadReader,
};
use crate::service::ble_protocol::{
    cmd, BleEvent, ChannelConnectionData, ChannelLabelData, HandshakeHostData, MeterData,
};

pub struct BleDispatcher {
    assembler: MultiChunkAssembler,
}

impl Default for BleDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl BleDispatcher {
    pub fn new() -> Self {
        Self {
            assembler: MultiChunkAssembler::new(),
        }
    }

    pub fn dispatch(&mut self, raw: &[u8]) -> Result<Option<BleEvent>, CodecError> {
        if raw.is_empty() {
            return Ok(None);
        }

        let cmd_type = raw[0];
        let total_chunks = if raw.len() > 1 { raw[1] } else { 1 };

        if total_chunks > 1 {
            return self.dispatch_multi_chunk(raw);
        }

        let pkt = decode_single(raw)?;
        self.dispatch_packet(cmd_type, &pkt)
    }

    fn dispatch_multi_chunk(&mut self, raw: &[u8]) -> Result<Option<BleEvent>, CodecError> {
        match self.assembler.feed(raw)? {
            Some(pkt) => {
                let cmd_type = pkt.cmd_type;
                self.dispatch_packet(cmd_type, &pkt)
            }
            None => Ok(None),
        }
    }

    fn dispatch_packet(&self, cmd_type: u8, pkt: &Packet) -> Result<Option<BleEvent>, CodecError> {
        match cmd_type {
            cmd::HANDSHAKE_HOST => self.handle_handshake_host(&pkt.payload),
            cmd::HANDSHAKE_REPLY => Ok(Some(BleEvent::HandshakeAck)),
            cmd::MIXER_STATE => Ok(Some(BleEvent::MixerState(pkt.payload.clone()))),
            cmd::METER_UPDATE => self.handle_meter_update(&pkt.payload),
            cmd::SNAPSHOT_LIST => self.handle_snapshot_list(&pkt.payload),
            cmd::SETTING => self.handle_setting(&pkt.payload),
            cmd::CHANNEL_LABELS => self.handle_channel_labels(&pkt.payload),
            cmd::CHANNEL_CONNECTION_STATE => self.handle_channel_connection(&pkt.payload),

            cmd::CHANNEL_PAN
            | cmd::CHANNEL_SOLO
            | cmd::CHANNEL_GAIN
            | cmd::CHANNEL_GEQ
            | cmd::CHANNEL_HPF
            | cmd::CHANNEL_LABEL
            | cmd::CHANNEL_MASTER
            | cmd::CHANNEL_MUTE
            | cmd::CHANNEL_PEQ
            | cmd::CHANNEL_PHASE
            | cmd::CHANNEL_SQUEEZER
            | cmd::CHANNEL_LIMITER
            | cmd::CHANNEL_PHANTOM
            | cmd::FX_SETUP
            | cmd::FX_LOAD_PRESET
            | cmd::FX_TEMPO
            | cmd::SNAPSHOT_SAVE
            | cmd::SNAPSHOT_LOAD
            | cmd::SNAPSHOT_DELETE
            | cmd::SNAPSHOT_RENAME
            | cmd::FX_CHANNEL_STATE
            | cmd::INPUT_CHANNEL_STATE
            | cmd::OUTPUT_CHANNEL_STATE => Ok(Some(BleEvent::ControlEcho {
                cmd_type,
                payload: pkt.payload.clone(),
            })),

            _ => Ok(None),
        }
    }

    fn handle_handshake_host(&self, payload: &[u8]) -> Result<Option<BleEvent>, CodecError> {
        if payload.len() < 21 {
            return Err(CodecError::BufferUnderflow);
        }
        let mut device_id = [0u8; 16];
        device_id.copy_from_slice(&payload[0..16]);

        let mut r = PayloadReader::new(&payload[16..]);
        let pairing_any = r.read_bool()?;
        let protocol_version = r.read_u16be()?;
        let firmware_build = r.read_u16be()?;

        Ok(Some(BleEvent::HandshakeHost(HandshakeHostData {
            device_id,
            pairing_any,
            protocol_version,
            firmware_build,
        })))
    }

    fn handle_meter_update(&self, payload: &[u8]) -> Result<Option<BleEvent>, CodecError> {
        let mut r = PayloadReader::new(payload);
        let mut levels = [0.0f32; 15];
        for level in &mut levels {
            *level = r.read_uf8(3)?;
        }
        let gain_reduction = r.read_u16be()?;

        Ok(Some(BleEvent::MeterUpdate(MeterData {
            levels,
            gain_reduction,
        })))
    }

    fn handle_snapshot_list(&self, payload: &[u8]) -> Result<Option<BleEvent>, CodecError> {
        let mut names: Vec<Option<String>> = Vec::new();
        let mut pos = 0;

        while pos < payload.len() && names.len() < 15 {
            let len = payload[pos] as usize;
            pos += 1;
            if len == 0 {
                names.push(None);
            } else if pos + len <= payload.len() {
                let s = String::from_utf8_lossy(&payload[pos..pos + len]).to_string();
                let trimmed = s.trim().to_string();
                names.push(if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                });
                pos += len;
            } else {
                break;
            }
        }

        Ok(Some(BleEvent::SnapshotList(names)))
    }

    fn handle_setting(&self, payload: &[u8]) -> Result<Option<BleEvent>, CodecError> {
        let mut r = PayloadReader::new(payload);
        let (tag, data) = r.read_blob()?;
        Ok(Some(BleEvent::Setting { tag, data }))
    }

    fn handle_channel_labels(&self, payload: &[u8]) -> Result<Option<BleEvent>, CodecError> {
        let mut r = PayloadReader::new(payload);
        let num = r.read_u8()?;
        let mut labels = Vec::with_capacity(num as usize);

        for _ in 0..num {
            let id = r.read_u8()?;
            let icon = r.read_u16be()?;
            let text = r.read_string()?;
            labels.push(ChannelLabelData { id, icon, text });
        }

        Ok(Some(BleEvent::ChannelLabels(labels)))
    }

    fn handle_channel_connection(&self, payload: &[u8]) -> Result<Option<BleEvent>, CodecError> {
        let mut r = PayloadReader::new(payload);
        let id = r.read_u8()?;
        let sub_channel = r.read_u8()?;
        let connected = r.read_bool()?;

        Ok(Some(BleEvent::ChannelConnection(ChannelConnectionData {
            id,
            sub_channel,
            connected,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ble_codec::{checksum, encode_single, uf8_encode};

    fn build_handshake_host_packet(pairing_any: bool) -> Vec<u8> {
        let device_id = [0x01u8; 16];
        let mut payload = Vec::new();
        payload.extend_from_slice(&device_id);
        payload.push(if pairing_any { 0x01 } else { 0x00 });
        payload.extend_from_slice(&1u16.to_be_bytes());
        payload.extend_from_slice(&100u16.to_be_bytes());
        encode_single(cmd::HANDSHAKE_HOST, &payload)
    }

    #[test]
    fn dispatch_handshake_host_pairing_true() {
        let mut d = BleDispatcher::new();
        let raw = build_handshake_host_packet(true);
        let event = d.dispatch(&raw).unwrap().unwrap();
        if let BleEvent::HandshakeHost(data) = event {
            assert!(data.pairing_any);
            assert_eq!(data.protocol_version, 1);
            assert_eq!(data.firmware_build, 100);
        } else {
            panic!("Expected HandshakeHost");
        }
    }

    #[test]
    fn dispatch_handshake_host_pairing_false() {
        let mut d = BleDispatcher::new();
        let raw = build_handshake_host_packet(false);
        let event = d.dispatch(&raw).unwrap().unwrap();
        if let BleEvent::HandshakeHost(data) = event {
            assert!(!data.pairing_any);
        } else {
            panic!("Expected HandshakeHost");
        }
    }

    #[test]
    fn dispatch_handshake_ack() {
        let mut d = BleDispatcher::new();
        let raw = encode_single(cmd::HANDSHAKE_REPLY, &[]);
        let event = d.dispatch(&raw).unwrap().unwrap();
        assert!(matches!(event, BleEvent::HandshakeAck));
    }

    #[test]
    fn dispatch_meter_update() {
        let mut d = BleDispatcher::new();
        let mut payload = Vec::new();
        for i in 0..15u8 {
            payload.push(uf8_encode(3, 0.0).unwrap());
            let _ = i;
        }
        payload.extend_from_slice(&0u16.to_be_bytes());
        let raw = encode_single(cmd::METER_UPDATE, &payload);
        let event = d.dispatch(&raw).unwrap().unwrap();
        if let BleEvent::MeterUpdate(data) = event {
            for level in &data.levels {
                assert!(level.abs() < 0.5);
            }
        } else {
            panic!("Expected MeterUpdate");
        }
    }

    #[test]
    fn dispatch_mute_echo() {
        let mut d = BleDispatcher::new();
        let raw = encode_single(cmd::CHANNEL_MUTE, &[0x00, 0x01]);
        let event = d.dispatch(&raw).unwrap().unwrap();
        if let BleEvent::ControlEcho { cmd_type, payload } = event {
            assert_eq!(cmd_type, cmd::CHANNEL_MUTE);
            assert_eq!(payload, vec![0x00, 0x01]);
        } else {
            panic!("Expected ControlEcho");
        }
    }

    #[test]
    fn dispatch_setting() {
        let mut d = BleDispatcher::new();
        let payload_inner = vec![0x05, 0x06, b'F', b'L', b'O', b'W', b' ', b'8'];
        let raw = encode_single(cmd::SETTING, &payload_inner);
        let event = d.dispatch(&raw).unwrap().unwrap();
        if let BleEvent::Setting { tag, data } = event {
            assert_eq!(tag, 0x05);
            assert_eq!(data, b"FLOW 8");
        } else {
            panic!("Expected Setting");
        }
    }

    #[test]
    fn dispatch_channel_connection() {
        let mut d = BleDispatcher::new();
        let mut payload = Vec::new();
        payload.push(0x00);
        payload.push(0x00);
        payload.push(0x01);
        let raw = encode_single(cmd::CHANNEL_CONNECTION_STATE, &payload);
        let event = d.dispatch(&raw).unwrap().unwrap();
        if let BleEvent::ChannelConnection(data) = event {
            assert_eq!(data.id, 0x00);
            assert_eq!(data.sub_channel, 0x00);
            assert!(data.connected);
        } else {
            panic!("Expected ChannelConnection");
        }
    }

    #[test]
    fn dispatch_multi_chunk_mixer_state() {
        let mut d = BleDispatcher::new();
        let test_payload = vec![0xAA, 0xBB, 0xCC, 0xDD];

        let mut chunk0 = vec![cmd::MIXER_STATE, 0x02, 0x01, 0x00, 0xAA, 0xBB];
        chunk0.push(checksum(&chunk0));

        let mut chunk1 = vec![cmd::MIXER_STATE, 0x02, 0x01, 0x01, 0xCC, 0xDD];
        chunk1.push(checksum(&chunk1));

        let result0 = d.dispatch(&chunk0).unwrap();
        assert!(result0.is_none());

        let result1 = d.dispatch(&chunk1).unwrap();
        assert!(result1.is_some());
        if let Some(BleEvent::MixerState(payload)) = result1 {
            assert_eq!(payload, test_payload);
        } else {
            panic!("Expected MixerState");
        }
    }

    #[test]
    fn dispatch_unknown_type_returns_none() {
        let mut d = BleDispatcher::new();
        let raw = encode_single(0xFE, &[0x01]);
        let result = d.dispatch(&raw).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn dispatch_snapshot_list() {
        let mut d = BleDispatcher::new();
        let mut payload = Vec::new();
        payload.push(4);
        payload.extend_from_slice(b"Live");
        payload.push(0);
        payload.push(6);
        payload.extend_from_slice(b"Studio");
        let raw = encode_single(cmd::SNAPSHOT_LIST, &payload);
        let event = d.dispatch(&raw).unwrap().unwrap();
        if let BleEvent::SnapshotList(names) = event {
            assert_eq!(names.len(), 3);
            assert_eq!(names[0], Some("Live".to_string()));
            assert_eq!(names[1], None);
            assert_eq!(names[2], Some("Studio".to_string()));
        } else {
            panic!("Expected SnapshotList");
        }
    }
}
