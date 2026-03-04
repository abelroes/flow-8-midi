use std::env;
use std::fs;

const NAMES_START: usize = 0x0554;
const NAMES_STRIDE: usize = 0x1E;
const NAME_SCAN_LEN: usize = 14;
const CHANNEL_COUNT: usize = 7;

fn parse_hex_dump(content: &str) -> Vec<u8> {
    let mut data = Vec::new();
    for line in content.lines() {
        let hex_part = match line.split('|').next() {
            Some(h) => h.trim(),
            None => continue,
        };
        let after_colon = match hex_part.split(':').nth(1) {
            Some(h) => h.trim(),
            None => continue,
        };
        for token in after_colon.split_whitespace() {
            if let Ok(byte) = u8::from_str_radix(token, 16) {
                data.push(byte);
            }
        }
    }
    data
}

fn restore_msb_byte(data: &[u8], pos: usize) -> Option<u8> {
    let group_pos = (pos + 2) % 7;
    if group_pos == 0 {
        return None; // MSB byte itself
    }
    if pos < group_pos {
        return Some(data[pos]); // can't reach MSB (near start of dump)
    }
    let msb_off = pos - group_pos;
    if msb_off >= data.len() {
        return Some(data[pos]);
    }
    let msb = data[msb_off];
    let bit_index = group_pos - 1;
    let mut b = data[pos];
    if msb & (1 << bit_index) != 0 {
        b |= 0x80;
    }
    Some(b)
}

fn decode_name_msb7(data: &[u8], start: usize) -> (String, Vec<u8>) {
    let end = (start + NAME_SCAN_LEN).min(data.len());
    let mut restored: Vec<u8> = Vec::new();

    for i in start..end {
        if let Some(b) = restore_msb_byte(data, i) {
            restored.push(b);
        }
    }

    // skip leading control bytes (metadata before name)
    let name_start = restored.iter().position(|&b| b >= 0x20).unwrap_or(restored.len());
    let after_start = &restored[name_start..];

    // stop at first null (string terminator)
    let name_len = after_start.iter().position(|&b| b == 0x00).unwrap_or(after_start.len());
    let raw_bytes = after_start[..name_len].to_vec();

    let name = String::from_utf8(raw_bytes.clone())
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
    (name, raw_bytes)
}

fn decode_name_ascii_skip(data: &[u8], start: usize) -> String {
    let end = (start + NAME_SCAN_LEN).min(data.len());
    let mut name = String::new();
    for i in start..end {
        let b = data[i];
        if (0x20..=0x7E).contains(&b) {
            name.push(b as char);
        } else {
            let has_more = data.get(i + 1).map_or(false, |&v| (0x20..=0x7E).contains(&v))
                || data.get(i + 2).map_or(false, |&v| (0x20..=0x7E).contains(&v));
            if !has_more {
                break;
            }
        }
    }
    name
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: analyze_names <hex_dump_file> [hex_dump_file2 ...]");
        eprintln!("Example: analyze_names calibration-data/usb-bt_level_min.hex");
        std::process::exit(1);
    }

    for path in &args[1..] {
        println!("=== {} ===", path);
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  Error reading file: {}", e);
                continue;
            }
        };

        let data = parse_hex_dump(&content);
        println!("  Dump size: {} bytes", data.len());

        if data.len() < NAMES_START + CHANNEL_COUNT * NAMES_STRIDE {
            eprintln!("  Dump too small for name region");
            continue;
        }

        println!("\n  {:>4}  {:>6}  {:<42}  {:<20}  {:<20}", "Ch", "Offset", "Raw hex (14 bytes)", "Old (ASCII skip)", "New (MSB 7-byte)");
        println!("  {}", "-".repeat(100));

        for i in 0..CHANNEL_COUNT {
            let off = NAMES_START + i * NAMES_STRIDE;
            let raw: Vec<String> = data[off..off + NAME_SCAN_LEN]
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect();

            let old = decode_name_ascii_skip(&data, off);
            let (new, new_bytes) = decode_name_msb7(&data, off);
            let utf8_valid = String::from_utf8(new_bytes.clone()).is_ok();

            println!(
                "  {:>4}  0x{:04X}  {:<42}  {:<20}  {:<20}  {}",
                i + 1,
                off,
                raw.join(" "),
                format!("\"{}\"", old),
                format!("\"{}\"", new),
                if utf8_valid { "UTF-8 OK" } else { "UTF-8 INVALID" }
            );
        }
        println!();
    }
}
