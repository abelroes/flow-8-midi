use crate::service::sysex_parser::format_hex_dump;
use crate::{log, log_warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SETTLE_TIME: Duration = Duration::from_millis(800);
const DUMP_TIMEOUT: Duration = Duration::from_secs(5);
const DUMPS_DIR: &str = "calibration-data";

#[derive(Debug, Clone, Copy, PartialEq)]
enum StepAction {
    Baseline,
    SetParam { midi_ch: u8, cc: u8, value: u8 },
    SetPreset { midi_ch: u8, program: u8 },
}

#[derive(Debug, Clone)]
struct CalibStep {
    action: StepAction,
    label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CalibPhase {
    Idle,
    SendCC(usize),
    Settling(usize, Instant),
    RequestDump(usize),
    WaitingDump(usize, Instant),
    Done,
}

pub struct CalibAction {
    pub send_cc: Option<(u8, u8, u8)>,
    pub send_pc: Option<(u8, u8)>,
    pub trigger_dump: bool,
}

pub struct CalibrationState {
    pub phase: CalibPhase,
    steps: Vec<CalibStep>,
    dumps: Vec<(String, Vec<u8>)>,
}

impl Default for CalibrationState {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationState {
    pub fn new() -> Self {
        Self {
            phase: CalibPhase::Idle,
            steps: Vec::new(),
            dumps: Vec::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        !matches!(self.phase, CalibPhase::Idle | CalibPhase::Done)
    }

    pub fn start(&mut self) {
        self.steps = build_calibration_steps();
        self.dumps.clear();
        self.phase = CalibPhase::RequestDump(0);
        log!(
            "[CALIB] Starting calibration ({} steps, ~{}s estimated)",
            self.steps.len(),
            self.steps.len() * 2
        );
    }

    pub fn tick(&mut self) -> CalibAction {
        let mut action = CalibAction {
            send_cc: None,
            send_pc: None,
            trigger_dump: false,
        };

        match self.phase {
            CalibPhase::SendCC(step) => {
                match self.steps[step].action {
                    StepAction::SetParam {
                        midi_ch,
                        cc,
                        value,
                    } => {
                        log!(
                            "[CALIB] Step {}/{}: {} — CC ch={} cc={} val={}",
                            step + 1,
                            self.steps.len(),
                            self.steps[step].label,
                            midi_ch,
                            cc,
                            value
                        );
                        action.send_cc = Some((midi_ch, cc, value));
                    }
                    StepAction::SetPreset { midi_ch, program } => {
                        log!(
                            "[CALIB] Step {}/{}: {} — PC ch={} prog={}",
                            step + 1,
                            self.steps.len(),
                            self.steps[step].label,
                            midi_ch,
                            program
                        );
                        action.send_pc = Some((midi_ch, program));
                    }
                    StepAction::Baseline => {}
                }
                self.phase = CalibPhase::Settling(step, Instant::now());
            }
            CalibPhase::Settling(step, since) => {
                if since.elapsed() >= SETTLE_TIME {
                    self.phase = CalibPhase::RequestDump(step);
                }
            }
            CalibPhase::RequestDump(step) => {
                log!(
                    "[CALIB] Requesting dump for step {}/{}...",
                    step + 1,
                    self.steps.len()
                );
                action.trigger_dump = true;
                self.phase = CalibPhase::WaitingDump(step, Instant::now());
            }
            CalibPhase::WaitingDump(step, since) => {
                if since.elapsed() >= DUMP_TIMEOUT {
                    log_warn!(
                        "[CALIB] Dump timeout at step {} ({})",
                        step,
                        self.steps[step].label
                    );
                    self.advance_to_next(step);
                }
            }
            _ => {}
        }

        action
    }

    pub fn on_dump_received(&mut self, data: Vec<u8>) {
        if let CalibPhase::WaitingDump(step, _) = self.phase {
            let label = self.steps[step].label.clone();
            log!(
                "[CALIB] Dump received for step {}/{} ({}) — {} bytes",
                step + 1,
                self.steps.len(),
                label,
                data.len()
            );
            self.dumps.push((label, data));
            self.advance_to_next(step);
        }
    }

    fn advance_to_next(&mut self, current_step: usize) {
        let next = current_step + 1;
        if next >= self.steps.len() {
            log!("[CALIB] All steps complete. Generating report...");
            self.generate_report();
            self.phase = CalibPhase::Done;
            return;
        }

        match self.steps[next].action {
            StepAction::Baseline => {
                self.phase = CalibPhase::RequestDump(next);
            }
            StepAction::SetParam { .. } | StepAction::SetPreset { .. } => {
                self.phase = CalibPhase::SendCC(next);
            }
        }
    }

    fn generate_report(&self) {
        let dir = PathBuf::from(DUMPS_DIR);
        if let Err(e) = fs::create_dir_all(&dir) {
            log_warn!("[CALIB] Failed to create calibration dir: {}", e);
            return;
        }

        for (label, data) in &self.dumps {
            let filename = format!("{}.hex", sanitize_filename(label));
            let path = dir.join(&filename);
            let hex = format_hex_dump(data);
            if let Err(e) = fs::write(&path, &hex) {
                log_warn!("[CALIB] Failed to save {}: {}", filename, e);
            }
        }

        let baseline = self.dumps.first().map(|(_, d)| d.as_slice());
        let Some(baseline_data) = baseline else {
            log_warn!("[CALIB] No baseline dump, cannot generate diff report");
            return;
        };

        let mut report = String::from("# SysEx Calibration Report\n\n");
        report.push_str(&format!(
            "Baseline: {} bytes\n",
            baseline_data.len()
        ));
        report.push_str(&format!("Steps: {}\n\n", self.dumps.len()));

        for (i, (label, data)) in self.dumps.iter().enumerate().skip(1) {
            report.push_str(&format!("## {}\n\n", label));

            if data.len() != baseline_data.len() {
                report.push_str(&format!(
                    "Size mismatch: baseline={}, this={}\n\n",
                    baseline_data.len(),
                    data.len()
                ));
                continue;
            }

            let mut diffs: Vec<(usize, u8, u8)> = Vec::new();
            for j in 0x20..data.len() {
                if data[j] != baseline_data[j] {
                    diffs.push((j, baseline_data[j], data[j]));
                }
            }

            report.push_str(&format!(
                "Data differences (excluding header): {}\n\n",
                diffs.len()
            ));

            if !diffs.is_empty() {
                report.push_str("| Offset | Baseline | This |\n");
                report.push_str("|--------|----------|------|\n");
                for (off, bv, nv) in &diffs {
                    report.push_str(&format!(
                        "| 0x{:04X} | 0x{:02X} | 0x{:02X} |\n",
                        off, bv, nv
                    ));
                }
                report.push('\n');
            }

            let prev_dump = if i > 0 {
                Some(self.dumps[i - 1].1.as_slice())
            } else {
                None
            };
            if let Some(prev) = prev_dump {
                if prev.len() == data.len() {
                    let mut incremental: Vec<(usize, u8, u8)> = Vec::new();
                    for j in 0x20..data.len() {
                        if data[j] != prev[j] {
                            incremental.push((j, prev[j], data[j]));
                        }
                    }
                    if !incremental.is_empty() {
                        report.push_str(&format!(
                            "Incremental diff (vs previous step): {} bytes\n\n",
                            incremental.len()
                        ));
                        report.push_str("| Offset | Prev | This |\n");
                        report.push_str("|--------|------|------|\n");
                        for (off, pv, nv) in &incremental {
                            report.push_str(&format!(
                                "| 0x{:04X} | 0x{:02X} | 0x{:02X} |\n",
                                off, pv, nv
                            ));
                        }
                        report.push('\n');
                    }
                }
            }
        }

        let report_path = dir.join("calibration-report.md");
        match fs::write(&report_path, &report) {
            Ok(_) => log!("[CALIB] Report saved to {}", report_path.display()),
            Err(e) => log_warn!("[CALIB] Failed to save report: {}", e),
        }

        log!("[CALIB] Saved {} dump files to {}", self.dumps.len(), DUMPS_DIR);

        self.generate_digest(&dir);
    }

    fn generate_digest(&self, dir: &Path) {
        let pairs = self.collect_param_pairs();
        if pairs.is_empty() {
            log_warn!("[CALIB] No param pairs found for digest");
            return;
        }

        let mut digest = String::from("# SysEx Calibration Digest\n\n");
        digest.push_str(&format!(
            "Auto-extracted from {} dumps ({} param pairs).\n\n",
            self.dumps.len(),
            pairs.len()
        ));

        let mut extracted: Vec<ExtractedParam> = Vec::new();
        let mut bool_extracted: Vec<BoolExtractedParam> = Vec::new();
        let mut failed: Vec<String> = Vec::new();

        for pair in &pairs {
            digest.push_str(&format!("## {}\n\n", pair.name));

            if pair.min_idx == 0
                || pair.min_idx >= self.dumps.len()
                || pair.max_idx >= self.dumps.len()
            {
                digest.push_str("WARNING: Invalid dump indices\n\n");
                failed.push(pair.name.clone());
                continue;
            }

            let prev_dump = &self.dumps[pair.min_idx - 1].1;
            let min_dump = &self.dumps[pair.min_idx].1;
            let max_dump = &self.dumps[pair.max_idx].1;

            if prev_dump.len() != min_dump.len() || min_dump.len() != max_dump.len() {
                digest.push_str("WARNING: Dump size mismatch\n\n");
                failed.push(pair.name.clone());
                continue;
            }

            let min_diffs: Vec<usize> = (0x20..min_dump.len())
                .filter(|&i| min_dump[i] != prev_dump[i])
                .collect();
            let max_diffs: Vec<usize> = (0x20..max_dump.len())
                .filter(|&i| max_dump[i] != min_dump[i])
                .collect();

            let mut all_offsets: Vec<usize> =
                min_diffs.iter().chain(max_diffs.iter()).copied().collect();
            all_offsets.sort();
            all_offsets.dedup();

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
                all_offsets.len(),
                fmt_offsets(&all_offsets)
            ));

            let is_bool_param = pair.name.contains("_mute") || pair.name.contains("_solo") || pair.name.contains("_phantom");

            if is_bool_param {
                match try_extract_bool(&max_diffs, min_dump, max_dump) {
                    Some(mut bp) => {
                        bp.name = pair.name.clone();
                        let inv_str = if bp.inverted { " (inverted: 0x01=off)" } else { "" };
                        digest.push_str(&format!(
                            "OK (bool): offset=0x{:04X}{}\n\n",
                            bp.offset, inv_str
                        ));
                        bool_extracted.push(bp);
                    }
                    None => {
                        digest.push_str("FAILED: Could not auto-extract BoolParam\n\n");
                        failed.push(pair.name.clone());
                    }
                }
                continue;
            }

            if all_offsets.len() < 5 || all_offsets.len() > 8 {
                digest.push_str(&format!(
                    "WARNING: Expected ~5 offsets, got {}. Skipping.\n\n",
                    all_offsets.len()
                ));
                failed.push(pair.name.clone());
                continue;
            }

            match try_extract_param(&all_offsets, min_dump, max_dump) {
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
                    extracted.push(param);
                }
                None => {
                    digest.push_str("FAILED: Could not auto-extract FloatParam\n\n");
                    failed.push(pair.name.clone());
                }
            }
        }

        digest.push_str("---\n\n# Generated Rust Code\n\n");
        digest.push_str(&generate_rust_tables(&extracted));

        if !bool_extracted.is_empty() {
            digest.push_str("\n# Boolean Parameters\n\n");
            digest.push_str(&generate_rust_bool_tables(&bool_extracted));
        }

        if !failed.is_empty() {
            digest.push_str("\n# Failed Extractions\n\n");
            for name in &failed {
                digest.push_str(&format!("- {}\n", name));
            }
        }

        let path = dir.join("calibration-digest.md");
        match fs::write(&path, &digest) {
            Ok(_) => log!("[CALIB] Digest saved to {}", path.display()),
            Err(e) => log_warn!("[CALIB] Failed to save digest: {}", e),
        }
    }

    fn collect_param_pairs(&self) -> Vec<ParamPair> {
        let mut pairs = Vec::new();
        let mut i = 1;
        while i + 1 < self.dumps.len() {
            let min_label = &self.dumps[i].0;
            let max_label = &self.dumps[i + 1].0;

            if min_label.ends_with("_min") && max_label.ends_with("_max") {
                let name = min_label.trim_end_matches("_min").to_string();
                if max_label.starts_with(&name) {
                    pairs.push(ParamPair {
                        name,
                        min_idx: i,
                        max_idx: i + 1,
                    });
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }
        pairs
    }
}

// ── Digest extraction types and helpers ────────────────────────────────

struct ParamPair {
    name: String,
    min_idx: usize,
    max_idx: usize,
}

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

fn try_extract_param(
    all_offsets: &[usize],
    min_dump: &[u8],
    max_dump: &[u8],
) -> Option<ExtractedParam> {
    let subsets = if all_offsets.len() == 5 {
        vec![all_offsets.to_vec()]
    } else {
        combinations(all_offsets, 5)
    };

    for subset in &subsets {
        if let Some(result) = try_extract_from_5(subset, min_dump, max_dump) {
            return Some(result);
        }
    }
    None
}

fn try_extract_from_5(
    offsets: &[usize],
    min_dump: &[u8],
    max_dump: &[u8],
) -> Option<ExtractedParam> {
    for msb_idx in 0..5 {
        let msb_off = offsets[msb_idx];
        let data_offs: [usize; 4] = {
            let v: Vec<usize> = offsets
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != msb_idx)
                .map(|(_, &o)| o)
                .collect();
            [v[0], v[1], v[2], v[3]]
        };

        let msb_min = min_dump[msb_off];
        let msb_max = max_dump[msb_off];

        for hi_min in 0u8..16 {
            let mut bytes_min = [0u8; 4];
            for i in 0..4 {
                bytes_min[i] = min_dump[data_offs[i]] | (((hi_min >> i) & 1) << 7);
            }
            let f_min = f32::from_le_bytes(bytes_min);
            if !f_min.is_finite() || !(-500.0..=500.0).contains(&f_min) {
                continue;
            }

            for hi_max in 0u8..16 {
                let mut bytes_max = [0u8; 4];
                for i in 0..4 {
                    bytes_max[i] = max_dump[data_offs[i]] | (((hi_max >> i) & 1) << 7);
                }
                let f_max = f32::from_le_bytes(bytes_max);
                if !f_max.is_finite() || !(-500.0..=500.0).contains(&f_max) {
                    continue;
                }
                if f_min >= f_max {
                    continue;
                }

                let mut valid_bits: [Vec<u8>; 4] = [vec![], vec![], vec![], vec![]];
                for (i, bits) in valid_bits.iter_mut().enumerate() {
                    let need_hi_min = (hi_min >> i) & 1 == 1;
                    let need_hi_max = (hi_max >> i) & 1 == 1;
                    for b in 0..8u8 {
                        let has_min = (msb_min >> b) & 1 == 1;
                        let has_max = (msb_max >> b) & 1 == 1;
                        if has_min == need_hi_min && has_max == need_hi_max {
                            bits.push(b);
                        }
                    }
                }

                if let Some(bits) = find_bit_assignment(&valid_bits, 0, &mut [0; 4], 0) {
                    return Some(ExtractedParam {
                        name: String::new(),
                        msb_off,
                        data_offs,
                        bit_indices: bits,
                        min_float: f_min,
                        max_float: f_max,
                    });
                }
            }
        }
    }
    None
}

fn find_bit_assignment(
    valid_bits: &[Vec<u8>; 4],
    idx: usize,
    result: &mut [u8; 4],
    used: u8,
) -> Option<[u8; 4]> {
    if idx == 4 {
        return Some(*result);
    }
    for &b in &valid_bits[idx] {
        if used & (1 << b) != 0 {
            continue;
        }
        result[idx] = b;
        if let Some(r) = find_bit_assignment(valid_bits, idx + 1, result, used | (1 << b)) {
            return Some(r);
        }
    }
    None
}

fn combinations(items: &[usize], k: usize) -> Vec<Vec<usize>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }
    let mut result = Vec::new();
    for (i, &item) in items.iter().enumerate() {
        for mut combo in combinations(&items[i + 1..], k - 1) {
            combo.insert(0, item);
            result.push(combo);
        }
    }
    result
}

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
        ("BUS_BALANCES", "balance"),
        ("BUS_LIMITERS", "limiter"),
    ];

    let ch_prefixes = ["ch1", "ch2", "ch3", "ch4", "ch5-6", "ch7-8", "usb-bt"];
    let bus_prefixes = ["main", "mon1", "mon2", "fx1", "fx2"];

    for (const_name, suffix) in channel_groups {
        emit_table(&mut code, const_name, suffix, &ch_prefixes, 7, params);
    }
    for (const_name, suffix) in bus_groups {
        emit_table(&mut code, const_name, suffix, &bus_prefixes, 5, params);
    }

    code
}

fn generate_rust_bool_tables(params: &[BoolExtractedParam]) -> String {
    let mut code = String::new();
    let ch_prefixes = ["ch1", "ch2", "ch3", "ch4", "ch5-6", "ch7-8", "usb-bt"];

    let bool_groups: &[(&str, &str, &[&str])] = &[
        ("CHANNEL_MUTES", "mute", &ch_prefixes),
        ("CHANNEL_SOLOS", "solo", &ch_prefixes),
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

fn emit_table(
    code: &mut String,
    const_name: &str,
    suffix: &str,
    prefixes: &[&str],
    count: usize,
    params: &[ExtractedParam],
) {
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

// ── Calibration step definitions ───────────────────────────────────────

struct CcDef {
    cc: u8,
    name: &'static str,
}

const CHANNEL_CCS: &[CcDef] = &[
    CcDef { cc: 7, name: "level" },
    CcDef { cc: 8, name: "gain" },
    CcDef { cc: 10, name: "pan" },
    CcDef { cc: 11, name: "comp" },
    CcDef { cc: 9, name: "lowcut" },
    CcDef { cc: 1, name: "eq_low" },
    CcDef { cc: 2, name: "eq_lowmid" },
    CcDef { cc: 3, name: "eq_himid" },
    CcDef { cc: 4, name: "eq_hi" },
    CcDef { cc: 14, name: "send_mon1" },
    CcDef { cc: 15, name: "send_mon2" },
    CcDef { cc: 16, name: "send_fx1" },
    CcDef { cc: 17, name: "send_fx2" },
    CcDef { cc: 5, name: "mute" },
    CcDef { cc: 6, name: "solo" },
];

const BUS_CCS: &[CcDef] = &[
    CcDef { cc: 7, name: "level" },
    CcDef { cc: 8, name: "limiter" },
    CcDef { cc: 10, name: "balance" },
    CcDef { cc: 11, name: "9band_62hz" },
    CcDef { cc: 12, name: "9band_125hz" },
    CcDef { cc: 13, name: "9band_250hz" },
    CcDef { cc: 14, name: "9band_500hz" },
    CcDef { cc: 15, name: "9band_1khz" },
    CcDef { cc: 16, name: "9band_2khz" },
    CcDef { cc: 17, name: "9band_4khz" },
    CcDef { cc: 18, name: "9band_8khz" },
    CcDef { cc: 19, name: "9band_16khz" },
];

const FX_CCS: &[CcDef] = &[
    CcDef { cc: 1, name: "param1" },
    CcDef { cc: 2, name: "param2" },
];

const FX_DEFS: [(u8, &str); 2] = [(13, "fx1"), (14, "fx2")];

fn build_calibration_steps() -> Vec<CalibStep> {
    let mut steps = Vec::new();

    steps.push(CalibStep {
        action: StepAction::Baseline,
        label: "baseline".to_string(),
    });

    let channel_labels = ["ch1", "ch2", "ch3", "ch4", "ch5-6", "ch7-8", "usb-bt"];

    for cc_def in CHANNEL_CCS {
        for (midi_ch, label) in channel_labels.iter().enumerate() {
            let midi_ch = midi_ch as u8;
            steps.push(CalibStep {
                action: StepAction::SetParam {
                    midi_ch,
                    cc: cc_def.cc,
                    value: 0,
                },
                label: format!("{}_{}_min", label, cc_def.name),
            });
            steps.push(CalibStep {
                action: StepAction::SetParam {
                    midi_ch,
                    cc: cc_def.cc,
                    value: 127,
                },
                label: format!("{}_{}_max", label, cc_def.name),
            });
        }
    }

    let phantom_channels: [(u8, &str); 2] = [(0, "ch1"), (1, "ch2")];
    for (midi_ch, label) in &phantom_channels {
        steps.push(CalibStep {
            action: StepAction::SetParam {
                midi_ch: *midi_ch,
                cc: 12,
                value: 0,
            },
            label: format!("{}_phantom_min", label),
        });
        steps.push(CalibStep {
            action: StepAction::SetParam {
                midi_ch: *midi_ch,
                cc: 12,
                value: 127,
            },
            label: format!("{}_phantom_max", label),
        });
    }

    let bus_labels = [
        (7u8, "main"),
        (8, "mon1"),
        (9, "mon2"),
        (10, "fx1"),
        (11, "fx2"),
    ];

    for cc_def in BUS_CCS {
        for (midi_ch, label) in &bus_labels {
            steps.push(CalibStep {
                action: StepAction::SetParam {
                    midi_ch: *midi_ch,
                    cc: cc_def.cc,
                    value: 0,
                },
                label: format!("{}_{}_min", label, cc_def.name),
            });
            steps.push(CalibStep {
                action: StepAction::SetParam {
                    midi_ch: *midi_ch,
                    cc: cc_def.cc,
                    value: 127,
                },
                label: format!("{}_{}_max", label, cc_def.name),
            });
        }
    }

    for cc_def in FX_CCS {
        for (midi_ch, label) in &FX_DEFS {
            steps.push(CalibStep {
                action: StepAction::SetParam {
                    midi_ch: *midi_ch,
                    cc: cc_def.cc,
                    value: 0,
                },
                label: format!("{}_{}_min", label, cc_def.name),
            });
            steps.push(CalibStep {
                action: StepAction::SetParam {
                    midi_ch: *midi_ch,
                    cc: cc_def.cc,
                    value: 127,
                },
                label: format!("{}_{}_max", label, cc_def.name),
            });
        }
    }

    for (midi_ch, label) in &FX_DEFS {
        steps.push(CalibStep {
            action: StepAction::SetPreset {
                midi_ch: *midi_ch,
                program: 0,
            },
            label: format!("{}_preset_min", label),
        });
        steps.push(CalibStep {
            action: StepAction::SetPreset {
                midi_ch: *midi_ch,
                program: 15,
            },
            label: format!("{}_preset_max", label),
        });
    }

    for (midi_ch, _) in channel_labels.iter().enumerate() {
        for cc_def in CHANNEL_CCS {
            steps.push(CalibStep {
                action: StepAction::SetParam {
                    midi_ch: midi_ch as u8,
                    cc: cc_def.cc,
                    value: 64,
                },
                label: format!("restore_ch{}_{}", midi_ch, cc_def.name),
            });
        }
    }
    for (midi_ch, label) in &phantom_channels {
        steps.push(CalibStep {
            action: StepAction::SetParam {
                midi_ch: *midi_ch,
                cc: 12,
                value: 0,
            },
            label: format!("restore_{}_phantom", label),
        });
    }
    for (midi_ch, label) in &bus_labels {
        for cc_def in BUS_CCS {
            steps.push(CalibStep {
                action: StepAction::SetParam {
                    midi_ch: *midi_ch,
                    cc: cc_def.cc,
                    value: 64,
                },
                label: format!("restore_{}_{}", label, cc_def.name),
            });
        }
    }
    for (midi_ch, label) in &FX_DEFS {
        for cc_def in FX_CCS {
            steps.push(CalibStep {
                action: StepAction::SetParam {
                    midi_ch: *midi_ch,
                    cc: cc_def.cc,
                    value: 64,
                },
                label: format!("restore_{}_{}", label, cc_def.name),
            });
        }
    }

    steps.push(CalibStep {
        action: StepAction::Baseline,
        label: "final_state".to_string(),
    });

    steps
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

// ── File-based digest (same logic as bin/digest.rs) ────────────────────

const FILE_DUMPS_DIR: &str = "calibration-data";
const SEARCH_RADIUS: usize = 8;
const MIN_FLOAT_SPREAD: f32 = 0.05;
const MAX_GROUP_SPAN: usize = 8;

const CHANNEL_LABELS: [&str; 7] = ["ch1", "ch2", "ch3", "ch4", "ch5-6", "ch7-8", "usb-bt"];
const BUS_LABELS: [&str; 5] = ["main", "mon1", "mon2", "fx1", "fx2"];
const CHANNEL_PARAM_NAMES: [&str; 15] = [
    "level", "gain", "pan", "comp", "lowcut",
    "eq_low", "eq_lowmid", "eq_himid", "eq_hi",
    "send_mon1", "send_mon2", "send_fx1", "send_fx2",
    "mute", "solo",
];
const BUS_PARAM_NAMES: [&str; 12] = [
    "level", "limiter", "balance",
    "9band_62hz", "9band_125hz", "9band_250hz", "9band_500hz",
    "9band_1khz", "9band_2khz", "9band_4khz", "9band_8khz", "9band_16khz",
];
const FX_PARAM_NAMES: [&str; 3] = ["param1", "param2", "preset"];
const FX_LABELS: [&str; 2] = ["fx1", "fx2"];

pub fn run_file_based_digest() -> Result<String, String> {
    let dumps_dir = PathBuf::from(FILE_DUMPS_DIR);
    if !dumps_dir.exists() {
        return Err(format!("No calibration dumps found at {}", FILE_DUMPS_DIR));
    }

    let ordered_labels = build_file_digest_labels();
    let dumps = load_dumps_from_files(&dumps_dir, &ordered_labels);
    let loaded = dumps.iter().filter(|d| d.is_some()).count();

    let pairs = collect_file_param_pairs(&ordered_labels, &dumps);
    if pairs.is_empty() {
        return Err("No param pairs found in dump files".to_string());
    }

    let mut digest = String::from("# SysEx Calibration Digest\n\n");
    digest.push_str(&format!(
        "Extracted from {} dumps ({} param pairs).\n\n",
        loaded,
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

        let all_offsets: std::collections::BTreeSet<usize> =
            min_diffs.iter().chain(max_diffs.iter()).copied().collect();
        let all_sorted: Vec<usize> = all_offsets.into_iter().collect();

        digest.push_str(&format!(
            "Min diff ({} bytes): {}\nMax diff ({} bytes): {}\nUnion ({} offsets): {}\n\n",
            min_diffs.len(), fmt_offsets(&min_diffs),
            max_diffs.len(), fmt_offsets(&max_diffs),
            all_sorted.len(), fmt_offsets(&all_sorted),
        ));

        if all_sorted.is_empty() {
            digest.push_str("SKIP: No byte differences found\n\n");
            failed.push(pair.name.clone());
            continue;
        }

        let is_bool_param = pair.name.contains("_mute") || pair.name.contains("_solo") || pair.name.contains("_phantom");

        if is_bool_param {
            match try_extract_bool(&max_diffs, min_d, max_d) {
                Some(mut bp) => {
                    bp.name = pair.name.clone();
                    let inv_str = if bp.inverted { " (inverted: 0x01=off)" } else { "" };
                    digest.push_str(&format!(
                        "OK (bool): offset=0x{:04X}{}\n\n",
                        bp.offset, inv_str
                    ));
                    bool_extracted.push(bp);
                }
                None => {
                    digest.push_str("FAILED: Could not auto-extract BoolParam\n\n");
                    failed.push(pair.name.clone());
                }
            }
            continue;
        }

        let (valid_lo, valid_hi) = validation_range(&pair.name);

        match file_try_extract_param(&all_sorted, min_d, max_d, valid_lo, valid_hi) {
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
                extracted.push(param);
            }
            None => {
                digest.push_str("FAILED: Could not auto-extract FloatParam\n\n");
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
        digest.push_str(&format!(
            "\n# Failed Extractions ({}/{})\n\n",
            failed.len(),
            pairs.len()
        ));
        for name in &failed {
            digest.push_str(&format!("- {}\n", name));
        }
    }

    let output_dir = PathBuf::from(DUMPS_DIR);
    fs::create_dir_all(&output_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;
    let path = output_dir.join("calibration-digest.md");
    fs::write(&path, &digest).map_err(|e| format!("Failed to write digest: {}", e))?;

    let summary = format!(
        "{}/{} params extracted. Digest saved to {}",
        extracted.len(),
        pairs.len(),
        path.display()
    );
    Ok(summary)
}

fn build_file_digest_labels() -> Vec<String> {
    let mut labels = vec!["baseline".to_string()];

    for param in &CHANNEL_PARAM_NAMES {
        for ch in &CHANNEL_LABELS {
            labels.push(format!("{}_{}_min", ch, param));
            labels.push(format!("{}_{}_max", ch, param));
        }
    }
    for param in &BUS_PARAM_NAMES {
        for bus in &BUS_LABELS {
            labels.push(format!("{}_{}_min", bus, param));
            labels.push(format!("{}_{}_max", bus, param));
        }
    }
    for param in &FX_PARAM_NAMES {
        for fx in &FX_LABELS {
            labels.push(format!("{}_{}_min", fx, param));
            labels.push(format!("{}_{}_max", fx, param));
        }
    }

    let phantom_channels = ["ch1", "ch2"];
    for ch in &phantom_channels {
        labels.push(format!("{}_phantom_min", ch));
        labels.push(format!("{}_phantom_max", ch));
    }

    labels.push("final_state".to_string());
    labels
}

fn load_dumps_from_files(dir: &Path, labels: &[String]) -> Vec<Option<Vec<u8>>> {
    labels
        .iter()
        .map(|label| {
            let filename = format!("{}.hex", sanitize_filename(label));
            let path = dir.join(&filename);
            if path.exists() {
                fs::read_to_string(&path)
                    .ok()
                    .and_then(|s| parse_hex_file(&s))
            } else {
                None
            }
        })
        .collect()
}

fn parse_hex_file(text: &str) -> Option<Vec<u8>> {
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

struct FileParamPair {
    name: String,
    prev_dump: Option<Vec<u8>>,
    min_dump: Option<Vec<u8>>,
    max_dump: Option<Vec<u8>>,
}

fn collect_file_param_pairs(labels: &[String], dumps: &[Option<Vec<u8>>]) -> Vec<FileParamPair> {
    let mut pairs = Vec::new();
    let mut i = 1;
    while i + 1 < labels.len() {
        let min_label = &labels[i];
        let max_label = &labels[i + 1];
        if min_label.ends_with("_min") && max_label.ends_with("_max") {
            let name = min_label.trim_end_matches("_min").to_string();
            if max_label.starts_with(&name) {
                let prev = if i > 0 { dumps[i - 1].clone() } else { None };
                pairs.push(FileParamPair {
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
    } else if name.contains("_eq_") {
        (-20.0, 20.0)
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

fn file_try_extract_param(
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
