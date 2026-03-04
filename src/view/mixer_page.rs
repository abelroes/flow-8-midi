use crate::model::{
    channels::{Bus, BusType, Channel, PhantomPowerType},
    flow8::FLOW8Controller,
    message::InterfaceMessage,
};
use crate::view::widgets::{
    h_slider, sync_dot, sync_label, v_slider,
    format_gain, format_level, format_pan, format_comp, format_lowcut, format_limiter,
};
use iced::{
    widget::{
        button, column, container, row, text, tooltip, tooltip::Position,
        Column, Space,
    },
    Center, Element, Fill, Length,
};

const FADER_HEIGHT: f32 = 280.0;
const BUS_FADER_HEIGHT: f32 = 320.0;

pub fn view_mixer(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let channels_row: Vec<Element<InterfaceMessage>> = controller
        .channels
        .iter()
        .map(|c| build_channel_strip(c))
        .collect();

    let channels_section = container(
        iced::widget::Row::with_children(channels_row)
            .spacing(3)
            .width(Fill),
    )
    .width(Length::FillPortion(7));

    let bus_strips: Vec<Element<InterfaceMessage>> = controller
        .buses
        .iter()
        .filter(|b| b.bus_type != BusType::Fx)
        .map(|b| build_bus_strip(b))
        .collect();

    let buses_section = container(
        iced::widget::Row::with_children(bus_strips)
            .spacing(3)
            .width(Fill),
    )
    .width(Length::FillPortion(3));

    row![channels_section, Space::new().width(6), buses_section]
        .width(Fill)
        .height(Fill)
        .padding([0, 10])
        .into()
}

const NAME_SLOT_HEIGHT: f32 = 16.0;
const PHANTOM_SLOT_HEIGHT: f32 = 30.0;
const H_SLIDER_SLOT_HEIGHT: f32 = 38.0;

fn build_channel_strip(channel: &Channel) -> Element<'_, InterfaceMessage> {
    let mut col = Column::new()
        .width(Fill)
        .align_x(Center)
        .spacing(6)
        .padding([10, 6]);

    col = col.push(text(channel.display_label()).size(13));

    let name_content: Element<'_, InterfaceMessage> = if !channel.name.is_empty() {
        text(&channel.name).size(11).into()
    } else {
        Space::new().into()
    };
    col = col.push(
        container(name_content)
            .height(Length::Fixed(NAME_SLOT_HEIGHT))
            .align_y(Center),
    );

    col = add_mute_solo(col, channel);

    col = col.push(build_phantom_slot(channel));

    let strip = &channel.channel_strip;
    let ch_id = channel.id;

    col = col.push(build_h_slider_slot(
        channel.has_gain(),
        "Gain",
        strip.gain_synced,
        0..=127,
        strip.gain,
        format_gain(strip.gain),
        move |v| InterfaceMessage::Gain(ch_id, v),
    ));

    col = col.push(
        column![
            sync_label("Level", 12.0, strip.level_synced),
            v_slider(1..=127, strip.level, FADER_HEIGHT, move |v| {
                InterfaceMessage::Level(ch_id, v)
            }, format_level(strip.level)),
        ]
        .align_x(Center)
        .spacing(4),
    );

    col = col.push(build_h_slider_slot(
        true,
        "Bal",
        strip.balance_synced,
        0..=127,
        strip.balance,
        format_pan(strip.balance),
        move |v| InterfaceMessage::Balance(ch_id, v),
    ));

    col = col.push(build_h_slider_slot(
        channel.has_compressor(),
        "Comp",
        strip.compressor_synced,
        0..=100,
        strip.compressor,
        format_comp(strip.compressor),
        move |v| InterfaceMessage::Compressor(ch_id, v),
    ));

    col = col.push(build_h_slider_slot(
        channel.has_low_cut(),
        "Low Cut",
        strip.low_cut_synced,
        0..=127,
        strip.low_cut,
        format_lowcut(strip.low_cut),
        move |v| InterfaceMessage::LowCut(ch_id, v),
    ));

    container(col)
        .style(container::rounded_box)
        .width(Fill)
        .padding(2)
        .into()
}

fn build_h_slider_slot<'a, F>(
    enabled: bool,
    label: &'a str,
    is_synced: bool,
    range: std::ops::RangeInclusive<u8>,
    value: u8,
    tip: String,
    on_change: F,
) -> Element<'a, InterfaceMessage>
where
    F: Fn(u8) -> InterfaceMessage + 'a,
{
    let content: Element<'a, InterfaceMessage> = if enabled {
        column![
            sync_label(label, 12.0, is_synced),
            h_slider(range, value, on_change, tip),
        ]
        .align_x(Center)
        .spacing(2)
        .width(Fill)
        .padding([0, 6])
        .into()
    } else {
        Space::new().into()
    };
    container(content)
        .height(Length::Fixed(H_SLIDER_SLOT_HEIGHT))
        .width(Fill)
        .align_y(Center)
        .into()
}

fn build_phantom_slot(channel: &Channel) -> Element<'_, InterfaceMessage> {
    let content: Element<'_, InterfaceMessage> = if let PhantomPowerType::Set48v =
        channel.phantom_pwr.phantom_power_type
    {
        let awaiting_confirm = channel
            .phantom_last_click
            .map(|t| t.elapsed().as_millis() < 500)
            .unwrap_or(false);

        let base = button(if awaiting_confirm {
            text("\u{26A0} 48V \u{26A0}").size(11)
        } else if channel.phantom_pwr.is_on {
            text("48V ON").size(11)
        } else {
            text("48V").size(11)
        })
        .on_press(InterfaceMessage::PhantomPower(channel.id))
        .padding([3, 10]);

        let btn = if awaiting_confirm {
            base.style(button::primary)
        } else if channel.phantom_pwr.is_on {
            base.style(button::danger)
        } else {
            base.style(button::secondary)
        };

        let tip_text = if awaiting_confirm {
            "\u{26A0} CLICK AGAIN to confirm!\nPhantom Power can damage unbalanced microphones."
        } else {
            "\u{26A0} Double-click to toggle Phantom Power (+48V)\nWARNING: May damage unbalanced mics!"
        };

        let tip = tooltip(
            btn,
            container(text(tip_text).size(12))
                .padding(8)
                .style(container::rounded_box),
            Position::FollowCursor,
        )
        .gap(5);

        row![sync_dot(channel.phantom_synced), tip]
            .align_y(Center)
            .spacing(2)
            .into()
    } else {
        button(text(" ").size(11))
            .padding([3, 10])
            .style(button::text)
            .into()
    };
    container(content)
        .height(Length::Fixed(PHANTOM_SLOT_HEIGHT))
        .align_y(Center)
        .into()
}

fn add_mute_solo<'a>(
    column: Column<'a, InterfaceMessage>,
    channel: &'a Channel,
) -> Column<'a, InterfaceMessage> {
    let mute_btn = {
        let btn = button(text("M").size(12))
            .on_press(InterfaceMessage::Mute(channel.id, channel.is_muted))
            .padding([4, 10]);
        if channel.is_muted {
            btn.style(button::danger)
        } else {
            btn.style(button::secondary)
        }
    };

    let solo_btn = {
        let btn = button(text("S").size(12))
            .on_press(InterfaceMessage::Solo(channel.id, channel.is_soloed))
            .padding([4, 10]);
        if channel.is_soloed {
            btn.style(button::success)
        } else {
            btn.style(button::secondary)
        }
    };

    column.push(
        row![
            sync_dot(channel.mute_synced),
            mute_btn,
            sync_dot(channel.solo_synced),
            solo_btn,
        ]
        .align_y(Center)
        .spacing(2),
    )
}

fn build_bus_strip(bus: &Bus) -> Element<'_, InterfaceMessage> {
    let mut col = Column::new()
        .width(Fill)
        .align_x(Center)
        .spacing(6)
        .padding([10, 6]);

    let bus_idx = bus.index;
    let bus_id = bus.id;

    col = col.push(text(bus.label()).size(14));

    col = col.push(
        column![
            sync_label("Level", 12.0, bus.bus_strip.level_synced),
            v_slider(1..=127, bus.bus_strip.level, BUS_FADER_HEIGHT, move |v| {
                InterfaceMessage::BusLevel(bus_idx, bus_id, v)
            }, format_level(bus.bus_strip.level)),
        ]
        .align_x(Center)
        .spacing(4),
    );

    let has_bal = bus.has_balance();
    let bal_content: Element<'_, InterfaceMessage> = if has_bal {
        column![
            sync_label("Bal", 12.0, bus.bus_strip.balance_synced),
            h_slider(0..=127, bus.bus_strip.balance, move |v| {
                InterfaceMessage::BusBalance(bus_idx, bus_id, v)
            }, format_pan(bus.bus_strip.balance)),
        ]
        .align_x(Center)
        .spacing(2)
        .width(Fill)
        .padding([0, 6])
        .into()
    } else {
        Space::new().into()
    };
    col = col.push(
        container(bal_content)
            .height(Length::Fixed(H_SLIDER_SLOT_HEIGHT))
            .width(Fill)
            .align_y(Center),
    );

    if bus.has_limiter() {
        col = col.push(
            column![
                sync_label("Limiter", 12.0, bus.bus_strip.limiter_synced),
                h_slider(0..=127, bus.bus_strip.limiter, move |v| {
                    InterfaceMessage::BusLimiter(bus_idx, bus_id, v)
                }, format_limiter(bus.bus_strip.limiter)),
            ]
            .align_x(Center)
            .spacing(2)
            .width(Fill)
            .padding([0, 6]),
        );
    }

    container(col)
        .style(container::rounded_box)
        .width(Fill)
        .padding(2)
        .into()
}
