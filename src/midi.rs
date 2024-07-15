// MIDI protocol constants
const CONTROL_CHANGE_PREFIX: u8 = 0xB0;
const FLOW_DEVICE_STR: &str = "FLOW 8 MIDI OUT";
const FLOW_CLIENT_NAME: &str = "FLOW 8 Midi Controller";

use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort, MidiOutputPorts};

pub fn get_midi_conn(is_optional: bool) -> Option<MidiOutputConnection> {
    if is_optional {
        return None;
    };

    let midi_out = get_midi_output();
    let midi_ports = get_midi_output_ports(&midi_out);
    let port_num = get_flow_midi_port(&midi_out, &midi_ports);

    let biding = midi_out.ports();
    let device_port = match biding.get(port_num) {
        Some(port) => Some(port),
        None => panic!("Invalid output port selected"),
    };

    let conn_out = match midi_out.connect(device_port.unwrap(), "midir-test") {
        Ok(conn) => conn,
        Err(_) => panic!("Couldn't connect to Midi port/device"),
    };

    Some(conn_out)
}

pub fn get_midi_output() -> MidiOutput {
    match MidiOutput::new(FLOW_CLIENT_NAME) {
        Ok(conn) => conn,
        Err(_) => panic!("Couldn't connect to FLOW 8 device"),
    }
}

pub fn get_midi_output_ports(midi_output: &MidiOutput) -> MidiOutputPorts {
    midi_output.ports()
}

pub fn get_port_name(midi_output: &MidiOutput, midi_port: &MidiOutputPort) -> Option<String> {
    match midi_output.port_name(midi_port) {
        Ok(name) => Some(name),
        Err(_) => None,
    }
}

pub fn get_flow_midi_port(midi_output: &MidiOutput, midi_ports: &MidiOutputPorts) -> usize {
    let mut port_num = 0;
    for (i, p) in midi_ports.iter().enumerate() {
        let name = get_port_name(&midi_output, p);
        if name != None {
            if name.unwrap() == FLOW_DEVICE_STR.to_string() {
                port_num = i
            }
        };
    }
    port_num
}

pub fn send_cc(mut midi_conn: MidiOutputConnection, midi_channel: u8, cc: u8, value: u8) {
    midi_conn
        .send(&[CONTROL_CHANGE_PREFIX | midi_channel, cc, value])
        .unwrap_or_else(|e| {
            eprintln!(
                "Failed to send CC#{} value {} on ch#{}: {:?}",
                cc,
                value,
                midi_channel + 1,
                e
            )
        });
}
