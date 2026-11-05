use crate::model::{flow8::FLOW8Controller, message::InterfaceMessage};
use crate::view::theme::{self, PRIMARY, SECONDARY, SUCCESS, TERTIARY};
use crate::view::widgets::{format_level, format_pan, h_slider, horizontal_meter};
use iced::{
    widget::{button, column, container, row, text, Space},
    Center, Element, Fill, Length,
};

pub fn view_stage(controller: &FLOW8Controller) -> Element<'_, InterfaceMessage> {
    let header = row![
        column![
            text("Stage").size(18).color(PRIMARY),
            text("Live channels. Mixer returns to the full strip.")
                .size(12)
                .color(SECONDARY),
        ],
        Space::new().width(Fill),
        button(text("Edit Layout").size(12).center())
            .padding([8, 16])
            .height(Length::Fixed(32.0))
            .on_press(InterfaceMessage::NavigateTo(crate::model::page::Page::Mixer))
            .style(theme::btn(theme::BtnKind::Secondary)),
    ]
    .align_y(Center)
    .padding([0, 10]);

    let mut grid = column![].spacing(10).width(Fill);
    let channels = &controller.channels;
    let mut i = 0;
    while i < channels.len() {
        let mut r = row![].spacing(10).width(Fill);
        for j in 0..4 {
            if i + j < channels.len() {
                let ch = &channels[i + j];
                let left = controller.meter.stereo_left(ch.id as usize);
                let ch_id = ch.id;
                let mute = button(text("Mute").size(12).center())
                    .width(Length::FillPortion(1))
                    .height(Length::Fixed(28.0))
                    .on_press(InterfaceMessage::Mute(ch_id, ch.is_muted))
                    .style(if ch.is_muted {
                        theme::btn(theme::BtnKind::Danger)
                    } else {
                        theme::btn(theme::BtnKind::Secondary)
                    });
                let solo = button(text("Solo").size(12).center())
                    .width(Length::FillPortion(1))
                    .height(Length::Fixed(28.0))
                    .on_press(InterfaceMessage::Solo(ch_id, ch.is_soloed))
                    .style(if ch.is_soloed {
                        theme::btn(theme::BtnKind::Success)
                    } else {
                        theme::btn(theme::BtnKind::Secondary)
                    });
                let card = container(
                    column![
                        row![
                            text(ch.display_label()).size(14).color(PRIMARY),
                            Space::new().width(Fill),
                            text(if ch.connected_l { "●" } else { "○" })
                                .size(10)
                                .color(if ch.connected_l { SUCCESS } else { TERTIARY }),
                        ]
                        .align_y(Center),
                        Space::new().height(4),
                        text(if ch.name.is_empty() {
                            "—"
                        } else {
                            ch.name.as_str()
                        })
                        .size(12)
                        .color(SECONDARY),
                        Space::new().height(12),
                        horizontal_meter(left.level_normalized()),
                        Space::new().height(10),
                        row![
                            text("Level").size(11).color(SECONDARY),
                            Space::new().width(Fill),
                            text(format_level(ch.channel_strip.level))
                                .size(11)
                                .color(PRIMARY),
                        ],
                        Space::new().height(4),
                        h_slider(
                            1..=127,
                            ch.channel_strip.level,
                            move |v| InterfaceMessage::Level(ch_id, v),
                            format_level(ch.channel_strip.level),
                        ),
                        Space::new().height(10),
                        row![mute, Space::new().width(8), solo],
                        Space::new().height(10),
                        row![
                            text("Pan").size(11).color(SECONDARY),
                            Space::new().width(Fill),
                            text(format_pan(ch.channel_strip.balance))
                                .size(11)
                                .color(PRIMARY),
                        ],
                        Space::new().height(4),
                        h_slider(
                            0..=127,
                            ch.channel_strip.balance,
                            move |v| InterfaceMessage::Balance(ch_id, v),
                            format_pan(ch.channel_strip.balance),
                        ),
                    ]
                    .padding(14),
                )
                .style(theme::surface_box)
                .width(Length::FillPortion(1));
                r = r.push(card);
            } else {
                r = r.push(Space::new().width(Length::FillPortion(1)));
            }
        }
        grid = grid.push(r);
        i += 4;
    }

    container(
        column![header, Space::new().height(16), grid]
            .padding([12, 10])
            .width(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(theme::app_box)
    .into()
}
