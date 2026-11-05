use serde_json;

const SCALE3_LUT_JSON: &str = include_str!("../../docs/rea/data/uf8_scale3_lut.json");

static SCALE3_LUT: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();

fn scale3_lut() -> &'static [f32] {
    SCALE3_LUT.get_or_init(|| {
        serde_json::from_str::<Vec<f32>>(SCALE3_LUT_JSON).expect("Invalid scale3 LUT JSON")
    })
}

pub fn checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

// --- Packet framing §2 ---

#[derive(Debug, Clone)]
pub struct Packet {
    pub cmd_type: u8,
    pub payload: Vec<u8>,
}

pub fn encode_single(cmd_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(3 + payload.len());
    buf.push(cmd_type);
    buf.push(0x01);
    buf.extend_from_slice(payload);
    let chk = checksum(&buf);
    buf.push(chk);
    buf
}

pub fn decode_single(data: &[u8]) -> Result<Packet, CodecError> {
    if data.len() < 3 {
        return Err(CodecError::TooShort(data.len()));
    }
    let expected_chk = checksum(&data[..data.len() - 1]);
    let actual_chk = data[data.len() - 1];
    if expected_chk != actual_chk {
        return Err(CodecError::BadChecksum {
            expected: expected_chk,
            actual: actual_chk,
        });
    }
    Ok(Packet {
        cmd_type: data[0],
        payload: data[2..data.len() - 1].to_vec(),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum CodecError {
    TooShort(usize),
    BadChecksum { expected: u8, actual: u8 },
    IncompleteMultiChunk { expected: u8, received: u8 },
    SeqIdMismatch { expected: u8, actual: u8 },
    InvalidScale(u8),
    BufferUnderflow,
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::TooShort(n) => write!(f, "Packet too short: {} bytes", n),
            CodecError::BadChecksum { expected, actual } => {
                write!(
                    f,
                    "Checksum mismatch: expected 0x{:02X}, got 0x{:02X}",
                    expected, actual
                )
            }
            CodecError::IncompleteMultiChunk { expected, received } => {
                write!(
                    f,
                    "Incomplete multi-chunk: expected {} chunks, received {}",
                    expected, received
                )
            }
            CodecError::SeqIdMismatch { expected, actual } => {
                write!(
                    f,
                    "Seq ID mismatch: expected 0x{:02X}, got 0x{:02X}",
                    expected, actual
                )
            }
            CodecError::InvalidScale(s) => write!(f, "Invalid uf8 scale: {}", s),
            CodecError::BufferUnderflow => write!(f, "Buffer underflow during decode"),
        }
    }
}

// --- Multi-chunk assembler §2.2 ---

#[derive(Debug)]
pub struct MultiChunkAssembler {
    cmd_type: Option<u8>,
    total_chunks: u8,
    seq_id: Option<u8>,
    chunks: Vec<Option<Vec<u8>>>,
}

impl Default for MultiChunkAssembler {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiChunkAssembler {
    pub fn new() -> Self {
        Self {
            cmd_type: None,
            total_chunks: 0,
            seq_id: None,
            chunks: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.cmd_type = None;
        self.total_chunks = 0;
        self.seq_id = None;
        self.chunks.clear();
    }

    pub fn feed(&mut self, data: &[u8]) -> Result<Option<Packet>, CodecError> {
        if data.len() < 5 {
            return Err(CodecError::TooShort(data.len()));
        }

        let expected_chk = checksum(&data[..data.len() - 1]);
        let actual_chk = data[data.len() - 1];
        if expected_chk != actual_chk {
            return Err(CodecError::BadChecksum {
                expected: expected_chk,
                actual: actual_chk,
            });
        }

        let pkt_type = data[0];
        let total = data[1];
        let seq = data[2];
        let chunk_idx = data[3] as usize;
        let payload_slice = &data[4..data.len() - 1];

        if self.cmd_type.is_none() {
            self.cmd_type = Some(pkt_type);
            self.total_chunks = total;
            self.seq_id = Some(seq);
            self.chunks = vec![None; total as usize];
        } else if self.seq_id != Some(seq) {
            self.reset();
            self.cmd_type = Some(pkt_type);
            self.total_chunks = total;
            self.seq_id = Some(seq);
            self.chunks = vec![None; total as usize];
        }

        if chunk_idx < self.chunks.len() {
            self.chunks[chunk_idx] = Some(payload_slice.to_vec());
        }

        let received = self.chunks.iter().filter(|c| c.is_some()).count() as u8;
        if received == self.total_chunks {
            let mut full_payload = Vec::new();
            for chunk in &self.chunks {
                if let Some(data) = chunk {
                    full_payload.extend_from_slice(data);
                }
            }
            let result = Packet {
                cmd_type: self.cmd_type.unwrap(),
                payload: full_payload,
            };
            self.reset();
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub fn is_assembling(&self) -> bool {
        self.cmd_type.is_some()
    }
}

// --- Field types §3 / uf8 scales ---

pub fn uf8_encode(scale: u8, value: f32) -> Result<u8, CodecError> {
    match scale {
        1 => {
            let clamped = value.clamp(-1.0, 1.0);
            Ok((clamped * 127.0 + 127.0).round() as u8)
        }
        2 => {
            let clamped = value.clamp(0.0, 1.0);
            Ok((clamped * 255.0).round() as u8)
        }
        3 => {
            let lut = scale3_lut();
            let mut best_idx = 0u8;
            let mut best_dist = f32::MAX;
            for (i, &lut_val) in lut.iter().enumerate() {
                let dist = (lut_val - value).abs();
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = i as u8;
                }
            }
            Ok(best_idx)
        }
        4 => {
            let clamped = value.clamp(-15.0, 15.0);
            Ok(((clamped / 15.0) * 127.0 + 127.0).round() as u8)
        }
        5 => {
            let clamped = value.clamp(-60.0, 60.0);
            Ok(((clamped + 60.0) * 2.0).round() as u8)
        }
        6 => {
            let clamped = value.clamp(0.0, 1.0);
            Ok((clamped * 255.0).round() as u8)
        }
        _ => Err(CodecError::InvalidScale(scale)),
    }
}

pub fn uf8_decode(scale: u8, raw: u8) -> Result<f32, CodecError> {
    match scale {
        1 => Ok((raw as f32 - 127.0) / 127.0),
        2 => Ok(raw as f32 / 255.0),
        3 => {
            let lut = scale3_lut();
            Ok(lut[raw as usize])
        }
        4 => Ok(((raw as f32 - 127.0) / 127.0) * 15.0),
        5 => Ok(raw as f32 / 2.0 - 60.0),
        6 => Ok(raw as f32 / 255.0),
        _ => Err(CodecError::InvalidScale(scale)),
    }
}

pub fn db_to_display(db: f32) -> String {
    if db <= -144.0 {
        "-∞".to_string()
    } else {
        format!("{:.1}", db)
    }
}

// --- Payload reader/writer helpers ---

pub struct PayloadWriter {
    buf: Vec<u8>,
    bool_accumulator: u8,
    bool_count: u8,
}

impl Default for PayloadWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadWriter {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            bool_accumulator: 0,
            bool_count: 0,
        }
    }

    pub fn write_u8(&mut self, val: u8) {
        self.flush_bools();
        self.buf.push(val);
    }

    pub fn write_u16be(&mut self, val: u16) {
        self.flush_bools();
        self.buf.extend_from_slice(&val.to_be_bytes());
    }

    pub fn write_u32be(&mut self, val: u32) {
        self.flush_bools();
        self.buf.extend_from_slice(&val.to_be_bytes());
    }

    pub fn write_uf8(&mut self, scale: u8, value: f32) -> Result<(), CodecError> {
        self.flush_bools();
        let encoded = uf8_encode(scale, value)?;
        self.buf.push(encoded);
        Ok(())
    }

    pub fn write_bool(&mut self, val: bool) {
        if val {
            self.bool_accumulator |= 1 << self.bool_count;
        }
        self.bool_count += 1;
        if self.bool_count == 8 {
            self.buf.push(self.bool_accumulator);
            self.bool_accumulator = 0;
            self.bool_count = 0;
        }
    }

    pub fn write_string(&mut self, s: &str) {
        self.flush_bools();
        let bytes = s.as_bytes();
        self.buf.push(bytes.len() as u8);
        self.buf.extend_from_slice(bytes);
    }

    pub fn write_blob(&mut self, tag: u8, data: &[u8]) {
        self.flush_bools();
        self.buf.push(tag);
        self.buf.push(data.len() as u8);
        self.buf.extend_from_slice(data);
    }

    pub fn write_raw(&mut self, data: &[u8]) {
        self.flush_bools();
        self.buf.extend_from_slice(data);
    }

    pub fn flush_bools(&mut self) {
        if self.bool_count > 0 {
            self.buf.push(self.bool_accumulator);
            self.bool_accumulator = 0;
            self.bool_count = 0;
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.flush_bools();
        self.buf
    }
}

pub struct PayloadReader<'a> {
    data: &'a [u8],
    pos: usize,
    bool_byte: u8,
    bool_bit: u8,
    bool_active: bool,
}

impl<'a> PayloadReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bool_byte: 0,
            bool_bit: 0,
            bool_active: false,
        }
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn read_u8(&mut self) -> Result<u8, CodecError> {
        self.end_bools();
        if self.pos >= self.data.len() {
            return Err(CodecError::BufferUnderflow);
        }
        let val = self.data[self.pos];
        self.pos += 1;
        Ok(val)
    }

    pub fn read_u16be(&mut self) -> Result<u16, CodecError> {
        self.end_bools();
        if self.pos + 2 > self.data.len() {
            return Err(CodecError::BufferUnderflow);
        }
        let val = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(val)
    }

    pub fn read_u32be(&mut self) -> Result<u32, CodecError> {
        self.end_bools();
        if self.pos + 4 > self.data.len() {
            return Err(CodecError::BufferUnderflow);
        }
        let val = u32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(val)
    }

    pub fn read_uf8(&mut self, scale: u8) -> Result<f32, CodecError> {
        self.end_bools();
        let raw = self.read_u8_raw()?;
        uf8_decode(scale, raw)
    }

    pub fn read_uf8_raw(&mut self) -> Result<u8, CodecError> {
        self.end_bools();
        self.read_u8_raw()
    }

    pub fn read_bool(&mut self) -> Result<bool, CodecError> {
        if !self.bool_active {
            if self.pos >= self.data.len() {
                return Err(CodecError::BufferUnderflow);
            }
            self.bool_byte = self.data[self.pos];
            self.pos += 1;
            self.bool_bit = 0;
            self.bool_active = true;
        }
        let val = (self.bool_byte >> self.bool_bit) & 1 == 1;
        self.bool_bit += 1;
        if self.bool_bit == 8 {
            self.bool_active = false;
        }
        Ok(val)
    }

    pub fn read_string(&mut self) -> Result<String, CodecError> {
        self.end_bools();
        let len = self.read_u8_raw()? as usize;
        if self.pos + len > self.data.len() {
            return Err(CodecError::BufferUnderflow);
        }
        let s = String::from_utf8_lossy(&self.data[self.pos..self.pos + len]).to_string();
        self.pos += len;
        Ok(s)
    }

    pub fn read_blob(&mut self) -> Result<(u8, Vec<u8>), CodecError> {
        self.end_bools();
        let tag = self.read_u8_raw()?;
        let len = self.read_u8_raw()? as usize;
        if self.pos + len > self.data.len() {
            return Err(CodecError::BufferUnderflow);
        }
        let data = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok((tag, data))
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], CodecError> {
        self.end_bools();
        if self.pos + n > self.data.len() {
            return Err(CodecError::BufferUnderflow);
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_u8_raw(&mut self) -> Result<u8, CodecError> {
        if self.pos >= self.data.len() {
            return Err(CodecError::BufferUnderflow);
        }
        let val = self.data[self.pos];
        self.pos += 1;
        Ok(val)
    }

    fn end_bools(&mut self) {
        self.bool_active = false;
        self.bool_bit = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_packet_roundtrip() {
        let payload = vec![0x00, 0x7F];
        let encoded = encode_single(0x08, &payload);

        assert_eq!(encoded[0], 0x08);
        assert_eq!(encoded[1], 0x01);
        assert_eq!(&encoded[2..4], &payload);
        let chk = checksum(&encoded[..encoded.len() - 1]);
        assert_eq!(encoded[encoded.len() - 1], chk);

        let decoded = decode_single(&encoded).unwrap();
        assert_eq!(decoded.cmd_type, 0x08);
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn known_packet_get_mixer_state() {
        let packet = [0x37, 0x01, 0x38];
        let decoded = decode_single(&packet).unwrap();
        assert_eq!(decoded.cmd_type, 0x37);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn known_packet_mute_ch1_on() {
        let payload = vec![0x00, 0x01];
        let encoded = encode_single(0x08, &payload);
        assert_eq!(encoded[0], 0x08);
        assert_eq!(encoded[1], 0x01);
        assert_eq!(encoded[2], 0x00);
        assert_eq!(encoded[3], 0x01);
        let expected_chk = (0x08u16 + 0x01 + 0x00 + 0x01) as u8;
        assert_eq!(encoded[4], expected_chk);
    }

    #[test]
    fn bad_checksum_rejected() {
        let mut data = encode_single(0x08, &[0x00, 0x01]);
        let last = data.len() - 1;
        data[last] ^= 0xFF;
        assert!(matches!(
            decode_single(&data),
            Err(CodecError::BadChecksum { .. })
        ));
    }

    #[test]
    fn multi_chunk_assembler() {
        let mut asm = MultiChunkAssembler::new();

        let chunk0_payload = vec![0xAA, 0xBB];
        let mut chunk0 = vec![0x38, 0x02, 0x01, 0x00];
        chunk0.extend_from_slice(&chunk0_payload);
        chunk0.push(checksum(&chunk0));

        let result = asm.feed(&chunk0).unwrap();
        assert!(result.is_none());
        assert!(asm.is_assembling());

        let chunk1_payload = vec![0xCC, 0xDD];
        let mut chunk1 = vec![0x38, 0x02, 0x01, 0x01];
        chunk1.extend_from_slice(&chunk1_payload);
        chunk1.push(checksum(&chunk1));

        let result = asm.feed(&chunk1).unwrap();
        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt.cmd_type, 0x38);
        assert_eq!(pkt.payload, vec![0xAA, 0xBB, 0xCC, 0xDD]);
        assert!(!asm.is_assembling());
    }

    #[test]
    fn scale1_pan_roundtrip() {
        let center = uf8_encode(1, 0.0).unwrap();
        assert_eq!(center, 127);
        let decoded = uf8_decode(1, 127).unwrap();
        assert!((decoded - 0.0).abs() < 0.01);

        let full_left = uf8_encode(1, -1.0).unwrap();
        assert_eq!(full_left, 0);
        let full_right = uf8_encode(1, 1.0).unwrap();
        assert_eq!(full_right, 254);
    }

    #[test]
    fn scale2_squeezer_roundtrip() {
        let zero = uf8_encode(2, 0.0).unwrap();
        assert_eq!(zero, 0);
        let full = uf8_encode(2, 1.0).unwrap();
        assert_eq!(full, 255);
        let decoded = uf8_decode(2, 128).unwrap();
        assert!((decoded - 0.502).abs() < 0.01);
    }

    #[test]
    fn scale3_lut_boundaries() {
        let lut = scale3_lut();
        assert_eq!(lut.len(), 256);
        assert!((lut[0] - (-144.0)).abs() < 0.01);
        assert!((lut[255] - 10.0).abs() < 0.01);

        let idx_zero = uf8_encode(3, 0.0).unwrap();
        let decoded = uf8_decode(3, idx_zero).unwrap();
        assert!(decoded.abs() < 0.5);
    }

    #[test]
    fn scale4_eq_gain_roundtrip() {
        let zero = uf8_encode(4, 0.0).unwrap();
        assert_eq!(zero, 127);
        let max = uf8_encode(4, 15.0).unwrap();
        assert_eq!(max, 254);
        let min = uf8_encode(4, -15.0).unwrap();
        assert_eq!(min, 0);
    }

    #[test]
    fn scale5_gain_roundtrip() {
        let zero = uf8_encode(5, 0.0).unwrap();
        assert_eq!(zero, 120);
        let neg60 = uf8_encode(5, -60.0).unwrap();
        assert_eq!(neg60, 0);
        let pos60 = uf8_encode(5, 60.0).unwrap();
        assert_eq!(pos60, 240);
    }

    #[test]
    fn payload_writer_boolpack() {
        let mut w = PayloadWriter::new();
        w.write_bool(true);
        w.write_bool(false);
        w.write_bool(true);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(false);
        w.write_bool(true);
        w.write_bool(false);
        let data = w.finish();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0], 0b01000101);
    }

    #[test]
    fn payload_reader_boolpack() {
        let data = [0b01000101u8];
        let mut r = PayloadReader::new(&data);
        assert!(r.read_bool().unwrap());
        assert!(!r.read_bool().unwrap());
        assert!(r.read_bool().unwrap());
        assert!(!r.read_bool().unwrap());
        assert!(!r.read_bool().unwrap());
        assert!(!r.read_bool().unwrap());
        assert!(r.read_bool().unwrap());
        assert!(!r.read_bool().unwrap());
    }

    #[test]
    fn payload_writer_reader_mixed() {
        let mut w = PayloadWriter::new();
        w.write_u8(0x42);
        w.write_bool(true);
        w.write_bool(false);
        w.write_bool(true);
        w.write_u16be(0x1234);
        w.write_string("FLOW");
        let data = w.finish();

        let mut r = PayloadReader::new(&data);
        assert_eq!(r.read_u8().unwrap(), 0x42);
        assert!(r.read_bool().unwrap());
        assert!(!r.read_bool().unwrap());
        assert!(r.read_bool().unwrap());
        assert_eq!(r.read_u16be().unwrap(), 0x1234);
        assert_eq!(r.read_string().unwrap(), "FLOW");
    }

    #[test]
    fn db_display_minus_inf() {
        assert_eq!(db_to_display(-144.0), "-∞");
        assert_eq!(db_to_display(-200.0), "-∞");
        assert_eq!(db_to_display(0.0), "0.0");
        assert_eq!(db_to_display(10.0), "10.0");
    }

    #[test]
    fn invalid_scale_rejected() {
        assert!(matches!(
            uf8_encode(7, 0.0),
            Err(CodecError::InvalidScale(7))
        ));
        assert!(matches!(uf8_decode(0, 0), Err(CodecError::InvalidScale(0))));
    }
}
