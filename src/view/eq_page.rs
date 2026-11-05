use crate::model::{
    channels::Bus,
    flow8::FLOW8Controller,
    message::InterfaceMessage,
    page::Page,
};
use crate::view::theme::{self, PRIMARY, SECONDARY, TERTIARY};
use crate::view::widgets::{
    detail_chrome, dual_meter, format_comp, format_eq, format_gain, format_level, format_limiter,
    format_lowcut, format_pan, h_slider, sync_dot, v_slider,
};
use iced::{
    widget::{button, column, container, row, text, Column, Row, Space},
    Center, Element, Fill, Length,
};

const EQ_GAIN_HEIGHT: f32 = 120.0;
const NINE_BAND_SLIDER_HEIGHT: f32 = 160.0;

pub fn view_eq(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    if controller.detail_is_bus {
        return view_bus_eq(controller);
    }
    view_input_eq(controller)
}

fn view_input_eq(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let ch_id = controller.selected_channel;
    let Some(channel) = controller.channels.iter().find(|c| c.id == ch_id) else {
        return text("Channel not found").into();
    };

    let idx = channel.id as usize;
    let left = controller.meter.stereo_left(idx);
    let right = controller.meter.stereo_right(idx);
    let show_parametric = controller.app_prefs[3];

    let chrome = detail_chrome("Main  /  Input", Page::Mixer);

    let channel_picker = channel_tabs(controller);

    let header = row![
        text(channel.display_label()).size(20).color(PRIMARY),
        Space::new().width(24),
        text("Name").size(12).color(SECONDARY),
        Space::new().width(8),
        container(
            text(if channel.name.is_empty() {
                "—"
            } else {
                channel.name.as_str()
            })
            .size(14)
            .color(PRIMARY)
        )
        .padding([6, 12])
        .style(theme::surface_box),
        Space::new().width(Fill),
        button(text("Sends").size(12).center())
            .padding([8, 14])
            .on_press(InterfaceMessage::NavigateTo(Page::Sends))
            .style(theme::btn(theme::BtnKind::Secondary)),
    ]
    .align_y(Center)
    .padding([0, 16]);

    let toggles = row![
        toggle_chip(
            "Mute",
            channel.is_muted,
            InterfaceMessage::Mute(ch_id, channel.is_muted),
            theme::BtnKind::Danger,
        ),
        toggle_chip(
            "Solo",
            channel.is_soloed,
            InterfaceMessage::Solo(ch_id, channel.is_soloed),
            theme::BtnKind::Success,
        ),
        button(text("Phase Ø").size(12).center())
            .padding([8, 14])
            .height(Length::Fixed(32.0))
            .style(theme::btn(theme::BtnKind::Secondary)),
        if channel.has_phantom() {
            toggle_chip(
                "Phantom 48V",
                channel.phantom_pwr.is_on,
                InterfaceMessage::PhantomPower(ch_id),
                theme::BtnKind::Primary,
            )
        } else {
            Space::new().width(0).into()
        },
    ]
    .spacing(8)
    .padding([8, 16]);

    let meters = column![
        text("Meters").size(12).color(SECONDARY),
        Space::new().height(12),
        dual_meter(
            left.level_normalized(),
            right.map(|r| r.level_normalized()),
        ),
        Space::new().height(8),
        text("L          R").size(11).color(TERTIARY),
        Space::new().height(8),
        text(format!(
            "Peak  {:.1} dB",
            left.level_normalized() * -60.0 + 0.0
        ))
        .size(11)
        .color(PRIMARY),
        text("GR  0.0 dB").size(11).color(SECONDARY),
    ]
    .width(Length::Fixed(120.0))
    .padding(12);

    let strip = &channel.channel_strip;
    let controls = column![
        control_row(
            "Gain",
            format_gain(strip.gain),
            h_slider(0..=127, strip.gain, move |v| InterfaceMessage::Gain(ch_id, v), format_gain(strip.gain)),
            channel.has_gain(),
        ),
        control_row(
            "Comp",
            format_comp(strip.compressor),
            h_slider(
                0..=100,
                strip.compressor.min(100),
                move |v| InterfaceMessage::Compressor(ch_id, v),
                format_comp(strip.compressor),
            ),
            channel.has_compressor(),
        ),
        control_row(
            "Low Cut",
            if strip.low_cut == 0 {
                "Off".into()
            } else {
                format_lowcut(strip.low_cut)
            },
            h_slider(
                0..=127,
                strip.low_cut,
                move |v| InterfaceMessage::LowCut(ch_id, v),
                format_lowcut(strip.low_cut),
            ),
            channel.has_low_cut(),
        ),
        control_row(
            "Level",
            format_level(strip.level),
            h_slider(
                1..=127,
                strip.level,
                move |v| InterfaceMessage::Level(ch_id, v),
                format_level(strip.level),
            ),
            true,
        ),
        control_row(
            "Pan",
            format_pan(strip.balance),
            h_slider(
                0..=127,
                strip.balance,
                move |v| InterfaceMessage::Balance(ch_id, v),
                format_pan(strip.balance),
            ),
            true,
        ),
    ]
    .spacing(10)
    .width(Fill)
    .padding(12);

    let main_card = container(
        row![meters, Space::new().width(16), controls]
            .width(Fill),
    )
    .style(theme::surface_box)
    .padding(8)
    .width(Fill);

    let peq_header = row![
        text("4-band PEQ").size(16).color(PRIMARY),
        Space::new().width(12),
        text("Freq and Q appear when EQ full parametric is on.")
            .size(12)
            .color(SECONDARY),
    ]
    .align_y(Center);

    let eq = &channel.four_band_eq;
    let bands = row![
        peq_band("Low", eq.low, eq.low_synced, "120 Hz", "0.7", show_parametric, move |v| {
            InterfaceMessage::EqLow(ch_id, v)
        }),
        peq_band(
            "Lo Mid",
            eq.low_mid,
            eq.low_mid_synced,
            "400 Hz",
            "1.0",
            show_parametric,
            move |v| InterfaceMessage::EqLowMid(ch_id, v),
        ),
        peq_band(
            "Hi Mid",
            eq.hi_mid,
            eq.hi_mid_synced,
            "2.5 kHz",
            "1.2",
            show_parametric,
            move |v| InterfaceMessage::EqHiMid(ch_id, v),
        ),
        peq_band("High", eq.hi, eq.hi_synced, "8 kHz", "0.7", show_parametric, move |v| {
            InterfaceMessage::EqHi(ch_id, v)
        }),
    ]
    .spacing(10)
    .width(Fill);

    column![
        chrome,
        channel_picker,
        header,
        toggles,
        Space::new().height(8),
        container(main_card).padding([0, 16]),
        container(peq_header).padding([12, 16]),
        container(bands).padding([0, 16]),
        Space::new().height(20),
    ]
    .width(Fill)
    .into()
}

fn view_bus_eq(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let bus_idx = controller.selected_bus_edit as usize;
    let Some(bus) = controller.buses.get(bus_idx) else {
        return text("Bus not found").into();
    };

    let chrome = detail_chrome("Main  /  Output  /  EQ", Page::Mixer);

    let header = row![
        text(format!("{} — Graphic EQ", bus.label()))
            .size(18)
            .color(PRIMARY),
    ]
    .padding([8, 16]);

    let body = if bus.has_nine_band_eq() {
        build_nine_band(bus)
    } else {
        text("This bus has no graphic EQ.")
            .size(13)
            .color(SECONDARY)
            .into()
    };

    column![
        chrome,
        header,
        container(body).padding([0, 16]),
        Space::new().height(20),
    ]
    .width(Fill)
    .into()
}

fn channel_tabs(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let mut r = row![].spacing(6).padding([4, 16]);
    for ch in &controller.channels {
        let active = ch.id == controller.selected_channel && !controller.detail_is_bus;
        r = r.push(
            button(text(ch.display_label()).size(11).center())
                .padding([6, 10])
                .on_press(InterfaceMessage::SelectChannel(ch.id))
                .style(if active {
                    theme::btn(theme::BtnKind::Primary)
                } else {
                    theme::btn(theme::BtnKind::Quiet)
                }),
        );
    }
    r.into()
}

fn toggle_chip(
    label: &'static str,
    active: bool,
    msg: InterfaceMessage,
    active_kind: theme::BtnKind,
) -> Element<'static, InterfaceMessage> {
    button(text(label).size(12).center())
        .padding([8, 14])
        .height(Length::Fixed(32.0))
        .on_press(msg)
        .style(if active {
            theme::btn(active_kind)
        } else {
            theme::btn(theme::BtnKind::Secondary)
        })
        .into()
}

fn control_row<'a>(
    label: &'a str,
    value: String,
    slider: Element<'a, InterfaceMessage>,
    enabled: bool,
) -> Element<'a, InterfaceMessage> {
    if !enabled {
        return Space::new().height(0).into();
    }
    column![
        row![
            text(label).size(12).color(PRIMARY),
            Space::new().width(Fill),
            text(value).size(12).color(SECONDARY),
        ],
        Space::new().height(4),
        slider,
    ]
    .into()
}

fn peq_band<'a, F>(
    label: &'a str,
    value: u8,
    synced: bool,
    freq: &'a str,
    q: &'a str,
    show_parametric: bool,
    on_change: F,
) -> Element<'a, InterfaceMessage>
where
    F: Fn(u8) -> InterfaceMessage + 'a,
{
    let mut col = Column::new()
        .spacing(6)
        .padding(12)
        .width(Fill)
        .align_x(Center);

    col = col.push(
        row![
            text(label).size(14).color(PRIMARY),
            Space::new().width(Fill),
            text(format_eq(value)).size(13).color(SECONDARY),
        ]
        .width(Fill),
    );
    col = col.push(sync_dot(synced));
    col = col.push(v_slider(
        0..=127,
        value,
        EQ_GAIN_HEIGHT,
        on_change,
        format_eq(value),
    ));

    if show_parametric {
        col = col.push(
            row![
                text("Freq").size(11).color(SECONDARY),
                Space::new().width(Fill),
                text(freq).size(13).color(PRIMARY),
            ]
            .width(Fill),
        );
        col = col.push(
            row![
                text("Q").size(11).color(SECONDARY),
                Space::new().width(Fill),
                text(q).size(13).color(PRIMARY),
            ]
            .width(Fill),
        );
    }

    container(col).style(theme::surface_box).width(Fill).into()
}

fn build_nine_band(bus: &Bus) -> Element<'_, InterfaceMessage> {
    let bus_idx = bus.index;
    let bus_id = bus.id;
    let eq = &bus.nine_band_eq;

    let bands_data: [(&str, u8, u8); 9] = [
        ("63", eq.freq_62_hz, 0),
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
                v_slider(
                    0..=127,
                    value,
                    NINE_BAND_SLIDER_HEIGHT,
                    move |v| InterfaceMessage::BusNineBandEq(bus_idx, bus_id, band_index, v),
                    format_eq(value),
                ),
                Space::new().height(4),
                text(label).size(10).color(SECONDARY),
            ]
            .align_x(Center)
            .spacing(2),
        );
    }

    let limiter = column![
        text("Limiter").size(12).color(SECONDARY),
        h_slider(
            0..=127,
            bus.bus_strip.limiter,
            move |v| InterfaceMessage::BusLimiter(bus_idx, bus_id, v),
            format_limiter(bus.bus_strip.limiter),
        ),
    ]
    .spacing(4)
    .width(Fill);

    container(
        column![bands, Space::new().height(12), limiter]
            .padding(16)
            .width(Fill),
    )
    .style(theme::surface_box)
    .width(Fill)
    .into()
}
