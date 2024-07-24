use crate::{
    midi::send_cc,
    model::{
        channels::{Bus, Channel, PhantomPowerType},
        message::InterfaceMessage,
    },
    utils::bool_to_u8,
};
use iced::{
    widget::{
        button, column, row, text, toggler, tooltip, tooltip::Position, vertical_slider, Column,
        Space,
    },
    Alignment,
};
use midir::MidiOutputConnection;

pub const CHANNEL_STRIP_WIDTH: u16 = 120;

pub fn add_channel<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    let mut column = add_channel_name(column, channel);
    column = add_mute_solo(column, channel);
    column = add_phantom(column, channel);
    column = add_channel_vertical_slider(column, channel);
    column
}

pub fn add_channel_name<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    column.push(row![column![
        Space::with_height(20),
        text(format!(
            "Ch. {} - {}",
            channel.id + 1,
            channel.audio_connection
        )),
        Space::with_height(10),
    ]])
}

pub fn add_mute_solo<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    column.push(row![
        button("Mute")
            .on_press(InterfaceMessage::Mute(channel.id, channel.is_muted,))
            .padding(5),
        Space::with_width(5),
        button("Solo")
            .on_press(InterfaceMessage::Solo(channel.id, channel.is_soloed,))
            .padding(5),
    ])
}

pub fn add_phantom<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    if let PhantomPowerType::Set48v(_) = channel.phantom_pwr.phantom_power_type {
        column.push(
            row![tooltip(
                toggler("48V".to_string(), channel.phantom_pwr.is_on, |_| {
                    InterfaceMessage::PhantomPower(
                        channel.id,
                        channel.phantom_pwr.is_on,
                        channel.phantom_pwr.is_confirmed,
                    )
                }),
                "Click twice if you're sure",
                Position::FollowCursor,
            )
            .gap(5)]
            .align_items(Alignment::Center)
            .padding(25),
        )
    } else {
        column.push(row![Space::with_height(70)])
    }
}

pub fn add_channel_vertical_slider<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    column.push(
        column![
            text("Level"),
            Space::with_height(5),
            vertical_slider(1..=127, channel.channel_strip.level, |v| {
                InterfaceMessage::Level(channel.id, v)
            })
            .height(300),
        ]
        .align_items(Alignment::Center),
    )
}

pub fn finalize_column(column: Column<'_, InterfaceMessage>) -> Column<'_, InterfaceMessage> {
    column
        .push(Space::with_height(50))
        .align_items(Alignment::Center)
}

pub fn add_bus<'a>(
    column: Column<'a, InterfaceMessage>,
    bus: &'a Bus,
) -> Column<'a, InterfaceMessage> {
    let mut column = add_bus_name(column, bus);
    column = add_bus_vertical_slider(column, bus);
    column
}

pub fn add_bus_name<'a>(
    column: Column<'a, InterfaceMessage>,
    bus: &'a Bus,
) -> Column<'a, InterfaceMessage> {
    column.push(row![column![
        Space::with_height(20),
        text(format!("{}", bus.bus_type)),
        Space::with_height(10),
    ]])
}

pub fn add_bus_vertical_slider<'a>(
    column: Column<'a, InterfaceMessage>,
    bus: &'a Bus,
) -> Column<'a, InterfaceMessage> {
    column.push(
        column![
            text("Level"),
            Space::with_height(10),
            vertical_slider(1..=127, bus.bus_strip.level, |v| {
                InterfaceMessage::BusLevel(bus.index, bus.id, v)
            })
            .height(400),
        ]
        .align_items(Alignment::Center),
    )
}

pub fn match_midi_command(message: InterfaceMessage, midi_conn: &mut MidiOutputConnection) {
    match message {
        InterfaceMessage::Mute(chn_id, value) => send_cc(midi_conn, chn_id, 5, bool_to_u8(!value)),
        InterfaceMessage::Solo(chn_id, value) => send_cc(midi_conn, chn_id, 6, bool_to_u8(!value)),
        InterfaceMessage::Gain(chn_id, value) => send_cc(midi_conn, chn_id, 8, value),
        InterfaceMessage::Level(bus_id, value) | InterfaceMessage::BusLevel(_, bus_id, value) => {
            send_cc(midi_conn, bus_id, 7, value)
        }
        InterfaceMessage::Balance(bus_id, value)
        | InterfaceMessage::BusBalance(_, bus_id, value) => send_cc(midi_conn, bus_id, 10, value),
        InterfaceMessage::PhantomPower(chn_id, value, is_confirmed) => {
            if is_confirmed {
                send_cc(midi_conn, chn_id, 12, bool_to_u8(!value))
            }
        }
        InterfaceMessage::Compressor(chn_id, value) => send_cc(midi_conn, chn_id, 11, value),
        InterfaceMessage::EqLow(chn_id, value) => send_cc(midi_conn, chn_id, 1, value),
        InterfaceMessage::EqLowMid(chn_id, value) => send_cc(midi_conn, chn_id, 2, value),
        InterfaceMessage::EqHiMid(chn_id, value) => send_cc(midi_conn, chn_id, 3, value),
        InterfaceMessage::EqHi(chn_id, value) => send_cc(midi_conn, chn_id, 4, value),
    }
}
