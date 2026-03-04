use crate::model::{
    channels::{Bus, Channel},
    flow8::FLOW8Controller,
    message::InterfaceMessage,
};
use crate::view::widgets::{
    h_slider, sync_dot, sync_label, v_slider,
    format_eq, format_limiter,
};
use iced::{
    widget::{column, container, row, text, Column, Row, Space},
    Center, Element, Fill,
};

const EQ_SLIDER_HEIGHT: f32 = 180.0;
const NINE_BAND_SLIDER_HEIGHT: f32 = 160.0;

pub fn view_eq(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let channel_strips: Vec<Element<InterfaceMessage>> = controller
        .channels
        .iter()
        .map(|c| build_channel_eq(c))
        .collect();

    let channels_section = Row::with_children(channel_strips)
        .spacing(3)
        .width(Fill);

    let bus_strips: Vec<Element<InterfaceMessage>> = controller
        .buses
        .iter()
        .filter(|b| b.has_nine_band_eq())
        .map(|b| build_bus_eq(b))
        .collect();

    let buses_section = Row::with_children(bus_strips)
        .spacing(4)
        .width(Fill);

    column![
        channels_section,
        Space::new().height(8),
        buses_section,
    ]
    .width(Fill)
    .height(Fill)
    .padding([0, 10])
    .spacing(4)
    .into()
}

fn build_channel_eq(channel: &Channel) -> Element<'_, InterfaceMessage> {
    let ch_id = channel.id;
    let eq = &channel.four_band_eq;

    let bands = row![
        eq_vertical_band("Lo", eq.low, eq.low_synced, move |v| InterfaceMessage::EqLow(ch_id, v)),
        eq_vertical_band("LM", eq.low_mid, eq.low_mid_synced, move |v| InterfaceMessage::EqLowMid(ch_id, v)),
        eq_vertical_band("HM", eq.hi_mid, eq.hi_mid_synced, move |v| InterfaceMessage::EqHiMid(ch_id, v)),
        eq_vertical_band("Hi", eq.hi, eq.hi_synced, move |v| InterfaceMessage::EqHi(ch_id, v)),
    ]
    .spacing(14)
    .align_y(iced::Alignment::End);

    let mut header: Column<'_, InterfaceMessage> = column![text(channel.display_label()).size(13)]
        .align_x(Center);
    if !channel.name.is_empty() {
        header = header.push(text(&channel.name).size(11));
    }

    let content = column![
        header,
        Space::new().height(6),
        bands,
    ]
    .align_x(Center)
    .spacing(2)
    .padding([10, 8])
    .width(Fill);

    container(content)
        .style(container::rounded_box)
        .width(Fill)
        .padding(2)
        .into()
}

fn eq_vertical_band<'a, F>(
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
        v_slider(0..=127, value, EQ_SLIDER_HEIGHT, on_change, format_eq(value)),
        Space::new().height(4),
        text(label).size(11),
    ]
    .align_x(Center)
    .spacing(2)
}

fn build_bus_eq(bus: &Bus) -> Element<'_, InterfaceMessage> {
    let bus_idx = bus.index;
    let bus_id = bus.id;
    let eq = &bus.nine_band_eq;

    let bands_data: [(&str, u8, u8); 9] = [
        ("62", eq.freq_62_hz, 0),
        ("125", eq.freq_125_hz, 1),
        ("250", eq.freq_250_hz, 2),
        ("500", eq.freq_500_hz, 3),
        ("1k", eq.freq_1_khz, 4),
        ("2k", eq.freq_2_khz, 5),
        ("4k", eq.freq_4_khz, 6),
        ("8k", eq.freq_8_khz, 7),
        ("16k", eq.freq_16_khz, 8),
    ];

    let mut bands = Row::new().spacing(10).align_y(iced::Alignment::End);
    for (label, value, band_index) in bands_data {
        let synced = eq.bands_synced[band_index as usize];
        bands = bands.push(
            column![
                sync_dot(synced),
                v_slider(0..=127, value, NINE_BAND_SLIDER_HEIGHT, move |v| {
                    InterfaceMessage::BusNineBandEq(bus_idx, bus_id, band_index, v)
                }, format_eq(value)),
                Space::new().height(4),
                text(label).size(10),
            ]
            .align_x(Center)
            .spacing(2),
        );
    }

    let limiter = column![
        sync_label("Limiter", 12.0, bus.bus_strip.limiter_synced),
        h_slider(0..=127, bus.bus_strip.limiter, move |v| {
            InterfaceMessage::BusLimiter(bus_idx, bus_id, v)
        }, format_limiter(bus.bus_strip.limiter)),
    ]
    .align_x(Center)
    .spacing(2)
    .width(Fill)
    .padding([0, 6]);

    let content = column![
        text(bus.label()).size(14),
        Space::new().height(6),
        bands,
        Space::new().height(8),
        limiter,
    ]
    .align_x(Center)
    .spacing(2)
    .padding([10, 10])
    .width(Fill);

    container(content)
        .style(container::rounded_box)
        .width(Fill)
        .padding(2)
        .into()
}
