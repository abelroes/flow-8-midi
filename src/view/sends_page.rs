use crate::model::{
    channels::Channel,
    flow8::FLOW8Controller,
    message::InterfaceMessage,
};
use crate::view::widgets::{sync_dot, v_slider, format_send};
use iced::{
    widget::{column, container, row, text, Column, Row, Space},
    Center, Element, Fill,
};

const SEND_FADER_HEIGHT: f32 = 250.0;

pub fn view_sends(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let children: Vec<Element<InterfaceMessage>> = controller
        .channels
        .iter()
        .map(|c| build_channel_sends(c))
        .collect();

    Row::with_children(children)
        .spacing(2)
        .width(Fill)
        .height(Fill)
        .padding([0, 8])
        .into()
}

fn build_channel_sends(channel: &Channel) -> Element<'_, InterfaceMessage> {
    let ch_id = channel.id;
    let sends = &channel.sends;

    let mon_group = column![
        row![
            send_fader("1", sends.mon1, sends.mon1_synced, move |v| {
                InterfaceMessage::SendMon1(ch_id, v)
            }),
            send_fader("2", sends.mon2, sends.mon2_synced, move |v| {
                InterfaceMessage::SendMon2(ch_id, v)
            }),
        ]
        .spacing(10)
        .align_y(iced::Alignment::End),
        text("Monitor").size(10),
    ]
    .align_x(Center)
    .spacing(4);

    let fx_group = column![
        row![
            send_fader("1", sends.fx1, sends.fx1_synced, move |v| {
                InterfaceMessage::SendFx1(ch_id, v)
            }),
            send_fader("2", sends.fx2, sends.fx2_synced, move |v| {
                InterfaceMessage::SendFx2(ch_id, v)
            }),
        ]
        .spacing(10)
        .align_y(iced::Alignment::End),
        text("FX").size(10),
    ]
    .align_x(Center)
    .spacing(4);

    let mut header: Column<'_, InterfaceMessage> = column![text(channel.display_label()).size(13)]
        .align_x(Center);
    if !channel.name.is_empty() {
        header = header.push(text(&channel.name).size(11));
    }

    let content = column![
        header,
        Space::new().height(8),
        row![mon_group, Space::new().width(12), fx_group]
            .align_y(iced::Alignment::End),
    ]
    .align_x(Center)
    .spacing(2)
    .padding([8, 6])
    .width(Fill);

    container(content)
        .style(container::rounded_box)
        .width(Fill)
        .padding(2)
        .into()
}

fn send_fader<'a, F>(
    label: &'a str,
    value: u8,
    is_synced: bool,
    on_change: F,
) -> Column<'a, InterfaceMessage>
where
    F: Fn(u8) -> InterfaceMessage + 'a,
{
    column![
        sync_dot(is_synced),
        v_slider(0..=127, value, SEND_FADER_HEIGHT, on_change, format_send(value)),
        text(label).size(11),
    ]
    .align_x(Center)
    .spacing(4)
}
