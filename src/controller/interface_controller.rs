use crate::{
    controller::message::InterfaceMessage,
    model::channels::{Channel, PhantomPower},
};
use iced::{
    widget::{button, column, row, text, toggler, vertical_slider, Column, Space},
    Alignment,
};

pub const CHANNEL_STRIP_WIDTH: u16 = 120;

pub fn add_channel_name<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    column.push(row![column![
        Space::with_height(20),
        text(format!("Ch. {} - {}", channel.id + 1, channel.audio_connection)),
        Space::with_height(10),
    ]])
}

pub fn add_mute_solo<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    column.push(row![
        button("Mute")
            .on_press(InterfaceMessage::Mute(channel.id))
            .padding(5),
        Space::with_width(5),
        button("Solo")
            .on_press(InterfaceMessage::Solo(channel.id))
            .padding(5),
    ])
}

pub fn add_phantom<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    if let PhantomPower::Set48v(_) = channel.phantom_pwr {
        column.push(
            row![toggler("48V".to_string(), false, |v| {
                InterfaceMessage::PhantomPower(channel.id, v.into())
            })]
            .align_items(Alignment::Center)
            .padding(25),
        )
    } else {
        column.push(row![Space::with_height(70)])
    }
}

pub fn add_vertical_slider<'a>(
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
            .height(350),
        ]
        .align_items(Alignment::Center),
    )
}

pub fn finalize_column(column: Column<'_, InterfaceMessage>) -> Column<'_, InterfaceMessage> {
    column
        .push(Space::with_height(50))
        .align_items(Alignment::Center)
}
