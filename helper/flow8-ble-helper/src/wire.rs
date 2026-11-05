//! Minimal FLOW 8 BLE framing for the DirectHCI POC (copied from app protocol).

pub const CMD_HANDSHAKE_HOST: u8 = 0x35;
pub const CMD_HANDSHAKE_REPLY: u8 = 0x36;
pub const CMD_GET_MIXER_STATE: u8 = 0x37;
pub const CMD_MIXER_STATE: u8 = 0x38;
pub const CMD_HANDSHAKE_CLIENT: u8 = 0x39;
pub fn checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// Full framed packet: `[type, 0x01, payload..., checksum]`.
pub fn encode_single(cmd_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(3 + payload.len());
    buf.push(cmd_type);
    buf.push(0x01);
    buf.extend_from_slice(payload);
    let chk = checksum(&buf);
    buf.push(chk);
    buf
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeHost {
    pub device_id: [u8; 16],
    pub pairing_any: bool,
    pub protocol_version: u16,
    pub firmware_build: u16,
}

/// Decode a single-chunk notify payload into HandshakeHost.
/// `raw` is the full GATT value including type/chunks/checksum.
pub fn parse_handshake_host(raw: &[u8]) -> Result<HandshakeHost, String> {
    if raw.len() < 3 {
        return Err(format!("HandshakeHost too short: {} bytes", raw.len()));
    }
    if raw[0] != CMD_HANDSHAKE_HOST {
        return Err(format!("expected 0x35, got 0x{:02X}", raw[0]));
    }
    let expected = checksum(&raw[..raw.len() - 1]);
    if expected != raw[raw.len() - 1] {
        return Err(format!(
            "HandshakeHost checksum: expected 0x{:02X}, got 0x{:02X}",
            expected,
            raw[raw.len() - 1]
        ));
    }
    // payload between chunks byte and checksum
    let payload = &raw[2..raw.len() - 1];
    if payload.len() < 21 {
        return Err(format!(
            "HandshakeHost payload too short: {} bytes",
            payload.len()
        ));
    }
    let mut device_id = [0u8; 16];
    device_id.copy_from_slice(&payload[0..16]);
    let pairing_any = payload[16] != 0;
    let protocol_version = u16::from_be_bytes([payload[17], payload[18]]);
    let firmware_build = u16::from_be_bytes([payload[19], payload[20]]);
    Ok(HandshakeHost {
        device_id,
        pairing_any,
        protocol_version,
        firmware_build,
    })
}

pub fn load_or_create_client_id() -> [u8; 16] {
    if let Some(id) = load_client_id() {
        eprintln!(
            "[flow8-ble-helper] Using ~/.flow8-midi/client_id = {}",
            hex::encode_bytes(&id)
        );
        return id;
    }
    use rand::Rng;
    let mut id = [0u8; 16];
    rand::thread_rng().fill(&mut id);
        eprintln!(
            "[flow8-ble-helper] No ~/.flow8-midi/client_id — using ephemeral {}",
            hex::encode_bytes(&id)
        );
    id
}

fn load_client_id() -> Option<[u8; 16]> {
    let path = dirs::home_dir()?.join(".flow8-midi").join("client_id");
    let hex = std::fs::read_to_string(path).ok()?;
    let hex = hex.trim();
    if hex.len() != 32 {
        return None;
    }
    let mut id = [0u8; 16];
    for (i, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(id)
}

mod hex {
    pub fn encode_bytes(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_get_mixer_state() {
        assert_eq!(encode_single(CMD_GET_MIXER_STATE, &[]), vec![0x37, 0x01, 0x38]);
    }

    #[test]
    fn encode_handshake_reply_empty() {
        // 0x36 + 0x01 + chk(0x36+0x01=0x37)
        assert_eq!(encode_single(CMD_HANDSHAKE_REPLY, &[]), vec![0x36, 0x01, 0x37]);
    }

    #[test]
    fn encode_handshake_client_matches_hci_capture() {
        let id: [u8; 16] = [
            0xFD, 0x06, 0x2B, 0x06, 0x39, 0xF1, 0x7F, 0xE7, 0xB7, 0x27, 0x8B, 0x8F, 0x35, 0x5A,
            0x49, 0x5C,
        ];
        assert_eq!(
            encode_single(CMD_HANDSHAKE_CLIENT, &id),
            vec![
                0x39, 0x01, 0xFD, 0x06, 0x2B, 0x06, 0x39, 0xF1, 0x7F, 0xE7, 0xB7, 0x27, 0x8B, 0x8F,
                0x35, 0x5A, 0x49, 0x5C, 0x2A,
            ]
        );
    }

    #[test]
    fn parse_handshake_host_pairing_any() {
        let mut payload = vec![0u8; 21];
        payload[16] = 0x01;
        payload[17] = 0x00;
        payload[18] = 0x01;
        payload[19] = 0x00;
        payload[20] = 0x02;
        let raw = encode_single(CMD_HANDSHAKE_HOST, &payload);
        let host = parse_handshake_host(&raw).unwrap();
        assert!(host.pairing_any);
        assert_eq!(host.protocol_version, 1);
        assert_eq!(host.firmware_build, 2);
    }
}
