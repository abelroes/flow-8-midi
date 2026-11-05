use crate::model::{
    channels::{Bus, BusType, FxSlot},
    flow8::FLOW8Controller,
    message::InterfaceMessage,
    page::Page,
};
use crate::view::theme::{self, PRIMARY, PURPLE, SECONDARY};
use crate::view::widgets::{
    detail_chrome, format_level, format_percent, h_slider, v_slider,
};
use iced::{
    widget::{button, column, container, row, text, Column, Row, Space},
    Center, Element, Fill, Length,
};

const FX_RETURN_HEIGHT: f32 = 360.0;

pub fn view_fx(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let slot = controller.selected_fx.min(1);
    let fx = &controller.fx_slots[slot as usize];
    let fx_buses: Vec<&Bus> = controller
        .buses
        .iter()
        .filter(|b| b.bus_type == BusType::Fx)
        .collect();
    let return_bus = fx_buses.get(slot as usize).copied();

    let chrome = detail_chrome("Main   /   FX1 or FX2   /   Open FX", Page::Mixer);

    let header = row![
        text("FX").size(20).color(PRIMARY),
        Space::new().width(16),
        button(text("FX1").size(12).center())
            .padding([8, 16])
            .height(Length::Fixed(32.0))
            .on_press(InterfaceMessage::SelectFxSlot(0))
            .style(if slot == 0 {
                theme::btn(theme::BtnKind::Primary)
            } else {
                theme::btn(theme::BtnKind::Secondary)
            }),
        button(text("FX2").size(12).center())
            .padding([8, 16])
            .height(Length::Fixed(32.0))
            .on_press(InterfaceMessage::SelectFxSlot(1))
            .style(if slot == 1 {
                theme::btn(theme::BtnKind::Primary)
            } else {
                theme::btn(theme::BtnKind::Secondary)
            }),
        Space::new().width(16),
        text("One processor at a time. Switch FX2 for its preset and tap tempo.")
            .size(12)
            .color(SECONDARY),
    ]
    .align_y(Center)
    .padding([8, 16]);

    let info = fx.preset_info();
    let preset_card = container(
        column![
            text("Preset").size(13).color(PRIMARY),
            Space::new().height(8),
            build_preset_grid(fx),
            Space::new().height(16),
            row![
                text(info.param1_label).size(13).color(PRIMARY),
                Space::new().width(Fill),
                text(format_percent(fx.param1)).size(13).color(SECONDARY),
            ],
            Space::new().height(4),
            h_slider(
                0..=100,
                fx.param1,
                {
                    let fx_id = fx.id;
                    move |v| InterfaceMessage::FxParam1(fx_id, v)
                },
                format_percent(fx.param1),
            ),
            Space::new().height(12),
            build_param2_toggle(fx),
        ]
        .padding(16)
        .width(Fill),
    )
    .style(theme::surface_box)
    .width(Fill);

    let return_col = build_return_column(return_bus, slot);

    let bottom = row![
        button(text(if controller.fx_muted {
            "FX MUTED"
        } else {
            "FX MUTE"
        })
        .size(12)
        .center())
        .padding([8, 20])
        .on_press(InterfaceMessage::FxMute)
        .style(if controller.fx_muted {
            theme::btn(theme::BtnKind::Danger)
        } else {
            theme::btn(theme::BtnKind::Secondary)
        }),
        Space::new().width(12),
        button(text("FX 2 TAP TEMPO").size(12).center())
            .padding([8, 20])
            .on_press(InterfaceMessage::TapTempo)
            .style(theme::btn(theme::BtnKind::Secondary)),
        Space::new().width(16),
        text("Tap tempo stays on screen for FX2, even while FX1 is selected.")
            .size(12)
            .color(SECONDARY),
    ]
    .align_y(Center)
    .padding([16, 16]);

    column![
        chrome,
        header,
        row![
            container(preset_card).padding([0, 16]).width(Fill),
            Space::new().width(8),
            container(return_col).padding([0, 16]),
        ]
        .width(Fill),
        bottom,
    ]
    .width(Fill)
    .into()
}

fn build_return_column(bus: Option<&Bus>, slot: u8) -> Element<'_, InterfaceMessage> {
    if let Some(bus) = bus {
        let bus_idx = bus.index;
        let bus_id = bus.id;
        container(
            column![
                text("Return").size(14).color(PRIMARY),
                text(format!("FX{} level", slot + 1))
                    .size(12)
                    .color(SECONDARY),
                Space::new().height(12),
                v_slider(
                    1..=127,
                    bus.bus_strip.level,
                    FX_RETURN_HEIGHT,
                    move |v| InterfaceMessage::BusLevel(bus_idx, bus_id, v),
                    format_level(bus.bus_strip.level),
                ),
                Space::new().height(8),
                text(format_level(bus.bus_strip.level))
                    .size(13)
                    .color(PRIMARY),
            ]
            .align_x(Center)
            .padding(16)
            .width(Length::Fixed(140.0)),
        )
        .style(theme::surface_box)
        .into()
    } else {
        Space::new().width(140).into()
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
            let btn = button(text(preset.name).size(12).center())
                .padding([10, 6])
                .width(Fill)
                .on_press(InterfaceMessage::FxPreset(fx_id, idx as u8))
                .style(if is_selected {
                    theme::btn(theme::BtnKind::Primary)
                } else {
                    theme::btn(theme::BtnKind::Secondary)
                });
            buttons.push(btn.into());
        }
        rows.push(Row::with_children(buttons).spacing(6).width(Fill).into());
    }

    let mut grid = Column::new().spacing(6).width(Fill);
    for r in rows {
        grid = grid.push(r);
    }
    grid.into()
}

fn build_param2_toggle(fx: &FxSlot) -> Element<'_, InterfaceMessage> {
    let fx_id = fx.id;
    let info = fx.preset_info();
    let is_on = fx.param2_is_on();

    let off_btn: Element<'_, InterfaceMessage> = button(text(info.param2_off).size(12).center())
        .padding([10, 12])
        .width(Fill)
        .on_press(InterfaceMessage::FxParam2(fx_id, 0))
        .style(move |_t, status| {
            if !is_on {
                purple_style(status)
            } else {
                theme::btn(theme::BtnKind::Secondary)(_t, status)
            }
        })
        .into();

    let on_btn: Element<'_, InterfaceMessage> = button(text(info.param2_on).size(12).center())
        .padding([10, 12])
        .width(Fill)
        .on_press(InterfaceMessage::FxParam2(fx_id, 127))
        .style(move |_t, status| {
            if is_on {
                purple_style(status)
            } else {
                theme::btn(theme::BtnKind::Secondary)(_t, status)
            }
        })
        .into();

    row![off_btn, Space::new().width(8), on_btn]
        .width(Fill)
        .into()
}

fn purple_style(status: button::Status) -> button::Style {
    let base = PURPLE;
    let background = match status {
        button::Status::Hovered => iced::Color {
            r: (base.r * 1.08).min(1.0),
            g: (base.g * 1.08).min(1.0),
            b: (base.b * 1.08).min(1.0),
            a: base.a,
        },
        button::Status::Pressed => iced::Color {
            r: base.r * 0.92,
            g: base.g * 0.92,
            b: base.b * 0.92,
            a: base.a,
        },
        _ => base,
    };
    button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: PRIMARY,
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}
