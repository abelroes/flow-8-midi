use crate::model::message::InterfaceMessage;
use crate::view::theme::{self, PRIMARY};
use iced::{
    widget::{button, column, container, row, text, Space},
    Element, Fill, Length,
};

pub fn setup_header(active_tab: u8) -> Element<'static, InterfaceMessage> {
    let tabs: &[(u8, &str)] = &[
        (0, "Preferences"),
        (1, "Routing"),
        (2, "Snapshots"),
        (3, "Info"),
    ];

    let mut tab_row = row![].spacing(8);
    for &(tab, label) in tabs {
        let btn = button(text(label).size(12).center())
            .padding([8, 16])
            .height(Length::Fixed(32.0))
            .on_press(InterfaceMessage::SelectSetupTab(tab))
            .style(if tab == active_tab {
                theme::btn(theme::BtnKind::Primary)
            } else {
                theme::btn(theme::BtnKind::Secondary)
            });
        tab_row = tab_row.push(btn);
    }

    column![
        text("Setup").size(18).color(PRIMARY),
        Space::new().height(12),
        tab_row,
    ]
    .width(Fill)
    .padding([12, 10])
    .into()
}

pub fn card_section<'a>(
    title: &'a str,
    body: Element<'a, InterfaceMessage>,
) -> Element<'a, InterfaceMessage> {
    container(
        column![
            text(title).size(14).color(PRIMARY),
            Space::new().height(12),
            body,
        ]
        .padding(16),
    )
    .width(Fill)
    .style(theme::surface_box)
    .into()
}
