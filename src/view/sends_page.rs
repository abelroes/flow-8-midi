use crate::model::{flow8::FLOW8Controller, message::InterfaceMessage, page::Page};
use crate::service::mixer_state::{BLE_FX1, BLE_FX2, BLE_MON1, BLE_MON2};
use crate::view::theme::{self, PRIMARY, SECONDARY, TERTIARY};
use crate::view::widgets::{detail_chrome, format_send, h_slider};
use iced::{
    widget::{button, column, container, row, text, Space},
    Center, Element, Fill, Length,
};

pub fn view_sends(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let ch_id = controller.selected_channel;
    let Some(channel) = controller.channels.iter().find(|c| c.id == ch_id) else {
        return text("Channel not found").into();
    };
    let sends = &channel.sends;

    let chrome = detail_chrome("Main / Input / Sends", Page::Eq);

    let header = row![
        text(format!("{} — Sends", channel.display_label()))
            .size(18)
            .color(PRIMARY),
        Space::new().width(16),
        text(if channel.name.is_empty() {
            "—"
        } else {
            channel.name.as_str()
        })
        .size(12)
        .color(SECONDARY),
    ]
    .align_y(Center)
    .padding([8, 16]);

    let bus_cards = row![
        send_card("Mon 1", sends.mon1, move |v| {
            InterfaceMessage::SendMon1(ch_id, v)
        }),
        send_card("Mon 2", sends.mon2, move |v| {
            InterfaceMessage::SendMon2(ch_id, v)
        }),
        send_card("FX 1", sends.fx1, move |v| {
            InterfaceMessage::SendFx1(ch_id, v)
        }),
        send_card("FX 2", sends.fx2, move |v| {
            InterfaceMessage::SendFx2(ch_id, v)
        }),
    ]
    .spacing(12)
    .padding([8, 16])
    .width(Fill);

    let overview_bus = match controller.selected_bus {
        BLE_FX1 | BLE_FX2 | BLE_MON1 | BLE_MON2 => controller.selected_bus,
        _ => BLE_MON1,
    };

    let overview_title = column![
        text(format!("All Channels → {}", layer_name(overview_bus)))
            .size(14)
            .color(PRIMARY),
        text(format!(
            "Send levels from each input to {}",
            layer_name(overview_bus)
        ))
        .size(12)
        .color(SECONDARY),
    ]
    .padding([12, 16]);

    let mut overview = row![].spacing(8).padding([0, 16]).width(Fill);
    for ch in &controller.channels {
        overview = overview.push(mini_channel_send(ch, overview_bus));
    }

    column![
        chrome,
        header,
        bus_cards,
        overview_title,
        overview,
        Space::new().height(20),
    ]
    .width(Fill)
    .into()
}

fn layer_name(bus: u8) -> &'static str {
    match bus {
        BLE_FX1 => "FX 1",
        BLE_FX2 => "FX 2",
        BLE_MON1 => "Mon 1",
        BLE_MON2 => "Mon 2",
        _ => "Mon 1",
    }
}

fn send_card<'a, F>(title: &'a str, value: u8, on_change: F) -> Element<'a, InterfaceMessage>
where
    F: Fn(u8) -> InterfaceMessage + 'a,
{
    container(
        column![
            text(title).size(14).color(PRIMARY),
            Space::new().height(8),
            text(format_send(value)).size(13).color(SECONDARY),
            Space::new().height(8),
            h_slider(0..=127, value, on_change, format_send(value)),
            Space::new().height(10),
            text("Post-fader").size(11).color(TERTIARY),
            Space::new().height(8),
            button(text("Mute").size(12).center())
                .padding([6, 12])
                .style(theme::btn(theme::BtnKind::Secondary)),
        ]
        .padding(14)
        .width(Fill),
    )
    .style(theme::surface_box)
    .width(Fill)
    .into()
}

fn mini_channel_send(
    channel: &crate::model::channels::Channel,
    bus: u8,
) -> Element<'_, InterfaceMessage> {
    let ch_id = channel.id;
    let value = match bus {
        BLE_MON2 => channel.sends.mon2,
        BLE_FX1 => channel.sends.fx1,
        BLE_FX2 => channel.sends.fx2,
        _ => channel.sends.mon1,
    };

    let slider: Element<'_, InterfaceMessage> = match bus {
        BLE_MON2 => h_slider(
            0..=127,
            value,
            move |v| InterfaceMessage::SendMon2(ch_id, v),
            format_send(value),
        ),
        BLE_FX1 => h_slider(
            0..=127,
            value,
            move |v| InterfaceMessage::SendFx1(ch_id, v),
            format_send(value),
        ),
        BLE_FX2 => h_slider(
            0..=127,
            value,
            move |v| InterfaceMessage::SendFx2(ch_id, v),
            format_send(value),
        ),
        _ => h_slider(
            0..=127,
            value,
            move |v| InterfaceMessage::SendMon1(ch_id, v),
            format_send(value),
        ),
    };

    container(
        column![
            text(channel.display_label()).size(12).color(PRIMARY),
            Space::new().height(6),
            text(format_send(value)).size(11).color(SECONDARY),
            Space::new().height(6),
            slider,
            Space::new().height(8),
            button(text("M").size(11).center())
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(24.0))
                .style(theme::btn(theme::BtnKind::Secondary)),
        ]
        .padding(10)
        .align_x(Center)
        .width(Fill),
    )
    .style(theme::surface_box)
    .width(Fill)
    .into()
}
