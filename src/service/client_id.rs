use rand::Rng;
use std::fs;
use std::path::PathBuf;

const CLIENT_ID_LEN: usize = 16;
const CLIENT_ID_FILE: &str = "client_id";
const APP_DIR: &str = ".flow8-midi";
const HANDSHAKE_TYPE: u8 = 0x39;
const HANDSHAKE_CHUNKS: u8 = 0x01;

fn app_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(APP_DIR))
}

fn client_id_path() -> Option<PathBuf> {
    app_dir().map(|d| d.join(CLIENT_ID_FILE))
}

fn generate_client_id() -> [u8; CLIENT_ID_LEN] {
    let mut rng = rand::thread_rng();
    let mut id = [0u8; CLIENT_ID_LEN];
    rng.fill(&mut id);
    id
}

fn persist_client_id(id: &[u8; CLIENT_ID_LEN]) -> Result<(), String> {
    let dir = app_dir().ok_or("Cannot resolve home directory")?;
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {}", dir.display(), e))?;

    let path = dir.join(CLIENT_ID_FILE);
    let hex = id.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    fs::write(&path, &hex).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

fn load_client_id() -> Option<[u8; CLIENT_ID_LEN]> {
    let path = client_id_path()?;
    let hex = fs::read_to_string(&path).ok()?;
    let hex = hex.trim();
    if hex.len() != CLIENT_ID_LEN * 2 {
        return None;
    }
    let mut id = [0u8; CLIENT_ID_LEN];
    for (i, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(id)
}

pub fn load_or_create() -> [u8; CLIENT_ID_LEN] {
    if let Some(id) = load_client_id() {
        return id;
    }
    let id = generate_client_id();
    if let Err(e) = persist_client_id(&id) {
        eprintln!("[CLIENT_ID] Warning: {}", e);
    }
    id
}

pub fn build_handshake_packet(id: &[u8; CLIENT_ID_LEN]) -> [u8; 19] {
    let mut pkt = [0u8; 19];
    pkt[0] = HANDSHAKE_TYPE;
    pkt[1] = HANDSHAKE_CHUNKS;
    pkt[2..18].copy_from_slice(id);

    let checksum: u8 = pkt[..18].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    pkt[18] = checksum;
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_packet_structure() {
        let id: [u8; 16] = [
            0xFD, 0x06, 0x2B, 0x06, 0x39, 0xF1, 0x7F, 0xE7, 0xB7, 0x27, 0x8B, 0x8F, 0x35, 0x5A,
            0x49, 0x5C,
        ];
        let pkt = build_handshake_packet(&id);

        assert_eq!(pkt[0], 0x39);
        assert_eq!(pkt[1], 0x01);
        assert_eq!(&pkt[2..18], &id);

        let expected_checksum: u8 = pkt[..18].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        assert_eq!(pkt[18], expected_checksum);
    }

    #[test]
    fn handshake_matches_known_packet() {
        let known: [u8; 19] = [
            0x39, 0x01, 0xFD, 0x06, 0x2B, 0x06, 0x39, 0xF1, 0x7F, 0xE7, 0xB7, 0x27, 0x8B, 0x8F,
            0x35, 0x5A, 0x49, 0x5C, 0x2A,
        ];
        let id: [u8; 16] = [
            0xFD, 0x06, 0x2B, 0x06, 0x39, 0xF1, 0x7F, 0xE7, 0xB7, 0x27, 0x8B, 0x8F, 0x35, 0x5A,
            0x49, 0x5C,
        ];
        let pkt = build_handshake_packet(&id);
        assert_eq!(pkt, known);
    }

    #[test]
    fn packet_length_is_19() {
        let id = [0u8; 16];
        let pkt = build_handshake_packet(&id);
        assert_eq!(pkt.len(), 19);
    }

    #[test]
    fn generate_produces_unique_ids() {
        let a = generate_client_id();
        let b = generate_client_id();
        assert_ne!(a, b);
    }
}
