use crate::model::{
    channels::{Bus, BusType, FxSlot},
    flow8::FLOW8Controller,
    message::InterfaceMessage,
};
use crate::view::widgets::{sync_label, v_slider, h_slider, format_level, format_percent, UNSYNCED_COLOR};
use iced::{
    widget::{button, column, container, row, text, tooltip, Column, Row, Space},
    Center, Element, Fill,
};

const FX_BUS_FADER_HEIGHT: f32 = 220.0;

const TOGGLE_ON_COLOR: iced::Color = iced::Color {
    r: 0.6,
    g: 0.3,
    b: 0.9,
    a: 1.0,
};

const MUTE_COLOR: iced::Color = iced::Color {
    r: 0.9,
    g: 0.2,
    b: 0.2,
    a: 1.0,
};

pub fn view_fx(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let fx_buses: Vec<&Bus> = controller.buses.iter().filter(|b| b.bus_type == BusType::Fx).collect();

    let fx1_col = build_fx_slot_column(&controller.fx_slots[0]);
    let fx2_col = build_fx_slot_column(&controller.fx_slots[1]);

    let fx1_bus_col = build_fx_bus_fader(fx_buses.first().copied());
    let fx2_bus_col = build_fx_bus_fader(fx_buses.get(1).copied());

    let tap_tempo = build_tap_tempo_btn();
    let mute_btn = build_mute_btn(controller.fx_muted);

    let slots_row = row![
        container(fx1_col).width(Fill),
        Space::new().width(4),
        fx1_bus_col,
        Space::new().width(12),
        container(fx2_col).width(Fill),
        Space::new().width(4),
        fx2_bus_col,
    ]
    .width(Fill);

    let bottom_bar = row![mute_btn, tap_tempo]
        .align_y(Center)
        .spacing(16);

    column![
        slots_row,
        Space::new().height(20),
        container(bottom_bar).center_x(Fill),
    ]
    .width(Fill)
    .height(Fill)
    .padding([8, 10])
    .into()
}

fn build_fx_slot_column(fx: &FxSlot) -> Element<'_, InterfaceMessage> {
    let fx_id = fx.id;
    let info = fx.preset_info();
    let is_on = fx.param2_is_on();

    let preset_grid = build_preset_grid(fx);

    let param1_row = row![
        sync_label(info.param1_label, 12.0, fx.param1_synced).width(80),
        h_slider(0..=100, fx.param1, move |v| {
            InterfaceMessage::FxParam1(fx_id, v)
        }, format_percent(fx.param1)),
    ]
    .align_y(Center)
    .spacing(8)
    .width(Fill);

    let param2_toggle = build_param2_toggle(fx_id, info.param2_off, info.param2_on, is_on, fx.param2_synced);

    let content = column![
        text(format!("FX {}", fx_id + 1)).size(15),
        Space::new().height(6),
        preset_grid,
        Space::new().height(10),
        param1_row,
        Space::new().height(8),
        param2_toggle,
    ]
    .padding([12, 10])
    .width(Fill)
    .spacing(2);

    container(content)
        .style(container::rounded_box)
        .width(Fill)
        .padding(2)
        .into()
}

fn build_fx_bus_fader(bus: Option<&Bus>) -> Element<'_, InterfaceMessage> {
    if let Some(bus) = bus {
        let bus_idx = bus.index;
        let bus_id = bus.id;

        column![
            text(bus.label()).size(12),
            Space::new().height(4),
            sync_label("Level", 10.0, bus.bus_strip.level_synced),
            v_slider(1..=127, bus.bus_strip.level, FX_BUS_FADER_HEIGHT, move |v| {
                InterfaceMessage::BusLevel(bus_idx, bus_id, v)
            }, format_level(bus.bus_strip.level)),
        ]
        .align_x(Center)
        .spacing(4)
        .width(60)
        .into()
    } else {
        Space::new().width(60).into()
    }
}

fn build_mute_btn(fx_muted: bool) -> Element<'static, InterfaceMessage> {
    let label = if fx_muted { "FX MUTED" } else { "FX MUTE" };
    let btn = button(text(label).size(12).center())
        .on_press(InterfaceMessage::FxMute)
        .padding([8, 20])
        .width(180);
    if fx_muted {
        btn.style(move |theme, status| {
            let mut style = button::danger(theme, status);
            style.background = Some(iced::Background::Color(MUTE_COLOR));
            style
        })
        .into()
    } else {
        btn.style(button::secondary).into()
    }
}

fn build_preset_grid(fx: &FxSlot) -> Element<'_, InterfaceMessage> {
    let fx_id = fx.id;
    let current = fx.preset as usize;
    let presets = fx.presets();

    let mut rows: Vec<Element<InterfaceMessage>> = Vec::new();

    for row_idx in 0..4 {
        let mut buttons: Vec<Element<InterfaceMessage>> = Vec::new();
        for col in 0..4 {
            let idx = row_idx * 4 + col;
            let preset = &presets[idx];
            let is_selected = idx == current;

            let label = column![text(preset.name).size(11)].align_x(Center);

            let btn = if is_selected {
                button(label)
                    .padding([8, 4])
                    .width(Fill)
                    .style(button::primary)
            } else {
                button(label)
                    .on_press(InterfaceMessage::FxPreset(fx_id, idx as u8))
                    .padding([8, 4])
                    .width(Fill)
                    .style(button::secondary)
            };

            buttons.push(btn.into());
        }
        rows.push(Row::with_children(buttons).spacing(4).width(Fill).into());
    }

    let mut grid = Column::new().spacing(5).width(Fill);
    for r in rows {
        grid = grid.push(r);
    }

    column![
        sync_label("Preset", 12.0, fx.preset_synced),
        Space::new().height(4),
        grid,
    ]
    .width(Fill)
    .into()
}

fn build_param2_toggle<'a>(
    fx_id: u8,
    off_label: &'a str,
    on_label: &'a str,
    is_on: bool,
    is_synced: bool,
) -> Element<'a, InterfaceMessage> {
    let off_btn = if !is_on {
        button(text(off_label).size(12).center())
            .padding([8, 12])
            .width(Fill)
            .style(move |theme, status| {
                let mut style = button::primary(theme, status);
                style.background = Some(iced::Background::Color(TOGGLE_ON_COLOR));
                style
            })
    } else {
        button(text(off_label).size(12).center())
            .on_press(InterfaceMessage::FxParam2(fx_id, 0))
            .padding([8, 12])
            .width(Fill)
            .style(button::secondary)
    };

    let on_btn = if is_on {
        button(text(on_label).size(12).center())
            .padding([8, 12])
            .width(Fill)
            .style(move |theme, status| {
                let mut style = button::primary(theme, status);
                style.background = Some(iced::Background::Color(TOGGLE_ON_COLOR));
                style
            })
    } else {
        button(text(on_label).size(12).center())
            .on_press(InterfaceMessage::FxParam2(fx_id, 127))
            .padding([8, 12])
            .width(Fill)
            .style(button::secondary)
    };

    let sync_dot = if is_synced {
        text("")
    } else {
        text("\u{25CF} ").size(10).color(UNSYNCED_COLOR)
    };

    row![sync_dot, off_btn, on_btn]
        .spacing(4)
        .align_y(Center)
        .width(Fill)
        .into()
}

fn build_tap_tempo_btn() -> Element<'static, InterfaceMessage> {
    let tap_btn = button(text("FX 2 TAP TEMPO").size(12).center())
        .on_press(InterfaceMessage::TapTempo)
        .padding([8, 20])
        .width(180)
        .style(button::secondary);

    tooltip(
        tap_btn,
        container(text("Tap repeatedly to set FX2 tempo.\nOnly works with delay/echo presets (1-12).").size(10))
            .padding(6)
            .style(container::rounded_box),
        tooltip::Position::Top,
    )
    .gap(4)
    .into()
}
