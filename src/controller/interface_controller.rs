use crate::{
    controller::message::InterfaceMessage,
    model::channels::{Channel, PhantomPower},
};
use iced::{
    widget::{button, column, row, text, toggler, vertical_slider, Column, Space},
    Alignment,
};

pub fn add_channel_name<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    column.push(row![
        Space::with_height(100),
        text(format!("Ch. {}", channel.id))
    ])
}

pub fn add_mute_solo<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    column.push(
        row![
            button("Mute")
                .on_press(InterfaceMessage::Mute(channel.id))
                .padding(5),
            Space::with_width(5),
            button("Solo")
                .on_press(InterfaceMessage::Solo(channel.id))
                .padding(5),
        ]
        .padding(10),
    )
}

pub fn add_vertical_slider<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    column.push(vertical_slider(1..=127, channel.channel_strip.level, |v| {
        InterfaceMessage::Level(channel.id, v)
    }))
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
            .padding(10),
        )
    } else {
        column.push(Space::with_height(5))
    }
}

pub fn finalize_column(column: Column<'_, InterfaceMessage>) -> Column<'_, InterfaceMessage> {
    column
        .push(Space::with_height(50))
        .align_items(Alignment::Center)
}
