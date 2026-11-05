use crate::model::{
    channels::{Bus, BusType, Channel},
    flow8::FLOW8Controller,
    message::InterfaceMessage,
};
use crate::service::mixer_state::{BLE_FX1, BLE_FX2, BLE_MON1, BLE_MON2};
use crate::view::theme::{
    self, BUS_STRIP_WIDTH, FADER_HEIGHT, MON_LINK, PRIMARY, SECONDARY, STRIP_WIDTH, TERTIARY,
};
use crate::view::widgets::{
    bus_selector_row, dual_meter, format_level, format_send, v_slider, BUS_MIX,
};
use iced::{
    widget::{button, column, container, row, text, Column, Space},
    Center, Element, Fill, Length,
};

pub fn view_mixer(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let bus_bar = container(bus_selector_row(controller.selected_bus))
        .width(Fill)
        .padding([8, 12]);

    let channels_row: Vec<Element<InterfaceMessage>> = controller
        .channels
        .iter()
        .map(|c| build_channel_strip(controller, c))
        .collect();

    let channels_section = iced::widget::Row::with_children(channels_row)
        .spacing(8)
        .width(Length::Shrink);

    let bus_strips: Vec<Element<InterfaceMessage>> = controller
        .buses
        .iter()
        .filter(|b| b.bus_type != BusType::Fx)
        .map(|b| build_bus_strip(controller, b))
        .collect();

    let buses_section = iced::widget::Row::with_children(bus_strips)
        .spacing(8)
        .width(Length::Shrink);

    container(
        column![
            bus_bar,
            row![
                channels_section,
                Space::new().width(16),
                buses_section,
            ]
            .padding([0, 12])
            .width(Fill),
        ]
        .width(Fill)
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(theme::app_box)
    .into()
}

fn layer_label(selected: u8) -> &'static str {
    match selected {
        BLE_MON1 => "Mon1",
        BLE_MON2 => "Mon2",
        BLE_FX1 => "FX1",
        BLE_FX2 => "FX2",
        BUS_MIX => "Mix",
        _ => "Main",
    }
}

fn build_channel_strip<'a>(
    controller: &'a FLOW8Controller,
    channel: &'a Channel,
) -> Element<'a, InterfaceMessage> {
    let ch_id = channel.id;
    let idx = channel.id as usize;
    let left = controller.meter.stereo_left(idx);
    let right = controller.meter.stereo_right(idx);
    let bus = controller.selected_bus;

    let id_color = PRIMARY;
    let _ = MON_LINK;

    let mut col = Column::new()
        .width(Length::Fixed(STRIP_WIDTH))
        .align_x(Center)
        .spacing(0)
        .padding([8, 4]);

    col = col.push(
        text(channel.display_label())
            .size(11)
            .color(id_color)
            .center(),
    );
    col = col.push(Space::new().height(4));
    col = col.push(
        text(if channel.name.is_empty() {
            "—"
        } else {
            channel.name.as_str()
        })
        .size(10)
        .color(SECONDARY)
        .center(),
    );
    col = col.push(Space::new().height(10));

    col = col.push(dual_meter(
        left.level_normalized(),
        right.map(|r| r.level_normalized()),
    ));
    col = col.push(Space::new().height(10));

    let mute = button(text("M").size(12).center())
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(26.0))
        .on_press(InterfaceMessage::Mute(ch_id, channel.is_muted))
        .style(if channel.is_muted {
            theme::btn(theme::BtnKind::Danger)
        } else {
            theme::btn(theme::BtnKind::Secondary)
        });
    let solo = button(text("S").size(12).center())
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(26.0))
        .on_press(InterfaceMessage::Solo(ch_id, channel.is_soloed))
        .style(if channel.is_soloed {
            theme::btn(theme::BtnKind::Success)
        } else {
            theme::btn(theme::BtnKind::Secondary)
        });
    col = col.push(row![mute, Space::new().width(8), solo].align_y(Center));
    col = col.push(Space::new().height(8));
    col = col.push(text(layer_label(bus)).size(10).color(SECONDARY));
    col = col.push(Space::new().height(8));

    let (fader_value, fader_tip) = match bus {
        BLE_MON1 => (channel.sends.mon1, format_send(channel.sends.mon1)),
        BLE_MON2 => (channel.sends.mon2, format_send(channel.sends.mon2)),
        BLE_FX1 => (channel.sends.fx1, format_send(channel.sends.fx1)),
        BLE_FX2 => (channel.sends.fx2, format_send(channel.sends.fx2)),
        _ => (
            channel.channel_strip.level,
            format_level(channel.channel_strip.level),
        ),
    };

    let fader: Element<'_, InterfaceMessage> = match bus {
        BLE_MON1 => v_slider(1..=127, fader_value, FADER_HEIGHT, move |v| {
            InterfaceMessage::SendMon1(ch_id, v)
        }, fader_tip),
        BLE_MON2 => v_slider(1..=127, fader_value, FADER_HEIGHT, move |v| {
            InterfaceMessage::SendMon2(ch_id, v)
        }, fader_tip),
        BLE_FX1 => v_slider(1..=127, fader_value, FADER_HEIGHT, move |v| {
            InterfaceMessage::SendFx1(ch_id, v)
        }, fader_tip),
        BLE_FX2 => v_slider(1..=127, fader_value, FADER_HEIGHT, move |v| {
            InterfaceMessage::SendFx2(ch_id, v)
        }, fader_tip),
        _ => v_slider(1..=127, fader_value, FADER_HEIGHT, move |v| {
            InterfaceMessage::Level(ch_id, v)
        }, fader_tip.clone()),
    };

    col = col.push(fader);
    col = col.push(Space::new().height(12));
    col = col.push(text(format_level(fader_value)).size(12).color(PRIMARY));
    col = col.push(Space::new().height(10));
    col = col.push(
        button(text("Edit").size(12).center())
            .width(Length::Fixed(80.0))
            .height(Length::Fixed(30.0))
            .on_press(InterfaceMessage::EditChannel(ch_id))
            .style(theme::btn(theme::BtnKind::Secondary)),
    );

    container(col)
        .style(theme::surface_box)
        .width(Length::Fixed(STRIP_WIDTH))
        .into()
}

fn build_bus_strip<'a>(
    _controller: &'a FLOW8Controller,
    bus: &'a Bus,
) -> Element<'a, InterfaceMessage> {
    let bus_idx = bus.index;
    let bus_id = bus.id;

    let mut col = Column::new()
        .width(Length::Fixed(BUS_STRIP_WIDTH))
        .align_x(Center)
        .spacing(0)
        .padding([8, 8]);

    col = col.push(text(bus.label()).size(14).color(PRIMARY));
    col = col.push(Space::new().height(8));
    col = col.push(text("—").size(10).color(TERTIARY));
    col = col.push(Space::new().height(16));

    let mute = button(text("M").size(12).center())
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(26.0))
        .style(theme::btn(theme::BtnKind::Secondary));
    col = col.push(mute);
    col = col.push(Space::new().height(16));
    col = col.push(text("Level").size(11).color(SECONDARY));
    col = col.push(Space::new().height(8));

    col = col.push(v_slider(
        1..=127,
        bus.bus_strip.level,
        FADER_HEIGHT,
        move |v| InterfaceMessage::BusLevel(bus_idx, bus_id, v),
        format_level(bus.bus_strip.level),
    ));
    col = col.push(Space::new().height(12));
    col = col.push(
        text(format_level(bus.bus_strip.level))
            .size(12)
            .color(PRIMARY),
    );
    col = col.push(Space::new().height(10));
    col = col.push(
        button(text("Edit").size(12).center())
            .width(Length::Fixed(80.0))
            .height(Length::Fixed(30.0))
            .on_press(InterfaceMessage::EditBus(bus_idx))
            .style(theme::btn(theme::BtnKind::Secondary)),
    );

    container(col)
        .style(theme::bus_strip_box)
        .width(Length::Fixed(BUS_STRIP_WIDTH))
        .into()
}
