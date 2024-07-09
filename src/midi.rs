// MIDI protocol constants
const CONTROL_CHANGE_PREFIX: u8 = 0xB0;

use std::error::Error;
use std::io::{stdin, stdout, Write};

use midir::{Ignore, MidiInput, MidiOutput};

pub fn run() -> Result<(), Box<dyn Error>> {
    let mut midi_in = MidiInput::new("midir test input")?;
    midi_in.ignore(Ignore::None);
    let midi_out = MidiOutput::new("midir test output")?;

    let mut input = String::new();

    println!("Available input ports:");
    for (i, p) in midi_in.ports().iter().enumerate() {
        println!("{}: {}", i, midi_in.port_name(p)?);
    }

    println!("\nAvailable output ports:");
    for (i, p) in midi_out.ports().iter().enumerate() {
        println!("{}: {}", i, midi_out.port_name(p)?);
    }

    let port_num = 1;
    let binding = midi_out.ports();
    let device_port = binding
        .get(port_num)
        .ok_or("invalid output port selected")?;
    let midi_channel = 0u8;

    println!("\nOpening connection");
    let mut conn_out = midi_out.connect(&device_port, "midir-test")?;
    println!("Connection open.");

    loop {
        print!("\nPress <enter> to MUTE ...");
        stdout().flush()?;
        input.clear();
        stdin().read_line(&mut input)?;

        // Mute CH 1
        let cc = 5u8;
        let value = 1u8;
        conn_out
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

        print!("\nPress <enter> to UNMUTE ...");
        stdout().flush()?;
        input.clear();
        stdin().read_line(&mut input)?;

        let value = 0u8;
        conn_out
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

        // run in endless loop if "--loop" parameter is specified
        match ::std::env::args().nth(1) {
            Some(ref arg) if arg == "--loop" => {}
            _ => break,
        }
        stdout().flush()?;
        input.clear();
        println!("\n");
    }

    Ok(())
}
