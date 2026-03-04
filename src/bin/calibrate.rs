#[path = "../logger.rs"]
pub mod logger;
#[path = "../service/midi.rs"]
pub mod midi;
#[path = "../service/ble.rs"]
pub mod ble;

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

const SETTLE_TIME: Duration = Duration::from_millis(800);
const DUMP_TIMEOUT: Duration = Duration::from_secs(5);
const OUTPUT_DIR: &str = "calibration-data";

const CHANNEL_LABELS: [&str; 7] = ["ch1", "ch2", "ch3", "ch4", "ch5-6", "ch7-8", "usb-bt"];
const BUS_DEFS: [(u8, &str); 5] = [(7, "main"), (8, "mon1"), (9, "mon2"), (10, "fx1"), (11, "fx2")];
const FX_DEFS: [(u8, &str); 2] = [(13, "fx1"), (14, "fx2")];

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

const PHANTOM_CHANNELS: [(u8, &str); 2] = [(0, "ch1"), (1, "ch2")];

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

fn main() {
    logger::init();
    println!("=== FLOW 8 SysEx Calibration (CLI) ===\n");

    let mut midi_conn = connect_midi_output();
    let sysex_rx = connect_midi_input();
    let ble_conn = connect_ble();

    let output_dir = PathBuf::from(OUTPUT_DIR);
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let steps = build_steps();
    let dump_count = steps.iter().filter(|s| s.request_dump).count();
    let total = steps.len();
    println!(
        "Starting calibration: {} steps ({} dumps, ~{}s estimated)\n",
        total,
        dump_count,
        dump_count * 2
    );

    let mut saved = 0usize;
    let mut timeouts = 0usize;

    for (i, step) in steps.iter().enumerate() {
        if step.request_dump {
            print!("  [{:>3}/{}] {:<35} ", i + 1, total, step.label);
        }

        if let Some((ch, cc, val)) = step.send_cc {
            midi::send_cc(&mut midi_conn, ch, cc, val);
        }
        if let Some((ch, program)) = step.send_pc {
            midi::send_program_change(&mut midi_conn, ch, program);
        }
        if step.request_dump && (step.send_cc.is_some() || step.send_pc.is_some()) {
            std::thread::sleep(SETTLE_TIME);
        }

        if step.request_dump {
            drain_pending(&sysex_rx);

            ble::send_dump_trigger(&ble_conn).expect("Dump trigger failed");

            match sysex_rx.recv_timeout(DUMP_TIMEOUT) {
                Ok(data) => {
                    let filename = format!("{}.hex", sanitize(&step.label));
                    let path = output_dir.join(&filename);
                    fs::write(&path, format_hex_dump(&data)).expect("Failed to write dump");
                    println!("OK ({} bytes)", data.len());
                    saved += 1;
                }
                Err(_) => {
                    println!("TIMEOUT");
                    timeouts += 1;
                }
            }
        }
    }

    ble::disconnect(&ble_conn);

    println!("\n=== Calibration complete ===");
    println!("  Dumps saved: {}/{}", saved, dump_count);
    if timeouts > 0 {
        println!("  Timeouts: {}", timeouts);
    }
    println!("  Output: {}/", OUTPUT_DIR);
    println!("\nRun `make digest` to extract parameters.");
}

fn connect_midi_output() -> midir::MidiOutputConnection {
    println!("Connecting MIDI output...");
    let devices = midi::list_midi_output_devices();
    let flow8 = devices
        .iter()
        .find(|d| d.is_flow8)
        .expect("FLOW 8 not found on MIDI output. Connect it via USB and retry.");
    println!("  Found: \"{}\"", flow8.name);
    midi::connect_to_device(flow8.index).expect("Failed to connect MIDI output")
}

fn connect_midi_input() -> mpsc::Receiver<Vec<u8>> {
    println!("Connecting MIDI input...");
    let devices = midi::list_midi_input_devices();
    let flow8 = devices
        .iter()
        .find(|d| d.is_flow8)
        .expect("FLOW 8 not found on MIDI input.");
    println!("  Found: \"{}\"", flow8.name);
    let (tx, rx) = midi::create_sysex_channel();
    let _conn = midi::connect_input_device(flow8.index, tx)
        .expect("Failed to connect MIDI input");
    std::mem::forget(_conn);
    rx
}

fn connect_ble() -> ble::BleConnection {
    println!("Connecting BLE...");
    let (status_tx, status_rx) = mpsc::channel();
    let (snapshot_tx, _snapshot_rx) = mpsc::channel();

    let handle = std::thread::spawn(move || ble::connect_flow8_ble(status_tx, snapshot_tx));

    while let Ok(status) = status_rx.recv() {
        println!("  BLE: {}", status);
        if status == ble::BleStatus::Connected || status == ble::BleStatus::Error {
            break;
        }
    }

    handle
        .join()
        .expect("BLE thread panicked")
        .expect("Failed to connect BLE")
}

fn drain_pending(rx: &mpsc::Receiver<Vec<u8>>) {
    while rx.try_recv().is_ok() {}
}

struct CalibStep {
    label: String,
    send_cc: Option<(u8, u8, u8)>,
    send_pc: Option<(u8, u8)>,
    request_dump: bool,
}

fn build_steps() -> Vec<CalibStep> {
    let mut steps = Vec::new();

    steps.push(CalibStep {
        label: "baseline".to_string(),
        send_cc: None,
        send_pc: None,
        request_dump: true,
    });

    for cc_def in CHANNEL_CCS {
        for (midi_ch, label) in CHANNEL_LABELS.iter().enumerate() {
            steps.push(CalibStep {
                label: format!("{}_{}_min", label, cc_def.name),
                send_cc: Some((midi_ch as u8, cc_def.cc, 0)),
                send_pc: None,
                request_dump: true,
            });
            steps.push(CalibStep {
                label: format!("{}_{}_max", label, cc_def.name),
                send_cc: Some((midi_ch as u8, cc_def.cc, 127)),
                send_pc: None,
                request_dump: true,
            });
        }
    }

    for cc_def in BUS_CCS {
        for (midi_ch, label) in &BUS_DEFS {
            steps.push(CalibStep {
                label: format!("{}_{}_min", label, cc_def.name),
                send_cc: Some((*midi_ch, cc_def.cc, 0)),
                send_pc: None,
                request_dump: true,
            });
            steps.push(CalibStep {
                label: format!("{}_{}_max", label, cc_def.name),
                send_cc: Some((*midi_ch, cc_def.cc, 127)),
                send_pc: None,
                request_dump: true,
            });
        }
    }

    for cc_def in FX_CCS {
        for (midi_ch, label) in &FX_DEFS {
            steps.push(CalibStep {
                label: format!("{}_{}_min", label, cc_def.name),
                send_cc: Some((*midi_ch, cc_def.cc, 0)),
                send_pc: None,
                request_dump: true,
            });
            steps.push(CalibStep {
                label: format!("{}_{}_max", label, cc_def.name),
                send_cc: Some((*midi_ch, cc_def.cc, 127)),
                send_pc: None,
                request_dump: true,
            });
        }
    }

    for (midi_ch, label) in &FX_DEFS {
        steps.push(CalibStep {
            label: format!("{}_preset_min", label),
            send_cc: None,
            send_pc: Some((*midi_ch, 0)),
            request_dump: true,
        });
        steps.push(CalibStep {
            label: format!("{}_preset_max", label),
            send_cc: None,
            send_pc: Some((*midi_ch, 15)),
            request_dump: true,
        });
    }

    for (midi_ch, label) in &PHANTOM_CHANNELS {
        steps.push(CalibStep {
            label: format!("{}_phantom_min", label),
            send_cc: Some((*midi_ch, 12, 0)),
            send_pc: None,
            request_dump: true,
        });
        steps.push(CalibStep {
            label: format!("{}_phantom_max", label),
            send_cc: Some((*midi_ch, 12, 127)),
            send_pc: None,
            request_dump: true,
        });
    }

    for (midi_ch, _) in CHANNEL_LABELS.iter().enumerate() {
        for cc_def in CHANNEL_CCS {
            steps.push(CalibStep {
                label: format!("restore_ch{}_{}", midi_ch, cc_def.name),
                send_cc: Some((midi_ch as u8, cc_def.cc, 64)),
                send_pc: None,
                request_dump: false,
            });
        }
    }
    for (midi_ch, label) in &PHANTOM_CHANNELS {
        steps.push(CalibStep {
            label: format!("restore_{}_phantom", label),
            send_cc: Some((*midi_ch, 12, 0)),
            send_pc: None,
            request_dump: false,
        });
    }
    for (midi_ch, label) in &BUS_DEFS {
        for cc_def in BUS_CCS {
            steps.push(CalibStep {
                label: format!("restore_{}_{}", label, cc_def.name),
                send_cc: Some((*midi_ch, cc_def.cc, 64)),
                send_pc: None,
                request_dump: false,
            });
        }
    }
    for (midi_ch, label) in &FX_DEFS {
        for cc_def in FX_CCS {
            steps.push(CalibStep {
                label: format!("restore_{}_{}", label, cc_def.name),
                send_cc: Some((*midi_ch, cc_def.cc, 64)),
                send_pc: None,
                request_dump: false,
            });
        }
    }

    steps.push(CalibStep {
        label: "final_state".to_string(),
        send_cc: None,
        send_pc: None,
        request_dump: true,
    });

    steps
}

fn format_hex_dump(data: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if (0x20..=0x7E).contains(&b) { b as char } else { '.' })
            .collect();
        out.push_str(&format!(
            "{:04X}: {:48} | {}\n",
            i * 16,
            hex.join(" "),
            ascii
        ));
    }
    out
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
