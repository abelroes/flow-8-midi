use crate::model::message::InterfaceMessage;
use iced::{
    widget::{container, row, slider, text, tooltip, vertical_slider, Row},
    Center, Element,
};
use std::ops::RangeInclusive;

pub const UNSYNCED_COLOR: iced::Color = iced::Color {
    r: 1.0,
    g: 0.7,
    b: 0.15,
    a: 1.0,
};

pub fn sync_label<'a>(label: &'a str, size: f32, is_synced: bool) -> Row<'a, InterfaceMessage> {
    if is_synced {
        row![text(label).size(size)].align_y(Center).spacing(3)
    } else {
        row![
            text("\u{25CF}").size(7).color(UNSYNCED_COLOR),
            text(label).size(size),
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
        slider(range, value, on_change).width(iced::Fill),
        container(text(tip).size(10))
            .padding(4)
            .style(container::rounded_box),
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
        vertical_slider(range, value, on_change).height(height),
        container(text(tip).size(10))
            .padding(4)
            .style(container::rounded_box),
        tooltip::Position::Right,
    )
    .gap(4)
    .into()
}

// ── CC → display value conversions ────────────────────────────────────
// Ranges from FLOW 8 MIDI Implementation Chart

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

pub fn format_lowcut(cc: u8) -> String {
    let hz = cc_to_range(cc, 127, LOWCUT_MIN_HZ, LOWCUT_MAX_HZ);
    format!("{:.0} Hz", hz)
}

const LIMITER_MIN: f32 = -30.0;
const LIMITER_MAX: f32 = 0.0;

pub fn format_comp(cc: u8) -> String {
    let pct = cc.min(100);
    format!("{}%", pct)
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
