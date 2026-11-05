use crate::model::message::InterfaceMessage;
use crate::service::mixer_state::{BLE_FX1, BLE_FX2, BLE_MAIN, BLE_MON1, BLE_MON2};
use crate::view::theme::{
    self, ACCENT, CONTROL_TRACK, METER_GREEN, METER_HEIGHT, PRIMARY, SECONDARY, SURFACE, TERTIARY,
    TITLEBAR_HEIGHT,
};
use iced::{
    widget::{button, container, row, slider, text, tooltip, vertical_slider, Column, Row, Space},
    Center, Element, Fill, Length,
};
use std::ops::RangeInclusive;

pub const UNSYNCED_COLOR: iced::Color = theme::WARNING;
pub const BUS_MIX: u8 = 0x01;

#[allow(dead_code)]
pub fn sync_label<'a>(label: &'a str, size: f32, is_synced: bool) -> Row<'a, InterfaceMessage> {
    if is_synced {
        row![text(label).size(size).color(SECONDARY)]
            .align_y(Center)
            .spacing(3)
    } else {
        row![
            text("\u{25CF}").size(7).color(UNSYNCED_COLOR),
            text(label).size(size).color(SECONDARY),
        ]
        .align_y(Center)
        .spacing(3)
    }
}

pub fn sync_dot(is_synced: bool) -> Element<'static, InterfaceMessage> {
    if is_synced {
        text("").size(7).into()
    } else {
        text("\u{25CF}").size(7).color(UNSYNCED_COLOR).into()
    }
}

pub fn h_slider<'a, F>(
    range: RangeInclusive<u8>,
    value: u8,
    on_change: F,
    tip: String,
) -> Element<'a, InterfaceMessage>
where
    F: Fn(u8) -> InterfaceMessage + 'a,
{
    tooltip(
        slider(range, value, on_change).width(iced::Fill).style(
            |_theme, status| {
                let active = matches!(
                    status,
                    iced::widget::slider::Status::Hovered | iced::widget::slider::Status::Dragged
                );
                iced::widget::slider::Style {
                    rail: iced::widget::slider::Rail {
                        backgrounds: (
                            iced::Background::Color(ACCENT),
                            iced::Background::Color(CONTROL_TRACK),
                        ),
                        width: 4.0,
                        border: iced::Border {
                            radius: 2.0.into(),
                            ..Default::default()
                        },
                    },
                    handle: iced::widget::slider::Handle {
                        shape: iced::widget::slider::HandleShape::Circle { radius: 7.0 },
                        background: iced::Background::Color(if active {
                            PRIMARY
                        } else {
                            ACCENT
                        }),
                        border_color: ACCENT,
                        border_width: 0.0,
                    },
                }
            },
        ),
        container(text(tip).size(10).color(PRIMARY))
            .padding(4)
            .style(theme::surface_box),
        tooltip::Position::Top,
    )
    .gap(4)
    .into()
}

pub fn v_slider<'a, F>(
    range: RangeInclusive<u8>,
    value: u8,
    height: f32,
    on_change: F,
    tip: String,
) -> Element<'a, InterfaceMessage>
where
    F: Fn(u8) -> InterfaceMessage + 'a,
{
    tooltip(
        vertical_slider(range, value, on_change)
            .height(height)
            .style(|_theme, status| {
                let active = matches!(
                    status,
                    iced::widget::slider::Status::Hovered | iced::widget::slider::Status::Dragged
                );
                iced::widget::slider::Style {
                    rail: iced::widget::slider::Rail {
                        backgrounds: (
                            iced::Background::Color(ACCENT),
                            iced::Background::Color(CONTROL_TRACK),
                        ),
                        width: 4.0,
                        border: iced::Border {
                            radius: 2.0.into(),
                            ..Default::default()
                        },
                    },
                    handle: iced::widget::slider::Handle {
                        shape: iced::widget::slider::HandleShape::Circle { radius: 7.0 },
                        background: iced::Background::Color(if active {
                            PRIMARY
                        } else {
                            ACCENT
                        }),
                        border_color: ACCENT,
                        border_width: 0.0,
                    },
                }
            }),
        container(text(tip).size(10).color(PRIMARY))
            .padding(4)
            .style(theme::surface_box),
        tooltip::Position::Right,
    )
    .gap(4)
    .into()
}

/// Short dual L/R meters as in Penpot Channel Strip (6×40).
pub fn dual_meter<'a>(left: f32, right: Option<f32>) -> Element<'a, InterfaceMessage> {
    let left_bar = meter_column(left.clamp(0.0, 1.0));
    let right_bar = meter_column(right.unwrap_or(0.0).clamp(0.0, 1.0));
    row![
        left_bar,
        Space::new().width(4),
        right_bar,
        Space::new().width(6),
        text("Pk").size(9).color(TERTIARY),
    ]
    .align_y(iced::alignment::Vertical::Bottom)
    .into()
}

fn meter_column<'a>(level: f32) -> Element<'a, InterfaceMessage> {
    let fill_h = (METER_HEIGHT * level).max(if level > 0.0 { 2.0 } else { 0.0 });
    let empty_h = (METER_HEIGHT - fill_h).max(0.0);
    let fill = container(Space::new())
        .width(Length::Fixed(6.0))
        .height(Length::Fixed(fill_h))
        .style(|_t| container::Style {
            background: Some(iced::Background::Color(METER_GREEN)),
            border: iced::Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    container(
        Column::new()
            .push(Space::new().height(Length::Fixed(empty_h)))
            .push(fill)
            .width(Length::Fixed(6.0))
            .height(Length::Fixed(METER_HEIGHT)),
    )
    .width(Length::Fixed(6.0))
    .height(Length::Fixed(METER_HEIGHT))
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(SURFACE)),
        border: iced::Border {
            radius: 2.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

#[allow(dead_code)]
pub fn meter_bar<'a>(level: f32, _peak: f32, height: f32) -> Element<'a, InterfaceMessage> {
    let level = level.clamp(0.0, 1.0);
    let fill_h = (height * level).max(if level > 0.0 { 2.0 } else { 0.0 });
    let empty_h = (height - fill_h).max(0.0);
    let fill = container(Space::new())
        .width(Length::Fixed(8.0))
        .height(Length::Fixed(fill_h))
        .style(|_t| container::Style {
            background: Some(iced::Background::Color(METER_GREEN)),
            border: iced::Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    container(
        Column::new()
            .push(Space::new().height(Length::Fixed(empty_h)))
            .push(fill)
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(height)),
    )
    .width(Length::Fixed(10.0))
    .height(Length::Fixed(height))
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(CONTROL_TRACK)),
        border: iced::Border {
            radius: 2.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

pub fn horizontal_meter<'a>(level: f32) -> Element<'a, InterfaceMessage> {
    let level = level.clamp(0.0, 1.0);
    let filled = ((level * 100.0).round() as u16).max(1);
    let empty = (100u16).saturating_sub(filled).max(1);
    row![
        container(Space::new())
            .width(Length::FillPortion(filled))
            .height(Length::Fixed(10.0))
            .style(|_t| container::Style {
                background: Some(iced::Background::Color(METER_GREEN)),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        container(Space::new())
            .width(Length::FillPortion(empty))
            .height(Length::Fixed(10.0))
            .style(|_t| container::Style {
                background: Some(iced::Background::Color(CONTROL_TRACK)),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    ]
    .width(iced::Fill)
    .into()
}

pub fn bus_selector_row(selected: u8) -> Element<'static, InterfaceMessage> {
    let buses: &[(u8, &str)] = &[
        (BUS_MIX, "Mix bus"),
        (BLE_FX1, "FX1"),
        (BLE_FX2, "FX2"),
        (BLE_MON1, "Mon1"),
        (BLE_MON2, "Mon2"),
        (BLE_MAIN, "Main"),
    ];

    let mut r = row![].spacing(8).align_y(Center);
    for &(id, label) in buses {
        let active = selected == id;
        let btn = button(text(label).size(12).center())
            .padding([6, 14])
            .height(Length::Fixed(28.0))
            .on_press(InterfaceMessage::SelectBus(id))
            .style(if active {
                theme::btn(theme::BtnKind::Primary)
            } else {
                theme::btn(theme::BtnKind::Secondary)
            });
        r = r.push(btn);
    }

    let open_fx = button(text("Open FX").size(12).center())
        .padding([6, 14])
        .height(Length::Fixed(28.0))
        .on_press(InterfaceMessage::NavigateTo(crate::model::page::Page::Fx))
        .style(theme::btn(theme::BtnKind::Primary));

    r = r.push(Space::new().width(12));
    r = r.push(open_fx);
    r = r.push(Space::new().width(8));
    r = r.push(text("Shown on FX1 / FX2").size(11).color(SECONDARY));
    r.into()
}

pub fn titlebar<'a>() -> Element<'a, InterfaceMessage> {
    container(
        text("FLOW 8 MIDI Controller")
            .size(13)
            .color(PRIMARY)
            .center(),
    )
    .width(Fill)
    .height(Length::Fixed(TITLEBAR_HEIGHT))
    .center_x(Fill)
    .center_y(Length::Fixed(TITLEBAR_HEIGHT))
    .style(theme::titlebar_box)
    .into()
}

pub fn detail_chrome<'a>(
    breadcrumb: &'a str,
    back_to: crate::model::page::Page,
) -> Element<'a, InterfaceMessage> {
    row![
        button(text("Back").size(12).center())
            .padding([6, 14])
            .height(Length::Fixed(28.0))
            .on_press(InterfaceMessage::NavigateTo(back_to))
            .style(theme::btn(theme::BtnKind::Secondary)),
        Space::new().width(12),
        text(breadcrumb).size(12).color(SECONDARY),
    ]
    .align_y(Center)
    .padding([8, 16])
    .into()
}

pub fn on_off_toggle<'a>(
    on: bool,
    msg: InterfaceMessage,
) -> Element<'a, InterfaceMessage> {
    button(text(if on { "ON" } else { "OFF" }).size(11).center())
        .width(Length::Fixed(60.0))
        .height(Length::Fixed(26.0))
        .on_press(msg)
        .style(if on {
            theme::btn(theme::BtnKind::Success)
        } else {
            theme::btn(theme::BtnKind::Secondary)
        })
        .into()
}

pub fn nav_tab<'a>(
    label: &'a str,
    active: bool,
    msg: InterfaceMessage,
) -> Element<'a, InterfaceMessage> {
    button(text(label).size(12).center())
        .width(Length::Fixed(78.0))
        .height(Length::Fixed(32.0))
        .on_press(msg)
        .style(if active {
            theme::btn(theme::BtnKind::Primary)
        } else {
            theme::btn(theme::BtnKind::Secondary)
        })
        .into()
}

pub fn view_tab<'a>(
    label: &'a str,
    active: bool,
    msg: InterfaceMessage,
) -> Element<'a, InterfaceMessage> {
    button(text(label).size(12).center())
        .width(Length::Fixed(84.0))
        .height(Length::Fixed(32.0))
        .on_press(msg)
        .style(if active {
            theme::btn(theme::BtnKind::Primary)
        } else {
            theme::btn(theme::BtnKind::Quiet)
        })
        .into()
}

// ── CC → display value conversions ────────────────────────────────────

const LEVEL_DB_MIN: f32 = -70.0;
const LEVEL_DB_MAX: f32 = 10.0;
const GAIN_MIN: f32 = -20.0;
const GAIN_MAX: f32 = 60.0;
const EQ_MIN: f32 = -15.0;
const EQ_MAX: f32 = 15.0;
const PAN_MIN: f32 = -1.0;
const PAN_MAX: f32 = 1.0;
const LOWCUT_MIN_HZ: f32 = 20.0;
const LOWCUT_MAX_HZ: f32 = 600.0;
const LIMITER_MIN: f32 = -30.0;
const LIMITER_MAX: f32 = 0.0;

fn cc_to_range(cc: u8, cc_max: u8, min: f32, max: f32) -> f32 {
    min + (cc as f32 / cc_max as f32) * (max - min)
}

pub fn format_level(cc: u8) -> String {
    if cc == 0 {
        return "OFF".to_string();
    }
    let db = LEVEL_DB_MIN + ((cc as f32 - 1.0) / 126.0) * (LEVEL_DB_MAX - LEVEL_DB_MIN);
    format!("{:.1} dB", db)
}

#[allow(dead_code)]
pub fn format_gain(cc: u8) -> String {
    let db = cc_to_range(cc, 127, GAIN_MIN, GAIN_MAX);
    format!("{:.1} dB", db)
}

pub fn format_pan(cc: u8) -> String {
    let pan = cc_to_range(cc, 127, PAN_MIN, PAN_MAX);
    if pan.abs() < 0.02 {
        "C".to_string()
    } else if pan < 0.0 {
        format!("L {:.0}%", pan.abs() * 100.0)
    } else {
        format!("R {:.0}%", pan * 100.0)
    }
}

pub fn format_eq(cc: u8) -> String {
    let db = cc_to_range(cc, 127, EQ_MIN, EQ_MAX);
    if db.abs() < 0.3 {
        "0 dB".to_string()
    } else {
        format!("{:+.1} dB", db)
    }
}

#[allow(dead_code)]
pub fn format_lowcut(cc: u8) -> String {
    let hz = cc_to_range(cc, 127, LOWCUT_MIN_HZ, LOWCUT_MAX_HZ);
    format!("{:.0} Hz", hz)
}

#[allow(dead_code)]
pub fn format_comp(cc: u8) -> String {
    format!("{}%", cc.min(100))
}

pub fn format_send(cc: u8) -> String {
    format_level(cc)
}

pub fn format_percent(cc: u8) -> String {
    format!("{}%", cc)
}

pub fn format_limiter(cc: u8) -> String {
    let db = cc_to_range(cc, 127, LIMITER_MIN, LIMITER_MAX);
    if db.abs() < 0.3 {
        "0 dB".to_string()
    } else {
        format!("{:.1} dB", db)
    }
}
