use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const DUMPS_DIR: &str = "calibration-data";
const OUTPUT_DIR: &str = "calibration-data";

const CHANNEL_LABELS: [&str; 7] = ["ch1", "ch2", "ch3", "ch4", "ch5-6", "ch7-8", "usb-bt"];
const BUS_LABELS: [&str; 5] = ["main", "mon1", "mon2", "fx1", "fx2"];

const CHANNEL_PARAMS: [&str; 15] = [
    "level", "gain", "pan", "comp", "lowcut",
    "eq_low", "eq_lowmid", "eq_himid", "eq_hi",
    "send_mon1", "send_mon2", "send_fx1", "send_fx2",
    "mute", "solo",
];
const BUS_PARAMS: [&str; 12] = [
    "level", "limiter", "balance",
    "9band_62hz", "9band_125hz", "9band_250hz", "9band_500hz",
    "9band_1khz", "9band_2khz", "9band_4khz", "9band_8khz", "9band_16khz",
];
const FX_LABELS: [&str; 2] = ["fx1", "fx2"];
const FX_PARAMS: [&str; 3] = ["param1", "param2", "preset"];

fn main() {
    let dumps_dir = PathBuf::from(DUMPS_DIR);
    if !dumps_dir.exists() {
        eprintln!("No calibration dumps found at {}", DUMPS_DIR);
        eprintln!("Run calibration from the app first.");
        std::process::exit(1);
    }

    println!("Reading calibration dumps from {}...", DUMPS_DIR);

    let ordered_labels = build_ordered_labels();
    let dumps = load_ordered_dumps(&dumps_dir, &ordered_labels);

    println!("Loaded {}/{} dumps.", dumps.len(), ordered_labels.len());

    let pairs = collect_param_pairs(&ordered_labels, &dumps);
    println!("Found {} param pairs.\n", pairs.len());

    let mut digest = String::from("# SysEx Calibration Digest\n\n");
    digest.push_str(&format!(
        "Extracted from {} dumps ({} param pairs).\n\n",
        dumps.len(),
        pairs.len()
    ));

    let mut extracted: Vec<ExtractedParam> = Vec::new();
    let mut bool_extracted: Vec<BoolExtractedParam> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for pair in &pairs {
        digest.push_str(&format!("## {}\n\n", pair.name));

        let prev = match &pair.prev_dump {
            Some(d) => d,
            None => {
                digest.push_str("SKIP: No previous dump available\n\n");
                failed.push(pair.name.clone());
                continue;
            }
        };
        let min_d = match &pair.min_dump {
            Some(d) => d,
            None => {
                digest.push_str("SKIP: Missing min dump\n\n");
                failed.push(pair.name.clone());
                continue;
            }
        };
        let max_d = match &pair.max_dump {
            Some(d) => d,
            None => {
                digest.push_str("SKIP: Missing max dump\n\n");
                failed.push(pair.name.clone());
                continue;
            }
        };

        if prev.len() != min_d.len() || min_d.len() != max_d.len() {
            digest.push_str("SKIP: Dump size mismatch\n\n");
            failed.push(pair.name.clone());
            continue;
        }

        let min_diffs: Vec<usize> = (0x20..min_d.len())
            .filter(|&i| min_d[i] != prev[i])
            .collect();
        let max_diffs: Vec<usize> = (0x20..max_d.len())
            .filter(|&i| max_d[i] != min_d[i])
            .collect();

        let all_offsets: BTreeSet<usize> = min_diffs.iter().chain(max_diffs.iter()).copied().collect();
        let all_sorted: Vec<usize> = all_offsets.into_iter().collect();

        digest.push_str(&format!(
            "Min diff ({} bytes): {}\n",
            min_diffs.len(),
            fmt_offsets(&min_diffs)
        ));
        digest.push_str(&format!(
            "Max diff ({} bytes): {}\n",
            max_diffs.len(),
            fmt_offsets(&max_diffs)
        ));
        digest.push_str(&format!(
            "Union ({} offsets): {}\n\n",
            all_sorted.len(),
            fmt_offsets(&all_sorted)
        ));

        if all_sorted.is_empty() {
            digest.push_str("SKIP: No byte differences found\n\n");
            failed.push(pair.name.clone());
            continue;
        }

        let is_bool_param = pair.name.contains("_mute")
            || pair.name.contains("_solo")
            || pair.name.contains("_phantom");

        if is_bool_param {
            match try_extract_bool(&max_diffs, min_d, max_d) {
                Some(mut bp) => {
                    bp.name = pair.name.clone();
                    let inv_str = if bp.inverted { " (inverted: 0x01=off)" } else { "" };
                    digest.push_str(&format!(
                        "OK (bool): offset=0x{:04X}{}\n\n",
                        bp.offset, inv_str
                    ));
                    println!("  {} — OK (bool, offset=0x{:04X})", pair.name, bp.offset);
                    bool_extracted.push(bp);
                }
                None => {
                    digest.push_str("FAILED: Could not auto-extract BoolParam\n\n");
                    println!("  {} — FAILED (bool)", pair.name);
                    failed.push(pair.name.clone());
                }
            }
            continue;
        }

        let (valid_lo, valid_hi) = validation_range(&pair.name);

        match try_extract_param(&all_sorted, min_d, max_d, valid_lo, valid_hi) {
            Some(mut param) => {
                param.name = pair.name.clone();
                digest.push_str(&format!(
                    "OK: msb=0x{:04X} data=[0x{:04X}, 0x{:04X}, 0x{:04X}, 0x{:04X}] bits=[{}, {}, {}, {}]\n",
                    param.msb_off,
                    param.data_offs[0], param.data_offs[1],
                    param.data_offs[2], param.data_offs[3],
                    param.bit_indices[0], param.bit_indices[1],
                    param.bit_indices[2], param.bit_indices[3],
                ));
                digest.push_str(&format!(
                    "   float range: {:.4} .. {:.4}\n\n",
                    param.min_float, param.max_float
                ));
                println!(
                    "  {} — OK ({:.1} .. {:.1})",
                    pair.name, param.min_float, param.max_float
                );
                extracted.push(param);
            }
            None => {
                digest.push_str("FAILED: Could not auto-extract FloatParam\n\n");
                println!("  {} — FAILED", pair.name);
                failed.push(pair.name.clone());
            }
        }
    }

    digest.push_str("---\n\n# Generated Rust Code\n\n");
    digest.push_str("Copy these tables into `src/sysex_parser.rs`.\n\n");
    digest.push_str(&generate_rust_tables(&extracted));

    if !bool_extracted.is_empty() {
        digest.push_str("\n# Boolean Parameters\n\n");
        digest.push_str(&generate_rust_bool_tables(&bool_extracted));
    }

    if !failed.is_empty() {
        digest.push_str(&format!("\n# Failed Extractions ({}/{})\n\n", failed.len(), pairs.len()));
        for name in &failed {
            digest.push_str(&format!("- {}\n", name));
        }
    }

    let output_dir = PathBuf::from(OUTPUT_DIR);
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");
    let path = output_dir.join("calibration-digest.md");
    fs::write(&path, &digest).expect("Failed to write digest");

    println!(
        "\nDone: {}/{} params extracted. Digest saved to {}",
        extracted.len(),
        pairs.len(),
        path.display()
    );
}

// ── Dump ordering (mirrors sysex_calibration::build_calibration_steps) ─

const PHANTOM_CHANNELS: [&str; 2] = ["ch1", "ch2"];

fn build_ordered_labels() -> Vec<String> {
    let mut labels = Vec::new();
    labels.push("baseline".to_string());

    for param in &CHANNEL_PARAMS {
        for ch in &CHANNEL_LABELS {
            labels.push(format!("{}_{}_min", ch, param));
            labels.push(format!("{}_{}_max", ch, param));
        }
    }

    for param in &BUS_PARAMS {
        for bus in &BUS_LABELS {
            labels.push(format!("{}_{}_min", bus, param));
            labels.push(format!("{}_{}_max", bus, param));
        }
    }
    for param in &FX_PARAMS {
        for fx in &FX_LABELS {
            labels.push(format!("{}_{}_min", fx, param));
            labels.push(format!("{}_{}_max", fx, param));
        }
    }

    for ch in &PHANTOM_CHANNELS {
        labels.push(format!("{}_phantom_min", ch));
        labels.push(format!("{}_phantom_max", ch));
    }

    labels.push("final_state".to_string());
    labels
}

fn load_ordered_dumps(dir: &PathBuf, labels: &[String]) -> Vec<Option<Vec<u8>>> {
    labels
        .iter()
        .map(|label| {
            let filename = format!("{}.hex", sanitize(label));
            let path = dir.join(&filename);
            if path.exists() {
                fs::read_to_string(&path).ok().and_then(|s| parse_hex_dump(&s))
            } else {
                None
            }
        })
        .collect()
}

fn parse_hex_dump(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    for line in text.lines() {
        let hex_part = line.split('|').next()?;
        let after_colon = hex_part.split(':').nth(1)?;
        for token in after_colon.split_whitespace() {
            if let Ok(b) = u8::from_str_radix(token, 16) {
                bytes.push(b);
            }
        }
    }
    if bytes.is_empty() { None } else { Some(bytes) }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

// ── Param pair collection ──────────────────────────────────────────────

struct ParamPair {
    name: String,
    prev_dump: Option<Vec<u8>>,
    min_dump: Option<Vec<u8>>,
    max_dump: Option<Vec<u8>>,
}

fn collect_param_pairs(labels: &[String], dumps: &[Option<Vec<u8>>]) -> Vec<ParamPair> {
    let mut pairs = Vec::new();
    let mut i = 1;
    while i + 1 < labels.len() {
        let min_label = &labels[i];
        let max_label = &labels[i + 1];
        if min_label.ends_with("_min") && max_label.ends_with("_max") {
            let name = min_label.trim_end_matches("_min").to_string();
            if max_label.starts_with(&name) {
                let prev = if i > 0 { dumps[i - 1].clone() } else { None };
                pairs.push(ParamPair {
                    name,
                    prev_dump: prev,
                    min_dump: dumps[i].clone(),
                    max_dump: dumps[i + 1].clone(),
                });
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    pairs
}

// ── FloatParam extraction ──────────────────────────────────────────────

struct ExtractedParam {
    name: String,
    msb_off: usize,
    data_offs: [usize; 4],
    bit_indices: [u8; 4],
    min_float: f32,
    max_float: f32,
}

struct BoolExtractedParam {
    name: String,
    offset: usize,
    inverted: bool,
}

fn fmt_offsets(offsets: &[usize]) -> String {
    offsets
        .iter()
        .map(|o| format!("0x{:04X}", o))
        .collect::<Vec<_>>()
        .join(", ")
}

const SEARCH_RADIUS: usize = 8;
const MIN_FLOAT_SPREAD: f32 = 0.05;
const MAX_GROUP_SPAN: usize = 8;

fn validation_range(name: &str) -> (f32, f32) {
    if name.contains("_pan") || name.contains("_balance") {
        (-1.5, 1.5)
    } else if name.contains("_comp") {
        (-0.5, 1.5)
    } else if name.contains("_lowcut") {
        (0.0, 800.0)
    } else if name.contains("_limiter") {
        (-35.0, 5.0)
    } else if name.contains("_send_fx") {
        (-200.0, 15.0)
    } else if name.contains("_gain") {
        (5.0, 65.0)
    } else if name.contains("_eq_") || name.contains("_9band_") {
        (-20.0, 20.0)
    } else if name.contains("_param1") || name.contains("_param2") {
        (-200.0, 200.0)
    } else if name.contains("_preset") {
        (-200.0, 200.0)
    } else {
        (-200.0, 200.0)
    }
}

fn offset_to_bit(d: isize) -> Option<u8> {
    match d {
        1..=7 => Some((d - 1) as u8),
        -7..=-1 => Some((-d + 1) as u8),
        _ => None,
    }
}

fn try_extract_param(
    changed: &[usize],
    min_dump: &[u8],
    max_dump: &[u8],
    valid_lo: f32,
    valid_hi: f32,
) -> Option<ExtractedParam> {
    if changed.is_empty() {
        return None;
    }

    let lo = changed.iter().copied().min().unwrap().saturating_sub(SEARCH_RADIUS);
    let hi = (changed.iter().copied().max().unwrap() + SEARCH_RADIUS).min(min_dump.len() - 1);

    let mut best: Option<(ExtractedParam, usize)> = None;

    for msb_off in lo..=hi {
        if msb_off >= min_dump.len() {
            continue;
        }

        let mut data_candidates: Vec<(usize, u8)> = Vec::new();
        for d in -7isize..=7 {
            if d == 0 {
                continue;
            }
            let off = msb_off as isize + d;
            if off < 0 || off as usize >= min_dump.len() {
                continue;
            }
            let off = off as usize;
            if let Some(bit) = offset_to_bit(d) {
                if bit < 7 {
                    data_candidates.push((off, bit));
                }
            }
        }

        if data_candidates.len() < 4 {
            continue;
        }

        let msb_min = min_dump[msb_off];
        let msb_max = max_dump[msb_off];

        for combo in combinations_indexed(&data_candidates, 4) {
            let data_offs = [combo[0].0, combo[1].0, combo[2].0, combo[3].0];
            let bit_indices = [combo[0].1, combo[1].1, combo[2].1, combo[3].1];

            let mut bits_used = 0u8;
            let mut unique = true;
            for &b in &bit_indices {
                if bits_used & (1 << b) != 0 {
                    unique = false;
                    break;
                }
                bits_used |= 1 << b;
            }
            if !unique {
                continue;
            }

            let all5 = [msb_off, data_offs[0], data_offs[1], data_offs[2], data_offs[3]];
            let span = all5.iter().max().unwrap() - all5.iter().min().unwrap();
            if span > MAX_GROUP_SPAN {
                continue;
            }

            let overlap = all5.iter().filter(|o| changed.contains(o)).count();
            if overlap == 0 {
                continue;
            }

            let mut bytes_min = [0u8; 4];
            let mut bytes_max = [0u8; 4];
            for i in 0..4 {
                let hi_min = (msb_min >> bit_indices[i]) & 1;
                let hi_max = (msb_max >> bit_indices[i]) & 1;
                bytes_min[i] = min_dump[data_offs[i]] | (hi_min << 7);
                bytes_max[i] = max_dump[data_offs[i]] | (hi_max << 7);
            }

            let f_min = f32::from_le_bytes(bytes_min);
            let f_max = f32::from_le_bytes(bytes_max);

            if !f_min.is_finite() || !f_max.is_finite() {
                continue;
            }
            if f_min < valid_lo || f_max > valid_hi {
                continue;
            }
            if f_min >= f_max || (f_max - f_min) < MIN_FLOAT_SPREAD {
                continue;
            }

            if let Some((_, best_score)) = &best {
                if overlap > *best_score {
                    best = Some((
                        ExtractedParam {
                            name: String::new(),
                            msb_off,
                            data_offs,
                            bit_indices,
                            min_float: f_min,
                            max_float: f_max,
                        },
                        overlap,
                    ));
                }
            } else {
                best = Some((
                    ExtractedParam {
                        name: String::new(),
                        msb_off,
                        data_offs,
                        bit_indices,
                        min_float: f_min,
                        max_float: f_max,
                    },
                    overlap,
                ));
            }

            if overlap == changed.len().min(5) {
                return best.map(|(p, _)| p);
            }
        }
    }

    best.map(|(p, _)| p)
}

fn combinations_indexed(items: &[(usize, u8)], k: usize) -> Vec<Vec<(usize, u8)>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }
    let mut result = Vec::new();
    for (i, &item) in items.iter().enumerate() {
        for mut combo in combinations_indexed(&items[i + 1..], k - 1) {
            combo.insert(0, item);
            result.push(combo);
        }
    }
    result
}

// ── BoolParam extraction ───────────────────────────────────────────────

fn try_extract_bool(
    max_diffs: &[usize],
    min_dump: &[u8],
    max_dump: &[u8],
) -> Option<BoolExtractedParam> {
    for &off in max_diffs {
        let min_val = min_dump[off];
        let max_val = max_dump[off];
        if (min_val == 0x00 && max_val == 0x01) || (min_val == 0x01 && max_val == 0x00) {
            let inverted = min_val == 0x01;
            return Some(BoolExtractedParam {
                name: String::new(),
                offset: off,
                inverted,
            });
        }
    }
    None
}

// ── Rust code generation ───────────────────────────────────────────────

fn generate_rust_tables(params: &[ExtractedParam]) -> String {
    let mut code = String::new();

    let channel_groups: &[(&str, &str)] = &[
        ("CHANNEL_LEVELS", "level"),
        ("CHANNEL_GAINS", "gain"),
        ("CHANNEL_PANS", "pan"),
        ("CHANNEL_COMPRESSORS", "comp"),
        ("CHANNEL_LOW_CUTS", "lowcut"),
        ("CHANNEL_EQ_LOW", "eq_low"),
        ("CHANNEL_EQ_LOW_MID", "eq_lowmid"),
        ("CHANNEL_EQ_HI_MID", "eq_himid"),
        ("CHANNEL_EQ_HI", "eq_hi"),
        ("CHANNEL_SEND_MON1", "send_mon1"),
        ("CHANNEL_SEND_MON2", "send_mon2"),
        ("CHANNEL_SEND_FX1", "send_fx1"),
        ("CHANNEL_SEND_FX2", "send_fx2"),
    ];

    let bus_groups: &[(&str, &str)] = &[
        ("BUS_LEVELS", "level"),
        ("BUS_LIMITERS", "limiter"),
        ("BUS_BALANCES", "balance"),
    ];

    let nine_band_groups: &[(&str, &str)] = &[
        ("NINE_BAND_62HZ", "9band_62hz"),
        ("NINE_BAND_125HZ", "9band_125hz"),
        ("NINE_BAND_250HZ", "9band_250hz"),
        ("NINE_BAND_500HZ", "9band_500hz"),
        ("NINE_BAND_1KHZ", "9band_1khz"),
        ("NINE_BAND_2KHZ", "9band_2khz"),
        ("NINE_BAND_4KHZ", "9band_4khz"),
        ("NINE_BAND_8KHZ", "9band_8khz"),
        ("NINE_BAND_16KHZ", "9band_16khz"),
    ];

    let fx_groups: &[(&str, &str)] = &[
        ("FX_PARAM1", "param1"),
        ("FX_PARAM2", "param2"),
        ("FX_PRESETS", "preset"),
    ];

    for (const_name, suffix) in channel_groups {
        emit_table(&mut code, const_name, suffix, &CHANNEL_LABELS, params);
    }
    for (const_name, suffix) in bus_groups {
        emit_table(&mut code, const_name, suffix, &BUS_LABELS, params);
    }
    for (const_name, suffix) in nine_band_groups {
        emit_table(&mut code, const_name, suffix, &BUS_LABELS, params);
    }
    for (const_name, suffix) in fx_groups {
        emit_table(&mut code, const_name, suffix, &FX_LABELS, params);
    }

    code
}

fn emit_table(
    code: &mut String,
    const_name: &str,
    suffix: &str,
    prefixes: &[&str],
    params: &[ExtractedParam],
) {
    let count = prefixes.len();
    let mut entries = Vec::new();
    let mut found = 0usize;

    for prefix in prefixes {
        let name = format!("{}_{}", prefix, suffix);
        if let Some(p) = params.iter().find(|p| p.name == name) {
            entries.push(format!(
                "    FloatParam {{ msb_off: 0x{:04X}, data_offs: [0x{:04X}, 0x{:04X}, 0x{:04X}, 0x{:04X}], bit_indices: [{}, {}, {}, {}] }}, // {} ({:.1}..{:.1})",
                p.msb_off,
                p.data_offs[0], p.data_offs[1], p.data_offs[2], p.data_offs[3],
                p.bit_indices[0], p.bit_indices[1], p.bit_indices[2], p.bit_indices[3],
                prefix, p.min_float, p.max_float,
            ));
            found += 1;
        } else {
            entries.push(format!("    // MISSING: {}", name));
        }
    }

    code.push_str(&format!(
        "```rust\n// {}/{} extracted\nconst {}: [FloatParam; {}] = [\n",
        found, count, const_name, count
    ));
    for entry in &entries {
        code.push_str(entry);
        code.push('\n');
    }
    code.push_str("];\n```\n\n");
}

fn generate_rust_bool_tables(params: &[BoolExtractedParam]) -> String {
    let mut code = String::new();
    let ch_prefixes: &[&str] = &["ch1", "ch2", "ch3", "ch4", "ch5-6", "ch7-8", "usb-bt"];

    let bool_groups: &[(&str, &str, &[&str])] = &[
        ("CHANNEL_MUTES", "mute", ch_prefixes),
        ("CHANNEL_SOLOS", "solo", ch_prefixes),
        ("CHANNEL_PHANTOMS", "phantom", &["ch1", "ch2"]),
    ];

    for (const_name, suffix, prefixes) in bool_groups {
        let mut entries = Vec::new();
        let mut found = 0usize;

        for prefix in *prefixes {
            let name = format!("{}_{}", prefix, suffix);
            if let Some(p) = params.iter().find(|p| p.name == name) {
                let inv_str = if p.inverted { " // inverted" } else { "" };
                entries.push(format!(
                    "    BoolParam {{ offset: 0x{:04X}, inverted: {} }},{} // {}",
                    p.offset, p.inverted, inv_str, prefix
                ));
                found += 1;
            } else {
                entries.push(format!("    // MISSING: {}", name));
            }
        }

        code.push_str(&format!(
            "```rust\n// {}/{} extracted\nconst {}: [BoolParam; {}] = [\n",
            found, prefixes.len(), const_name, prefixes.len()
        ));
        for entry in &entries {
            code.push_str(entry);
            code.push('\n');
        }
        code.push_str("];\n```\n\n");
    }

    code
}
