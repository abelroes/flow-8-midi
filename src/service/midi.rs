use crate::{log, log_debug};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::sync::mpsc;

const NOTE_ON_PREFIX: u8 = 0x90;
const CONTROL_CHANGE_PREFIX: u8 = 0xB0;
const PROGRAM_CHANGE_PREFIX: u8 = 0xC0;
const SYSEX_START: u8 = 0xF0;
const FLOW_CLIENT_NAME: &str = "FLOW 8 Midi Controller";
const FLOW_DEVICE_KEYWORD: &str = "FLOW 8";

pub fn create_sysex_channel() -> (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
    mpsc::channel()
}


#[derive(Clone, Debug)]
pub struct MidiDeviceInfo {
    pub index: usize,
    pub name: String,
    pub is_flow8: bool,
}

pub fn list_midi_output_devices() -> Vec<MidiDeviceInfo> {
    let midi_out = match MidiOutput::new(FLOW_CLIENT_NAME) {
        Ok(out) => out,
        Err(e) => {
            log!("[MIDI] Failed to initialize MIDI output: {:?}", e);
            return vec![];
        }
    };

    let ports = midi_out.ports();
    log!("[MIDI] Found {} output port(s):", ports.len());

    let mut devices: Vec<MidiDeviceInfo> = ports
        .iter()
        .enumerate()
        .filter_map(|(i, port)| {
            midi_out.port_name(port).ok().map(|name| {
                let is_flow8 = name.to_uppercase().contains(&FLOW_DEVICE_KEYWORD.to_uppercase());
                log!("  [{}] \"{}\" {}", i, name, if is_flow8 { "<-- FLOW 8" } else { "" });
                MidiDeviceInfo {
                    index: i,
                    name,
                    is_flow8,
                }
            })
        })
        .collect();

    devices.sort_by(|a, b| b.is_flow8.cmp(&a.is_flow8));
    devices
}

/// Investigation notes (FLOW 8 MIDI Input):
///   - The FLOW 8 has one-way MIDI communication: it only receives CC/PC, it does NOT send
///     CC messages when physical faders are moved.
///   - Firmware v11739+ added a SysEx state dump feature, but it must be triggered manually
///     from the mixer's Snapshots menu — it cannot be requested via MIDI.
///   - Therefore, automatic state synchronization (reading the mixer's current state on connect)
///     is NOT feasible with the standard MIDI protocol on this device.
pub fn list_midi_input_devices() -> Vec<MidiDeviceInfo> {
    let midi_in = match MidiInput::new(FLOW_CLIENT_NAME) {
        Ok(inp) => inp,
        Err(e) => {
            log!("[MIDI] Failed to initialize MIDI input: {:?}", e);
            return vec![];
        }
    };

    let ports = midi_in.ports();
    log!("[MIDI] Found {} input port(s):", ports.len());

    ports
        .iter()
        .enumerate()
        .filter_map(|(i, port)| {
            midi_in.port_name(port).ok().map(|name| {
                let is_flow8 = name.to_uppercase().contains(&FLOW_DEVICE_KEYWORD.to_uppercase());
                log!("  [{}] \"{}\" {}", i, name, if is_flow8 { "<-- FLOW 8" } else { "" });
                MidiDeviceInfo {
                    index: i,
                    name,
                    is_flow8,
                }
            })
        })
        .collect()
}

pub fn connect_to_device(device_index: usize) -> Result<MidiOutputConnection, String> {
    log!("[MIDI] Connecting to output device index {}...", device_index);
    let midi_out =
        MidiOutput::new(FLOW_CLIENT_NAME).map_err(|e| format!("MIDI output error: {}", e))?;

    let ports = midi_out.ports();
    let port = ports
        .get(device_index)
        .ok_or_else(|| "Invalid port index".to_string())?;

    let conn = midi_out
        .connect(port, "flow8-midi-controller")
        .map_err(|e| format!("Connection failed: {}", e))?;

    log!("[MIDI] Output connected successfully");
    Ok(conn)
}

pub fn connect_input_device(
    device_index: usize,
    sysex_tx: mpsc::Sender<Vec<u8>>,
) -> Result<MidiInputConnection<()>, String> {
    log!("[MIDI] Connecting to input device index {}...", device_index);
    let midi_in =
        MidiInput::new(FLOW_CLIENT_NAME).map_err(|e| format!("MIDI input error: {}", e))?;

    let ports = midi_in.ports();
    let port = ports
        .get(device_index)
        .ok_or_else(|| "Invalid input port index".to_string())?;

    let conn = midi_in
        .connect(
            port,
            "flow8-midi-input",
            move |timestamp, message, _| {
                if !message.is_empty() && message[0] == SYSEX_START {
                    log!(
                        "[MIDI IN] SysEx received ({} bytes) ts={}",
                        message.len(),
                        timestamp
                    );
                    let _ = sysex_tx.send(message.to_vec());
                } else {
                    log!("[MIDI IN] ts={} data={:02X?}", timestamp, message);
                }
            },
            (),
        )
        .map_err(|e| format!("Input connection failed: {}", e))?;

    log!("[MIDI] Input connected successfully");
    Ok(conn)
}

pub fn send_cc(midi_conn: &mut MidiOutputConnection, midi_channel: u8, cc: u8, value: u8) {
    log_debug!(
        "[MIDI] CC#{} val={} ch#{}",
        cc,
        value,
        midi_channel + 1
    );
    midi_conn
        .send(&[CONTROL_CHANGE_PREFIX | midi_channel, cc, value])
        .unwrap_or_else(|e| {
            log!(
                "Failed to send CC#{} value {} on ch#{}: {:?}",
                cc,
                value,
                midi_channel + 1,
                e
            )
        });
}

pub fn send_note_on(midi_conn: &mut MidiOutputConnection, midi_channel: u8, note: u8, velocity: u8) {
    log_debug!(
        "[MIDI] NoteOn note={} vel={} ch#{}",
        note,
        velocity,
        midi_channel + 1
    );
    midi_conn
        .send(&[NOTE_ON_PREFIX | midi_channel, note, velocity])
        .unwrap_or_else(|e| {
            log!(
                "Failed to send NoteOn note={} vel={} on ch#{}: {:?}",
                note,
                velocity,
                midi_channel + 1,
                e
            )
        });
}

pub fn send_program_change(midi_conn: &mut MidiOutputConnection, midi_channel: u8, program: u8) {
    log_debug!(
        "[MIDI] PC={} ch#{}",
        program,
        midi_channel + 1
    );
    midi_conn
        .send(&[PROGRAM_CHANGE_PREFIX | midi_channel, program])
        .unwrap_or_else(|e| {
            log!(
                "Failed to send PC {} on ch#{}: {:?}",
                program,
                midi_channel + 1,
                e
            )
        });
}
